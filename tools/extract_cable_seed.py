"""Extract the cable sizing workbook into backend/data/cable_seed.json.

Reproduces the workbook calculation for every circuit and fails if the numbers
do not match, so the seed and the engine stay in step with the source document.
"""

import json
import math
import os
import sys

import openpyxl

SRC = os.path.join(os.path.dirname(__file__), "..", "0000-EF-D-41335_0 Power Cable Sizing Calc.XLSX")
OUT = os.path.join(os.path.dirname(__file__), "..", "backend", "data", "cable_seed.json")

SQRT3 = 1.732  # the literal the workbook uses

# The workbook's maximum-length cell for SI 1 (AK15) points at AJ16 - the 3 %
# limit of the downstream lighting chain - and uses the single-phase factor 2
# for a three-phase circuit. The engine uses the circuit's own limit and 1.732,
# which gives 95.7 m instead of 248.5 m; both pass the 25 m check, so the
# verdict in the workbook is unaffected.
KNOWN_DEVIATIONS = {("3000-NLP-2001-P01", "max_length_m")}

CORES = {"2C+E": "two_core_earth", "3C+E": "three_core_earth", "4C+E": "four_core_earth"}


def clean(value):
    if value is None:
        return None
    if isinstance(value, str):
        value = value.strip()
        if not value or value == "-":
            return None
    return value


def num(value):
    return float(value) if isinstance(value, (int, float)) else None


def main():
    wb = openpyxl.load_workbook(SRC, data_only=True)
    ws = wb["Summary"]

    # ---- circuits --------------------------------------------------------
    circuits = []
    groups = {}
    for i in range(15, 25):
        g = lambda column: ws.cell(i, column).value
        if clean(g(2)) is None and clean(g(6)) is None:
            continue
        cores = clean(g(5))
        size = num(g(6))
        circuits.append(
            {
                "si_no": int(num(g(1)) or 0),
                "tag": clean(g(2)),
                "voltage_grade": clean(g(3)),
                "cores": CORES[cores],
                "size_mm2": size,
                "cable_type": clean(g(7)),
                "length_m": num(g(8)),
                "from_description": clean(g(9)),
                "from_tag": clean(g(10)),
                "from_location": clean(g(11)),
                "to_description": clean(g(12)),
                "to_tag": clean(g(13)),
                "to_location": clean(g(14)),
                "building": clean(g(15)),
                "feeder_load": clean(g(16)),
                "equipment_kw": num(g(17)),
                "load_kw": num(g(18)),
                "voltage_v": num(g(19)),
                "power_factor": num(g(20)),
                "protection_rating_a": num(g(22)),
                "breaker_rating_a": num(g(23)),
                "breaker_type": clean(g(24)),
                "runs_selected": int(num(g(29)) or 1),
                "allowable_vd_pct": num(g(36)),
                "remark": clean(g(42)),
                # source values, kept for verification only
                "_expected": {
                    "iflc": num(g(21)),
                    "ampacity": num(g(25)),
                    "derating": num(g(26)),
                    "iz": num(g(27)),
                    "runs_required": num(g(28)),
                    "vd_v": num(g(32)),
                    "vd_pct": num(g(34)),
                    "vd_last_pct": num(g(35)),
                    "max_length_m": num(g(37)),
                    "checks": [clean(g(38)), clean(g(39)), clean(g(40)), clean(g(41))],
                },
            }
        )
        groups[i] = circuits[-1]

    # ---- cable reference (AS/NZS 3008 values used by the workbook) -------
    reference = []
    seen = {}
    for i in range(15, 25):
        cores = CORES[clean(ws.cell(i, 5).value)]
        size = num(ws.cell(i, 6).value)
        it = num(ws.cell(i, 25).value)
        r = num(ws.cell(i, 30).value)
        x = num(ws.cell(i, 31).value)
        key = (cores, size)
        if key in seen:
            assert (seen[key][0], seen[key][1], seen[key][2]) == (it, r, x), key
            continue
        seen[key] = (it, r, x)
        reference.append(
            {
                "cores": cores,
                "size_mm2": size,
                "ampacity_a": it,
                "resistance_ohm_km": r,
                "reactance_ohm_km": x,
                "source": "AS/NZS 3008 Tables 14/30/35 (per workbook Attach-3/4/5)",
            }
        )
    reference.sort(key=lambda row: (row["cores"], row["size_mm2"]))

    # ---- chain assignment ------------------------------------------------
    # SI 1 is the transformer-to-panel cable, SI 2-7 the lighting fixtures
    # downstream of that panel and SI 8-10 the socket chain. The workbook
    # reports the cumulative drop only on the first circuit of a chain.
    chain_by_si = {1: None, 2: "NLP-2001", 3: "NLP-2001", 4: "NLP-2001", 5: "NLP-2001",
                   6: "NLP-2001", 7: "NLP-2001", 8: "DP-2001", 9: "DP-2001", 10: "DP-2001"}
    for circuit in circuits:
        circuit["group"] = chain_by_si.get(circuit["si_no"])

    # ---- replicate the calculation --------------------------------------
    lookup = {(row["cores"], row["size_mm2"]): row for row in reference}

    def evaluate(circuit):
        ref = lookup[(circuit["cores"], circuit["size_mm2"])]
        three_phase = circuit["voltage_v"] > 300
        k = SQRT3 if three_phase else 2.0
        pf = circuit["power_factor"]
        iflc = circuit["load_kw"] * 1000 / ((math.sqrt(3) if False else (SQRT3 if three_phase else 1)) * circuit["voltage_v"] * pf)
        derating = round(circuit["grouping_factor"] * circuit["temperature_factor"], 4)
        iz = ref["ampacity_a"] * derating
        runs_required = iflc / iz
        runs = circuit["runs_selected"]
        sin_phi = math.sin(math.acos(pf))
        vd = (
            k
            * iflc
            * (circuit["length_m"] / 1000)
            / runs
            * (ref["resistance_ohm_km"] * pf + ref["reactance_ohm_km"] * sin_phi)
        )
        vd_pct = vd / circuit["voltage_v"] * 100
        max_length = None
        if circuit["allowable_vd_pct"] is not None:
            max_length = (circuit["allowable_vd_pct"] / 100 * circuit["voltage_v"]) / (
                k
                * iflc
                / runs
                * (1 / 1000)
                * (ref["resistance_ohm_km"] * pf + ref["reactance_ohm_km"] * sin_phi)
            )
        return {
            "iflc": iflc,
            "iz": iz,
            "runs_required": runs_required,
            "vd_v": vd,
            "vd_pct": vd_pct,
            "max_length_m": max_length,
        }

    for circuit in circuits:
        circuit["grouping_factor"] = 0.8 if circuit["feeder_load"] == "PANEL" else 0.66
        circuit["temperature_factor"] = 1.0
        circuit["phases"] = "three" if circuit["voltage_v"] > 300 else "single"
        circuit.pop("grouping", None)
        derived = evaluate(circuit)
        circuit["_derived"] = derived

    # cumulative drop per chain
    for circuit in circuits:
        group = circuit["group"]
        if group is None:
            circuit["_derived"]["vd_last_pct"] = circuit["_derived"]["vd_pct"]
            circuit["_derived"]["chain_length_m"] = circuit["length_m"]
            continue
        chain = [c for c in circuits if c["group"] == group]
        first = chain[0]
        if circuit is first:
            circuit["_derived"]["vd_last_pct"] = sum(c["_derived"]["vd_pct"] for c in chain)
            circuit["_derived"]["chain_length_m"] = sum(c["length_m"] for c in chain)
        else:
            circuit["_derived"]["vd_last_pct"] = None
            circuit["_derived"]["chain_length_m"] = None

    # ---- verify against the workbook ------------------------------------
    failures = []
    for circuit in circuits:
        expected = circuit["_expected"]
        derived = circuit["_derived"]
        checks = [
            ("iflc", derived["iflc"], expected["iflc"]),
            ("iz", derived["iz"], expected["iz"]),
            ("runs_required", derived["runs_required"], expected["runs_required"]),
            ("vd_v", derived["vd_v"], expected["vd_v"]),
            ("vd_pct", derived["vd_pct"], expected["vd_pct"]),
            ("max_length_m", derived["max_length_m"], expected["max_length_m"]),
        ]
        if expected["vd_last_pct"] is not None:
            checks.append(("vd_last_pct", derived["vd_last_pct"], expected["vd_last_pct"]))
        for name, got, want in checks:
            if want is None or got is None:
                continue
            if (circuit["tag"], name) in KNOWN_DEVIATIONS:
                print(f"  note: {circuit['tag']}.{name} corrected ({want} -> {got:.3f})")
                continue
            if abs(got - want) > 1e-6:
                failures.append(f"{circuit['tag']}.{name}: {got!r} != {want!r}")

        verdicts = [
            "ACCEPTABLE" if derived["iz"] >= derived["iflc"] else "NOT-ACCEPTABLE",
            "ACCEPTABLE" if derived["iz"] > circuit["protection_rating_a"] else "NOT-ACCEPTABLE",
            None,
            None,
        ]
        if expected["vd_last_pct"] is not None and circuit["allowable_vd_pct"] is not None:
            verdicts[2] = (
                "ACCEPTABLE"
                if circuit["allowable_vd_pct"] >= derived["vd_last_pct"]
                else "NOT-ACCEPTABLE"
            )
            verdicts[3] = (
                "ACCEPTABLE"
                if derived["max_length_m"] >= derived["chain_length_m"]
                else "NOT-ACCEPTABLE"
            )
        for index, want in enumerate(expected["checks"]):
            if want is not None and verdicts[index] != want:
                failures.append(f"{circuit['tag']}.check{index}: {verdicts[index]} != {want}")

    if failures:
        print("VALIDATION FAILED")
        for line in failures:
            print("  ", line)
        return 1
    print(f"VALIDATION OK - {len(circuits)} circuits reproduced")
    for circuit in circuits:
        print(
            f"  {circuit['tag']:24} IFLC {circuit['_derived']['iflc']:7.3f} A  "
            f"IZ {circuit['_derived']['iz']:7.2f} A  VD {circuit['_derived']['vd_pct']:.4f} %"
            + (
                f"  VDLAST {circuit['_derived']['vd_last_pct']:.4f} %"
                if circuit["_derived"]["vd_last_pct"] is not None
                else ""
            )
        )

    for circuit in circuits:
        circuit.pop("_expected", None)
        circuit.pop("_derived", None)

    seed = {
        "project": {"name": "PROJECT CERES", "document": "PESB POWER CABLE SIZING CALCULATION / CRITERIA", "revision": "0"},
        "reference": reference,
        "circuits": circuits,
    }
    with open(OUT, "w") as handle:
        json.dump(seed, handle, indent=1)
    print(f"wrote {OUT}: {len(circuits)} circuits, {len(reference)} cable sizes")
    return 0


if __name__ == "__main__":
    sys.exit(main())
