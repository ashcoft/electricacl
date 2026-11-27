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


def parse_sheet(sheet):
    tag = first_text(sheet, r"Vessel Tag")
    description = None
    row, column = find_label(sheet, r"Vessel Tag")
    if row is not None:
        for candidate in (row + 1,):
            for col in range(column + 1, min(sheet.ncols, VALUE_COLUMNS + 1)):
                value = sheet.cell_value(candidate, col)
                if isinstance(value, str) and value.strip() and value.strip() != "-":
                    description = value.strip()
                    break
            if description:
                break
    if description and tag and description == tag:
        description = None

    design_life = first_number(sheet, r"Anode Design Life")
    coating = first_number(sheet, r"Coating Breakdown")
    current_density = first_number(sheet, r"Cathode.*Current Density")
    capacity = first_number(sheet, r"Electrochemical Capacity")
    utilization = first_number(sheet, r"Utilization Factor")
    safety_factor = first_number(sheet, r"Safety Factor") or 1.0

    design_temp = first_number(sheet, r"Design Temp")
    operating_temp = first_number(sheet, r"Operating Temp")
    current_density_cached = first_number(sheet, r"Current Density used")
    capacity_temp_cached = first_number(sheet, r"Temp \(Cap\)")

    diameter = first_number(sheet, r"Vessel Diameter")
    length = first_number(sheet, r"Vessel (High|Length|Height)")
    width = first_number(sheet, r"Vessel Width")
    liquid_height = first_number(sheet, r"Liquid Height")

    exposed = first_number(sheet, r"Exposed Surface")
    wetted = first_number(sheet, r"Total Wetted")
    protection_current = first_number(sheet, r"Protection Current")
    mass = first_number(sheet, r"Total Mass")
    anode_weight = first_number(sheet, r"Anode unit weight")
    anode_output = first_number(sheet, r"Mean Anode Output")
    selected = first_number(sheet, r"Number of Anodes To be Used")
    anode_type = first_text(sheet, r"Anode Type")

    required = [
        design_life,
        coating,
        current_density,
        capacity,
        utilization,
        exposed,
        wetted,
        mass,
        anode_weight,
        current_density_cached,
        capacity_temp_cached,
        protection_current,
        selected,
    ]
    if any(value is None for value in required):
        return None, "missing design parameters"

    # The workbook sometimes mislabels a vertical tank as HORIZONTAL, so pick the
    # geometry that reproduces the sheet's own wetted area.
    candidates = []
    for structure_candidate in ("tank", "vessel"):
        if diameter is not None and liquid_height is not None:
            candidates.append((structure_candidate, "vertical"))
        if diameter is not None and length is not None and liquid_height is not None:
            candidates.append((structure_candidate, "horizontal"))
    if length is not None and width is not None and liquid_height is not None:
        candidates.append(("tank", "rectangular"))
    if not candidates:
        return None, "missing geometry"

    best = None
    for structure_candidate, orientation_candidate in candidates:
        _, _, total = areas(
            structure_candidate,
            orientation_candidate,
            diameter or 0,
            length or 0,
            width or 0,
            liquid_height or 0,
        )
        error = abs(total - wetted)
        if best is None or error < best[0]:
            best = (error, structure_candidate, orientation_candidate, total)
    _, structure, orientation, total = best
    if not close(total, wetted, 1e-6):
        return None, f"area model gives {total:.4f} m2, workbook has {wetted:.4f} m2"

    discrepancy_area_cached = round_half_up(exposed - wetted, 2)
    label_row, label_column = find_label(sheet, r"Discrepancy")
    label_pct = None
    if label_row is not None:
        match = re.search(r"(\d+(?:\.\d+)?)\s*%", str(sheet.cell_value(label_row, label_column)))
        if match:
            label_pct = float(match.group(1))
    candidates = [value for value in (label_pct, 5.0, 10.0, 20.0) if value is not None]
    discrepancy_pct = None
    for candidate in dict.fromkeys(candidates):
        if close(round_half_up(wetted * candidate / 100, 2), discrepancy_area_cached, 1e-9):
            discrepancy_pct = candidate
            break
    if discrepancy_pct is None:
        discrepancy_pct = round_half_up((exposed / wetted - 1) * 100, 2)
    discrepancy_area = round_half_up(wetted * discrepancy_pct / 100, 2)
    exposed_model = wetted + discrepancy_area
    if not close(exposed_model, exposed, 1e-9):
        return None, f"discrepancy allowance gives {exposed_model:.6f} m2, workbook has {exposed:.6f} m2"

    # Pick the temperature basis and coating treatment the sheet actually uses.
    options = []
    for label, temp in (("design", design_temp), ("operating", operating_temp)):
        if temp is None:
            continue
        for apply_coating in (True, False):
            cdu = (current_density + (temp - 25) * TEMP_COEFFICIENT) * (
                coating if apply_coating else 1.0
            )
            options.append((abs(cdu - current_density_cached), label, temp, apply_coating, cdu))
    if not options:
        return None, "no temperature basis"
    _, current_temp_basis, current_temp, apply_coating, cdu = min(options)
    if not close(cdu, current_density_cached, 1e-9):
        return None, f"current density model gives {cdu:.8f}, workbook has {current_density_cached:.8f}"

    capacity_options = []
    for label, temp in (("design", design_temp), ("operating", operating_temp)):
        if temp is None:
            continue
        cap = capacity - CAPACITY_SLOPE * (temp - 20)
        capacity_options.append((abs(cap - capacity_temp_cached), label, temp, cap))
    if not capacity_options:
        return None, "no capacity temperature"
    _, capacity_basis, capacity_temp, capacity_at_temp = min(capacity_options)
    if not close(capacity_at_temp, capacity_temp_cached, 1e-9):
        return None, (
            f"capacity model gives {capacity_at_temp:.4f}, workbook has {capacity_temp_cached:.4f}"
        )

    protection_model = exposed * cdu
    if not close(protection_model, protection_current, 1e-9):
        return None, f"protection current model gives {protection_model:.8f}, workbook has {protection_current:.8f}"

    mass_model = (
        protection_model
        * design_life
        * HOURS_PER_YEAR
        / capacity_at_temp
        / utilization
        * safety_factor
    )
    if not close(mass_model, mass, 1e-9):
        return None, f"anode mass model gives {mass_model:.6f}, workbook has {mass:.6f}"

    required_count = math.ceil(mass_model / anode_weight - 1e-9)
    if int(selected) < required_count:
        return None, f"workbook uses {int(selected)} anodes, model requires {required_count}"

    item = {
        "id": "",
        "tag": tag or description or sheet.name,
        "description": description or sheet.name,
        "source_sheet": sheet.name,
        "structure": structure,
        "orientation": orientation,
        "surface": "internal",
        "method": "sacp",
        "design_life_years": design_life,
        "coating_breakdown": coating,
        "apply_coating_breakdown": apply_coating,
        "current_density_a_m2": current_density,
        "temp_coefficient": TEMP_COEFFICIENT,
        "design_temp_c": current_temp,
        "operating_temp_c": capacity_temp,
        "capacity_temp_c": capacity_temp,
        "capacity_ah_kg": capacity,
        "utilization": utilization,
        "safety_factor": safety_factor,
        "diameter_m": diameter or 0.0,
        "length_m": length or 0.0,
        "width_m": width or 0.0,
        "liquid_height_m": liquid_height or 0.0,
        "discrepancy_pct": discrepancy_pct,
        "current_density_override": None,
        "anode_type": anode_type,
        "anode_weight_kg": anode_weight,
        "anode_output_a": anode_output,
        "anodes_selected": int(selected),
        "rectifier": None,
        "remark": f"Extracted from sheet '{sheet.name}' ({current_temp_basis} temp for current density, {capacity_basis} temp for capacity).",
    }
    expected = {
        "wetted_area_m2": wetted,
        "exposed_area_m2": exposed,
        "current_density_a_m2": current_density_cached,
        "capacity_at_temp_ah_kg": capacity_temp_cached,
        "protection_current_a": protection_current,
        "anode_mass_kg": mass,
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
        row, _ = find_label(sheet, r"Vessel Tag")
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
