#!/usr/bin/env python3
"""
marktune_fea_generator.py
Usage: python marktune_fea_generator.py <NUM_ROWS> <STEP_PERCENT> <UPM> [output.fea]

STEP_PERCENT : percentage of UPM (e.g. 2  means 2%)
UPM          : units per em (e.g. 1000 or 2048)
STEP         : round(UPM * STEP_PERCENT / 100)
"""

import sys

# ── OpenType class / lookup definitions ──────────────────────────────────────

FILTER_DEFS = [
    ("fTMX",  "topMarks",    "TMX"),
    ("fTMNX", "topMarks",    "TMNX"),
    ("fTMY",  "topMarks",    "TMY"),
    ("fTMNY", "topMarks",    "TMNY"),
    ("fBMX",  "bottomMarks", "BMX"),
    ("fBMNX", "bottomMarks", "BMNX"),
    ("fBMY",  "bottomMarks", "BMY"),
    ("fBMNY", "bottomMarks", "BMNY"),
]

LOOKUPS = [
    # (lookup_name, filter_index, axis, positive)
    ("adjTMX",  0, "X", True),
    ("adjTMNX", 1, "X", False),
    ("adjTMY",  2, "Y", True),
    ("adjTMNY", 3, "Y", False),
    ("adjBMX",  4, "X", True),
    ("adjBMNX", 5, "X", False),
    ("adjBMY",  6, "Y", True),
    ("adjBMNY", 7, "Y", False),
]

# ── Helpers ───────────────────────────────────────────────────────────────────

def value_record(axis: str, value: int) -> str:
    if axis == "X":
        return f"<{value} 0 0 0>"
    else:
        return f"<0 {value} 0 0>"


def generate_mark_filter_sets() -> list[str]:
    """تولید Mark Filtering Set definitions — هشت تعریف واقعی."""
    lines = ["# Mark Filtering Sets"]
    for class_name, marks_class, anchor_glyph in FILTER_DEFS:
        lines.append(f"@{class_name:<6} = [@{marks_class} {anchor_glyph}];")
    lines.append("")
    return lines


def generate(num_rows: int, step: int, upm: int, step_percent: float) -> str:
    """Build the complete .fea file content and return it as a string."""
    max_value = num_rows * step
    lines = []


    # ── Automatic Code End marker ─────────────────────────────────────────────
    lines.append("# Automatic Code End\n")


    # ── Header comment ────────────────────────────────────────────────────────
    lines.append(
        f"# Mark adjustment lookups\n"
        f"# {num_rows} rows × step {step} units  "
        f"({step_percent}% of UPM {upm})"
        f"  |  max value: {max_value}\n"
    )


    # ── Mark Filtering Sets ───────────────────────────────────────────────────
    lines.extend(generate_mark_filter_sets())

    # ── Lookups ───────────────────────────────────────────────────────────────
    for lookup_name, filter_idx, axis, positive in LOOKUPS:
        class_name, marks_class, _ = FILTER_DEFS[filter_idx]
        mark_class = "@topMarks" if "top" in marks_class else "@bottomMarks"

        lines.append(f"lookup {lookup_name} {{")
        lines.append(f"    lookupflag UseMarkFilteringSet @{class_name};")
        lines.append("")

        for row in range(num_rows, 0, -1):
            raw_value = row * step
            value = raw_value if positive else -raw_value
            vr = value_record(axis, value)
            glyph_tokens = " ".join([mark_class] * row)
            lines.append(f"    pos {glyph_tokens}' {vr};")

        lines.append(f"}} {lookup_name};")
        lines.append("")

    return "\n".join(lines)


# ── Entry point ───────────────────────────────────────────────────────────────

def main():
    if len(sys.argv) < 4:
        print(
            "Usage: python marktune_fea_generator.py "
            "<NUM_ROWS> <STEP_PERCENT> <UPM> [output.fea]"
        )
        print("  NUM_ROWS     : positive integer (e.g. 32)")
        print("  STEP_PERCENT : positive number, percentage of UPM (e.g. 2)")
        print("  UPM          : positive integer (e.g. 1000 or 2048)")
        print("  output.fea   : optional output filename (default: mkmk.fea)")
        sys.exit(1)

    try:
        num_rows = int(sys.argv[1])
        if num_rows <= 0:
            raise ValueError
    except ValueError:
        print("Error: NUM_ROWS must be a positive integer.")
        sys.exit(1)

    try:
        step_percent = float(sys.argv[2])
        if step_percent <= 0:
            raise ValueError
    except ValueError:
        print("Error: STEP_PERCENT must be a positive number.")
        sys.exit(1)

    try:
        upm = int(sys.argv[3])
        if upm <= 0:
            raise ValueError
    except ValueError:
        print("Error: UPM must be a positive integer.")
        sys.exit(1)

    output_file = sys.argv[4] if len(sys.argv) >= 5 else "mkmk.fea"

    step = round(upm * step_percent / 100)
    if step == 0:
        print(
            f"Warning: {step_percent}% of UPM {upm} rounds to 0. "
            "Use a larger percentage or UPM."
        )
        sys.exit(1)

    content = generate(num_rows, step, upm, step_percent)

    with open(output_file, "w", encoding="utf-8") as f:
        f.write(content)

    print(
        f"✓ {output_file} — {num_rows} rows × step {step} units "
        f"({step_percent}% of UPM {upm}  |  max value: {num_rows * step})"
    )


if __name__ == "__main__":
    main()
