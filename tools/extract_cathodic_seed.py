"""Extract the internal-tank SACP workbook into backend/data/cathodic_seed.json.

Reproduces the workbook calculation for every vessel/tank sheet that follows the
DNV RP B401 / GS EP COR method, and fails if the numbers do not match, so the
seed and the engine stay in step with the source document.

Sheets with one-off formulas (a dimensionally inconsistent shell area, a current
density that mixes the coating factor into the temperature term, or a 50 %
wetted-area allowance) are reported and skipped instead of being forced into a
model they do not follow.
"""

import json
import math
import os
import re
import sys

import xlrd

SRC = os.path.join(
    os.path.dirname(__file__),
    "..",
    "Basic Cal SACP for Internal Tank - PT Pertamina MOR VII Sulawesi.xls",
)
OUT = os.path.join(os.path.dirname(__file__), "..", "backend", "data", "cathodic_seed.json")

TEMP_COEFFICIENT = 0.001  # DNV RP B401 para 6.3.9
CAPACITY_SLOPE = 27.0  # Ah/kg per degC above 20 degC
HOURS_PER_YEAR = 8760.0
VALUE_COLUMNS = 10  # first vessel block only (E..K); helpers live further right


def round_half_up(value, digits):
    scale = 10**digits
    return math.floor(value * scale + 0.5) / scale


def cell_number(sheet, row, column):
    value = sheet.cell_value(row, column)
    return float(value) if isinstance(value, (int, float)) else None


def find_label(sheet, pattern):
    for row in range(sheet.nrows):
        for column in range(sheet.ncols):
            value = sheet.cell_value(row, column)
            if isinstance(value, str) and re.search(pattern, value, re.I):
                return row, column
    return None, None


def first_number(sheet, pattern):
    for row in range(sheet.nrows):
        for column in range(sheet.ncols):
            value = sheet.cell_value(row, column)
            if not (isinstance(value, str) and re.search(pattern, value, re.I)):
                continue
            for index in range(column + 1, min(sheet.ncols, VALUE_COLUMNS + 1)):
                number = cell_number(sheet, row, index)
                if number is not None:
                    return number
    return None


def first_text(sheet, pattern):
    for row in range(sheet.nrows):
        for column in range(sheet.ncols):
            value = sheet.cell_value(row, column)
            if not (isinstance(value, str) and re.search(pattern, value, re.I)):
                continue
            for index in range(column + 1, min(sheet.ncols, VALUE_COLUMNS + 1)):
                candidate = sheet.cell_value(row, index)
                if isinstance(candidate, str) and candidate.strip() and candidate.strip() != "-":
                    return candidate.strip()
    return None


def areas(structure, orientation, diameter, length, width, liquid_height):
    """Return (shell area, end/head area, total wetted area)."""
    if orientation == "rectangular":
        shell = 2 * (length + width) * liquid_height
        ends = 2 * length * width
        return shell, ends, shell + ends
    if structure == "vessel" and orientation == "vertical":
        shell = math.pi * diameter * liquid_height
        ends = 1.5 * math.pi * diameter * diameter / 4
        return shell, ends, shell + ends
    if orientation == "vertical":
        shell = math.pi * diameter * liquid_height
        ends = math.pi * diameter * diameter / 4
        return shell, ends, shell + ends
    # Horizontal vessel/tank: shell and heads scaled by the fill fraction.
    fill = liquid_height / diameter if diameter > 0 else 0.0
    shell = math.pi * diameter * length * fill
    ends = 3 * math.pi * diameter * diameter / 4 * fill
    return shell, ends, shell + ends


def close(a, b, tolerance=1e-9):
    if a is None or b is None:
        return False
    return abs(a - b) <= tolerance * max(1.0, abs(a), abs(b))


TAG_LABEL = r"Vessel Tag"
DISCREPANCY_LABEL = r"Discrepancy"


def parse_identification(sheet):
    tag = first_text(sheet, TAG_LABEL)
    description = None
    row, column = find_label(sheet, TAG_LABEL)
    if row is not None:
        for col in range(column + 1, min(sheet.ncols, VALUE_COLUMNS + 1)):
            value = sheet.cell_value(row + 1, col)
            if isinstance(value, str) and value.strip() and value.strip() != "-":
                description = value.strip()
                break
    if description and tag and description == tag:
        description = None
    return tag, description


def parse_design(sheet):
    return {
        "life": first_number(sheet, r"Anode Design Life"),
        "coating": first_number(sheet, r"Coating Breakdown"),
        "current_density": first_number(sheet, r"Cathode.*Current Density"),
        "capacity": first_number(sheet, r"Electrochemical Capacity"),
        "utilization": first_number(sheet, r"Utilization Factor"),
        "safety_factor": first_number(sheet, r"Safety Factor") or 1.0,
        "design_temp": first_number(sheet, r"Design Temp"),
        "operating_temp": first_number(sheet, r"Operating Temp"),
        "current_density_cached": first_number(sheet, r"Current Density used"),
        "capacity_temp_cached": first_number(sheet, r"Temp \(Cap\)"),
    }


def parse_geometry(sheet):
    return {
        "diameter": first_number(sheet, r"Vessel Diameter"),
        "length": first_number(sheet, r"Vessel (High|Length|Height)"),
        "width": first_number(sheet, r"Vessel Width"),
        "liquid_height": first_number(sheet, r"Liquid Height"),
    }


def parse_results(sheet):
    return {
        "exposed": first_number(sheet, r"Exposed Surface"),
        "wetted": first_number(sheet, r"Total Wetted"),
        "protection_current": first_number(sheet, r"Protection Current"),
        "mass": first_number(sheet, r"Total Mass"),
        "anode_weight": first_number(sheet, r"Anode unit weight"),
        "anode_output": first_number(sheet, r"Mean Anode Output"),
        "selected": first_number(sheet, r"Number of Anodes To be Used"),
        "anode_type": first_text(sheet, r"Anode Type"),
    }


def detect_geometry(diameter, length, width, liquid_height, wetted):
    """Pick the geometry that reproduces the sheet's own wetted area."""
    candidates = []
    if diameter is not None and liquid_height is not None:
        candidates.extend([("tank", "vertical"), ("vessel", "vertical")])
    if diameter is not None and length is not None and liquid_height is not None:
        candidates.extend([("tank", "horizontal"), ("vessel", "horizontal")])
    if length is not None and width is not None and liquid_height is not None:
        candidates.append(("tank", "rectangular"))
    if not candidates:
        return None, "missing geometry"

    scored = []
    for structure, orientation in candidates:
        total = areas(
            structure,
            orientation,
            diameter or 0,
            length or 0,
            width or 0,
            liquid_height or 0,
        )[2]
        scored.append((abs(total - wetted), structure, orientation, total))
    _, structure, orientation, total = min(scored, key=lambda entry: entry[0])
    if not close(total, wetted, 1e-6):
        return None, f"area model gives {total:.4f} m2, workbook has {wetted:.4f} m2"
    return (structure, orientation), None


def discrepancy_percentage(sheet, wetted, exposed):
    cached_area = round_half_up(exposed - wetted, 2)
    label_row, label_column = find_label(sheet, DISCREPANCY_LABEL)
    label_pct = None
    if label_row is not None:
        text = str(sheet.cell_value(label_row, label_column))
        if "%" in text:
            try:
                label_pct = float(text.split("%")[0].split()[-1])
            except ValueError:
                label_pct = None
    for candidate in dict.fromkeys(value for value in (label_pct, 5.0, 10.0, 20.0) if value is not None):
        if close(round_half_up(wetted * candidate / 100, 2), cached_area, 1e-9):
            return candidate
    return round_half_up((exposed / wetted - 1) * 100, 2)


def select_current_density(design, cached):
    options = []
    for label, temperature in (
        ("design", design["design_temp"]),
        ("operating", design["operating_temp"]),
    ):
        if temperature is None:
            continue
        for apply_coating in (True, False):
            cdu = (design["current_density"] + (temperature - 25) * TEMP_COEFFICIENT) * (
                design["coating"] if apply_coating else 1.0
            )
            options.append((abs(cdu - cached), label, temperature, apply_coating, cdu))
    if not options:
        return None, "no temperature basis"
    _, basis, temperature, apply_coating, cdu = min(options)
    if not close(cdu, cached, 1e-9):
        return None, f"current density model gives {cdu:.8f}, workbook has {cached:.8f}"
    return {
        "basis": basis,
        "temperature": temperature,
        "apply_coating": apply_coating,
        "cdu": cdu,
    }, None


def select_capacity(design, cached):
    options = []
    for label, temperature in (
        ("design", design["design_temp"]),
        ("operating", design["operating_temp"]),
    ):
        if temperature is None:
            continue
        capacity = design["capacity"] - CAPACITY_SLOPE * (temperature - 20)
        options.append((abs(capacity - cached), label, temperature, capacity))
    if not options:
        return None, "no capacity temperature"
    _, basis, temperature, capacity = min(options)
    if not close(capacity, cached, 1e-9):
        return None, f"capacity model gives {capacity:.4f}, workbook has {cached:.4f}"
    return {"basis": basis, "temperature": temperature, "capacity": capacity}, None


def parse_sheet(sheet):
    tag, description = parse_identification(sheet)
    design = parse_design(sheet)
    geometry = parse_geometry(sheet)
    results = parse_results(sheet)

    required = [
        design["life"],
        design["coating"],
        design["current_density"],
        design["capacity"],
        design["utilization"],
        design["current_density_cached"],
        design["capacity_temp_cached"],
        results["exposed"],
        results["wetted"],
        results["protection_current"],
        results["mass"],
        results["anode_weight"],
        results["selected"],
    ]
    if any(value is None for value in required):
        return None, "missing design parameters"

    detected, reason = detect_geometry(
        geometry["diameter"],
        geometry["length"],
        geometry["width"],
        geometry["liquid_height"],
        results["wetted"],
    )
    if detected is None:
        return None, reason
    structure, orientation = detected

    discrepancy_pct = discrepancy_percentage(sheet, results["wetted"], results["exposed"])
    discrepancy_area = round_half_up(results["wetted"] * discrepancy_pct / 100, 2)
    exposed_model = results["wetted"] + discrepancy_area
    if not close(exposed_model, results["exposed"], 1e-9):
        return None, (
            f"discrepancy allowance gives {exposed_model:.6f} m2, "
            f"workbook has {results['exposed']:.6f} m2"
        )

    current, reason = select_current_density(design, design["current_density_cached"])
    if current is None:
        return None, reason
    capacity, reason = select_capacity(design, design["capacity_temp_cached"])
    if capacity is None:
        return None, reason

    protection_model = results["exposed"] * current["cdu"]
    if not close(protection_model, results["protection_current"], 1e-9):
        return None, (
            f"protection current model gives {protection_model:.8f}, "
            f"workbook has {results['protection_current']:.8f}"
        )

    mass_model = (
        protection_model
        * design["life"]
        * HOURS_PER_YEAR
        / capacity["capacity"]
        / design["utilization"]
        * design["safety_factor"]
    )
    if not close(mass_model, results["mass"], 1e-9):
        return None, f"anode mass model gives {mass_model:.6f}, workbook has {results['mass']:.6f}"

    required_count = math.ceil(mass_model / results["anode_weight"] - 1e-9)
    if int(results["selected"]) < required_count:
        return None, (
            f"workbook uses {int(results['selected'])} anodes, "
            f"model requires {required_count}"
        )

    item = {
        "id": "",
        "tag": tag or description or sheet.name,
        "description": description or sheet.name,
        "source_sheet": sheet.name,
        "structure": structure,
        "orientation": orientation,
        "surface": "internal",
        "method": "sacp",
        "design_life_years": design["life"],
        "coating_breakdown": design["coating"],
        "apply_coating_breakdown": current["apply_coating"],
        "current_density_a_m2": design["current_density"],
        "temp_coefficient": TEMP_COEFFICIENT,
        "design_temp_c": current["temperature"],
        "operating_temp_c": capacity["temperature"],
        "capacity_temp_c": capacity["temperature"],
        "capacity_ah_kg": design["capacity"],
        "utilization": design["utilization"],
        "safety_factor": design["safety_factor"],
        "diameter_m": geometry["diameter"] or 0.0,
        "length_m": geometry["length"] or 0.0,
        "width_m": geometry["width"] or 0.0,
        "height_m": 0.0,
        "liquid_height_m": geometry["liquid_height"] or 0.0,
        "discrepancy_pct": discrepancy_pct,
        "current_density_override": None,
        "anode_type": results["anode_type"],
        "anode_weight_kg": results["anode_weight"],
        "anode_output_a": results["anode_output"],
        "anodes_selected": int(results["selected"]),
        "rectifier": None,
        "remark": (
            f"Extracted from sheet '{sheet.name}' ({current['basis']} temp for current density, "
            f"{capacity['basis']} temp for capacity)."
        ),
    }
    expected = {
        "wetted_area_m2": results["wetted"],
        "exposed_area_m2": results["exposed"],
        "current_density_a_m2": design["current_density_cached"],
        "capacity_at_temp_ah_kg": design["capacity_temp_cached"],
        "protection_current_a": results["protection_current"],
        "anode_mass_kg": results["mass"],
        "anodes_required": required_count,
    }
    return (item, expected), None


def main():
    if not os.path.exists(SRC):
        sys.exit(f"source workbook not found: {SRC}\nPlace it in the repository root (see PR #4).")
    workbook = xlrd.open_workbook(SRC, formatting_info=False)
    items = []
    expected = []
    skipped = []
    for sheet in workbook.sheets():
        row, _ = find_label(sheet, TAG_LABEL)
        if row is None:
            continue
        parsed, reason = parse_sheet(sheet)
        if parsed is None:
            skipped.append((sheet.name, reason))
            continue
        item, values = parsed
        items.append(item)
        expected.append(values)

    if not items:
        sys.exit("no sheet reproduced the model")

    payload = {
        "project": {
            "name": "Internal tank cathodic protection",
            "document": "Basic Cal SACP for Internal Tank - PT Pertamina MOR VII Sulawesi.xls",
            "revision": "DNV RP B401 (2005) / GS EP COR 201",
        },
        "items": items,
        "verification": expected,
    }
    with open(OUT, "w", encoding="utf-8") as handle:
        json.dump(payload, handle, indent=2, ensure_ascii=False)
        handle.write("\n")

    print(f"wrote {len(items)} items to {OUT}")
    for name, reason in skipped:
        print(f"skipped '{name}': {reason}")


if __name__ == "__main__":
    main()
