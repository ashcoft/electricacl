"""Extract the load list + motor reference table from the source workbook.

Produces backend/data/seed.json and validates the extraction against the
numbers computed by the spreadsheet itself.
"""
import json
import math
import os
import re
import sys

import openpyxl

SRC = os.path.join(os.path.dirname(__file__), "..", "LOAD LIST.xlsx")
OUT = os.path.join(os.path.dirname(__file__), "..", "backend", "data", "seed.json")

SCENARIOS = [
    # (key, label, flag_col, cont_kw, int_kw, std_kw, spare_kw)
    ("normal", "Normal / Holding", 13, 37, 39, 41, 43),
    ("offloading", "Offloading", 14, 46, 48, 50, 52),
    ("maintenance", "Maintenance", 15, 55, 57, 59, 61),
    ("essential", "Essential", 16, 64, 66, 68, 70),
    ("emergency", "Emergency", 17, 73, 75, 77, 79),
    ("black_start", "Black Start", 18, 82, 84, 86, 88),
]

DUTY = {"C": "continuous", "I": "intermittent", "S": "standby", "SB": "standby",
        "B": "spare", "SP": "spare"}


def clean(v):
    if v is None:
        return None
    if isinstance(v, str):
        v = v.strip()
        if not v or v == "-":
            return None
    return v


def num(v):
    return float(v) if isinstance(v, (int, float)) else None


def main():
    wb = openpyxl.load_workbook(SRC, data_only=True)
    ws = wb["Overall Load List"]
    ap = wb["Appendix-11 Motor PF and Ef"]

    motors = []
    for i in range(8, 88):
        rated = num(ap.cell(i, 2).value)
        if rated is None:
            continue
        motors.append({
            "rated_kw": rated,
            "starting_ratio": num(ap.cell(i, 3).value),
            "eff_full": num(ap.cell(i, 4).value),
            "eff_75": num(ap.cell(i, 5).value),
            "eff_50": num(ap.cell(i, 6).value),
            "pf_full": num(ap.cell(i, 7).value),
            "pf_75": num(ap.cell(i, 8).value),
            "pf_50": num(ap.cell(i, 9).value),
        })

    items = []
    for i in range(9, 994):
        rated = num(ws.cell(i, 21).value)
        absorbed = num(ws.cell(i, 20).value)
        if rated is None and absorbed is None:
            continue
        duty_raw = clean(ws.cell(i, 11).value)
        items.append({
            "line_no": ws.cell(i, 1).value if isinstance(ws.cell(i, 1).value, (int, float)) else None,
            "revision": clean(ws.cell(i, 2).value),
            "status": clean(ws.cell(i, 3).value),
            "unit": clean(ws.cell(i, 4).value),
            "area": clean(ws.cell(i, 5).value),
            "system": clean(ws.cell(i, 6).value),
            "tag": clean(ws.cell(i, 7).value),
            "description": clean(ws.cell(i, 8).value),
            "equipment_class": {"M": "motor", "F": "feeder"}.get(
                clean(ws.cell(i, 9).value), "other"),
            "feeder_type": clean(ws.cell(i, 10).value),
            "duty": DUTY.get(duty_raw),
            "load_type": clean(ws.cell(i, 12).value),
            "scenarios": [s[0] for s in SCENARIOS if clean(ws.cell(i, s[2]).value) == "X"],
            "voltage_kv": num(ws.cell(i, 19).value) or 0.69,
            "phase": clean(ws.cell(i, 27).value),
            "substation": clean(ws.cell(i, 29).value),
            "switchgear": clean(ws.cell(i, 30).value),
            "bus": clean(ws.cell(i, 31).value),
            "drive_type": clean(ws.cell(i, 32).value),
            "absorbed_kw": absorbed or 0.0,
            "rated_kw": rated or absorbed or 0.0,
            "remark": clean(ws.cell(i, 45).value),
        })

    # ---- replicate the workbook calculation and compare -------------------
    def lookup(r):
        return next((m for m in motors if abs(m["rated_kw"] - r) < 1e-9), None)

    def eff_pf(item):
        rated, absorbed = item["rated_kw"], item["absorbed_kw"]
        if item["equipment_class"] == "feeder":
            return 0.95, 0.9
        if item["equipment_class"] == "other":
            return 1.0, 1.0
        lf = absorbed / rated if rated else 0.0
        m = lookup(rated)
        if m is None:
            return 0.95, 0.9
        if lf > 0.75:
            e = m["eff_75"] + (m["eff_full"] - m["eff_75"]) * (lf - 0.75) / 0.25
            p = m["pf_75"] + (m["pf_full"] - m["pf_75"]) * (lf - 0.75) / 0.25
        elif absorbed > 0.5:
            e = m["eff_50"] + (m["eff_75"] - m["eff_50"]) * (lf - 0.5) / 0.25
            p = m["pf_50"] + (m["pf_75"] - m["pf_50"]) * (lf - 0.5) / 0.25
        else:
            e = m["eff_50"] + (m["eff_75"] - m["eff_50"]) * (0.5 - lf) / 0.25
            p = m["pf_50"] - (m["pf_75"] - m["pf_50"]) * (0.5 - lf) / 0.25
        return e, p

    computed = []
    for it in items:
        eff, pf = eff_pf(it)
        kw = it["absorbed_kw"] / eff if eff else 0.0
        kvar = kw * math.tan(math.acos(pf)) if pf else 0.0
        computed.append((it, kw, kvar, eff, pf))

    print(f"{'scenario':14} {'continuous':>12} {'interm':>11} {'standby':>11} {'spare':>11} "
          f"{'maxrun':>11} {'peak':>11} {'demand':>11}")
    results = {}
    for key, label, *_ in SCENARIOS:
        tot = {"continuous": 0.0, "intermittent": 0.0, "standby": 0.0, "spare": 0.0}
        mx = {k: 0.0 for k in tot}
        for it, kw, _kvar, _e, _p in computed:
            if key not in it["scenarios"]:
                continue
            d = it["duty"]
            if d is None:
                continue
            tot[d] += kw
            mx[d] = max(mx[d], kw)
        maxrun = tot["continuous"] + max(0.3 * tot["intermittent"], mx["intermittent"])
        peak = tot["continuous"] + 0.3 * tot["intermittent"] + max(0.1 * tot["standby"], mx["standby"])
        results[key] = (tot, mx, maxrun, peak, peak * 1.2)
        print(f"{label:14} {tot['continuous']:12.2f} {tot['intermittent']:11.2f} "
              f"{tot['standby']:11.2f} {tot['spare']:11.2f} {maxrun:11.2f} {peak:11.2f} {peak*1.2:11.2f}")

    expected = {
        "normal": (23884.462013822522, 24510.293138586345, 25950.62235669334, 31140.746828032006),
        "maintenance": (5712.609852187783, 6317.613400074892, 6528.139715864365, 7833.7676590372375),
        "essential": (8859.485766208509, 9467.962998306144, None, 11614.18717691474),
    }
    ok = True
    for key, (cont, maxrun, peak, demand) in expected.items():
        tot, mx, mr, pk, dm = results[key]
        checks = [(tot["continuous"], cont, "continuous"), (mr, maxrun, "maxrun"),
                  (dm, demand, "demand")]
        if peak is not None:
            checks.append((pk, peak, "peak"))
        for got, want, what in checks:
            if abs(got - want) > 1e-6:
                ok = False
                print(f"MISMATCH {key}.{what}: got {got!r} want {want!r}")
    print("VALIDATION:", "OK" if ok else "FAILED")

    seed = {
        "project": {
            "name": "XXX Project",
            "document": "Electrical Load List",
            "revision": "01",
        },
        "motor_table": motors,
        "items": items,
    }
    with open(OUT, "w") as fh:
        json.dump(seed, fh, indent=1)
    print(f"wrote {OUT}: {len(items)} items, {len(motors)} motor rows")
    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main())
