#!/usr/bin/env python3
"""Generate the icon lookup seed SQL from libs/iconator/icons/lookup_table.rs.

The Rust lookup table is the single source of truth for name -> icon-id mappings.
This emits one INSERT block per kind into the single `icons.lookups` table so the
DB-backed handlers stay in sync with the in-memory (fst) handlers.

Usage: python3 scripts/gen_icon_seed.py <out.sql>
"""
import re
import sys

SRC = "libs/iconator/icons/lookup_table.rs"

# Rust const array name -> value of the `kind` column in icons.lookups.
BLOCKS = {
    "EXT_ICONS": "ext",
    "FILENAME_ICONS": "filename",
    "FOLDER_ICONS": "folder",
}

ENTRY_RE = re.compile(r'\(\s*"(.+?)"\s*,\s*(\d+)\s*\)')


def esc(s: str) -> str:
    return s.replace("'", "''")


def main() -> None:
    text = open(SRC).read()

    # Locate the start of each const array, then slice each block up to the next one.
    starts = {}
    for const in BLOCKS:
        m = re.search(r"const %s: \[\(&str, u64\); \d+\] = \[" % const, text)
        if not m:
            raise SystemExit(f"could not find `{const}` in {SRC}")
        starts[const] = m.start()

    order = sorted(starts, key=lambda c: starts[c])

    out = [
        "-- @generated from libs/iconator/icons/lookup_table.rs -- do not edit by hand.",
        "-- Regenerate via `python3 scripts/gen_icon_seed.py <out.sql>` when the lookup table changes.",
        "",
    ]
    counts = {}
    for i, const in enumerate(order):
        seg_end = starts[order[i + 1]] if i + 1 < len(order) else len(text)
        segment = text[starts[const] : seg_end]
        segment = segment[segment.index("[", segment.index("=")) :]
        pairs = ENTRY_RE.findall(segment)
        kind = BLOCKS[const]
        counts[kind] = len(pairs)
        out.append(f"-- {const} ({len(pairs)} rows)")
        out.append("INSERT INTO icons.lookups (kind, name, icon_id) VALUES")
        rows = [f"    ('{kind}', '{esc(name)}', {icon_id})" for name, icon_id in pairs]
        out.append(",\n".join(rows))
        out.append("ON CONFLICT (kind, name) DO NOTHING;")
        out.append("")

    with open(sys.argv[1], "w") as f:
        f.write("\n".join(out))
    print(f"row counts: {counts}", file=sys.stderr)


if __name__ == "__main__":
    main()
