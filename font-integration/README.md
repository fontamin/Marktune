
# Font Integration Guide

This guide explains how to prepare a font so it works correctly with [Marktune](../).

## Step 1: Define the mark classes

In your font project, define two glyph classes:

- `@topMarks` — the marks that should move when the "top" control markers are used.
- `@bottomMarks` — the marks that should move when the "bottom" control markers are used.

## Step 2: Add the control glyphs

Make sure the following **control glyphs** exist in the font. These are the invisible marks Marktune actually inserts into the text to trigger the shift.

| Glyph name | Default Unicode |
|---|---|
| `TMX`  | `U+06DF` |
| `TMNX` | `U+06E0` |
| `TMY`  | `U+06EB` |
| `TMNY` | `U+06EC` |
| `BMX`  | `U+08ED` |
| `BMNX` | `U+08EE` |
| `BMY`  | `U+08EF` |
| `BMNY` | `U+08F2` |

> **Important:** If you change any of these Unicode values for any reason, you must also update the corresponding values in Marktune's `config.json` so both stay in sync.

## Step 3: Generate the mkmk feature file

Use the included generator script to produce the `mkmk` (mark-to-mark) lookups that shift `@topMarks`/`@bottomMarks` whenever the control glyphs above are present.

Run it with:
```
python marktune_fea_generator.py <NUM_ROWS> <STEP_PERCENT> <UPM> [output.fea]
```

| Argument | Description |
|---|---|
| `NUM_ROWS` | Number of rows, i.e. how many times a mark can be shifted (moved) via keyboard shortcut in a single direction. |
| `STEP_PERCENT` | The shift amount per step, as a percentage of the font's UPM. |
| `UPM` | The font's units per em (e.g. `1000` or `2048`). |
| `output.fea` | *(optional)* Output filename. Defaults to `mkmk.fea`. |

### Example
```
python marktune_fea_generator.py 24 4 1000
```

This generates lookups for up to 24 steps per direction, each step being 4% of a 1000 UPM (40 units), and writes the result to `mkmk.fea`.

## Step 4: Compile

Add the generated `.fea` file to your font build (e.g. via a `feature` include statement) and compile as usual. Merge/import the resulting `mkmk` lookups into your existing GPOS features if needed.

## Requirements

Python must be installed to run the generator script.

- **macOS/Linux:** run `marktune_fea_generator.sh`
- **Windows:** run `marktune_fea_generator.bat`

Both wrapper scripts call `marktune_fea_generator.py` with a default set of arguments and can be edited directly to change them.
