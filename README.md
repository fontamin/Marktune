
# Marktune

Marktune is an open-source utility for repositioning Arabic diacritics using keyboard shortcuts defined in the app.

<img width="2208" height="1040" alt="Marktune-0 2 0" src="https://github.com/user-attachments/assets/72a8c4f8-d611-4dbf-ba6e-03631e7df16c" />


## Concept

The idea relies on an `mkmk` OpenType feature defined inside the font, pairing specific marker characters with Arabic diacritics through mark-to-mark kerning (the mechanism and auto-generation logic live in the `mkmk fea generator` folder). Marktune inserts these marker characters at the cursor position; the font's `mkmk` feature then visually shifts the diacritic according to the kerning defined for that marker.

While inserting, Marktune also interleaves `U+034F` (CGJ — Combining Grapheme Joiner) between characters, one after another, to prevent the OS-level AMTRA algorithm from reordering/reshaping the inserted marks.

## Current approach

Marktune currently tries to avoid interfering with text shaping/handling as much as possible, relying only on inserting plain characters at the cursor. Several alternative approaches exist, but none work consistently across all apps or produce reliable visual results. This approach may change in the future.

## How it works

- Global hotkeys (configurable) trigger insertion of specific marker characters for a "top" or "bottom" group, along axis X or Y (positive/negative).
- Text is injected directly at the cursor using OS-level text insertion (via [enigo](https://github.com/enigo-rs/enigo)), not by manipulating the app's internal buffer.
- Marktune scans backward from the cursor to find the base letter and any existing marker/CGJ sequence, recomputes the offset, and rewrites the cluster.

## Configuration

All shortcuts and marker characters are defined in `config.json`:

- `groups.top` / `groups.bottom`: marker characters (as Unicode code points, e.g. `"U+06DF"`) for the `x`, `x_neg`, `y`, `y_neg` directions of each group.
- `cgj`: the CGJ character used as a separator (defaults to `U+034F`).
- `bindings`: list of hotkeys, each mapped to a group + axis.

Example hotkeys (defaults):

| Shortcut | Action |
|---|---|
| `Ctrl+Alt+Right/Left/Up/Down` | Move top group marker |
| `Ctrl+Alt+Shift+Right/Left/Up/Down` | Move bottom group marker |

The config file location is resolved at startup (CLI argument → app bundle resources → next to executable → project dir in dev builds → current working directory), and can be opened directly from the tray menu ("Edit Shortcut Config…"). once you modify it, you had to restart the app to apply the changes.

## Platforms

- **macOS**: global hotkeys via `global-hotkey` (RegisterHotKey-equivalent), text insertion via `CGEventKeyboardSetUnicodeString`.
- **Windows**: a low-level keyboard hook (`WH_KEYBOARD_LL`) is used instead of `RegisterHotKey`, since some apps (e.g. Illustrator) read Raw Input, which `RegisterHotKey`-based hotkeys don't intercept.

The app runs as a system tray/menu bar utility (no visible window), with options to toggle shortcuts on/off, edit the config, view About info, and quit.

## Building

Requires Rust (edition 2021). Build with:
windows and mac(arm64):
```
cargo build --release
```
mac(Universal):
```
cd /Users/amin/Marktune
chmod +x bundle.sh
./bundle.sh
```

## Roadmap

Support for repositioning dots in decomposed letter structures (in addition to Arabic diacritics) has been tested and confirmed to work, and may be added in a future release.

## License

MIT License - Copyright (c) 2026 fontamin
<br>This project was built with the help of AI tools (Claude and ChatGPT), used for coding assistance throughout development.
