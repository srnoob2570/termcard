# Changelog

All notable changes to termcard are documented in this file. The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and the project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.3] - 2026-09-10

### Added

- Manual capture mode: paste terminal output and build a card from it without running a command, through the same preview and export pipeline as a captured run. The controls move to a left panel beside the preview.
- Directory picker button next to the working-directory field, so the folder can be chosen from a dialog instead of typed.

### Changed

- The pseudo-terminal is sized to the card width at capture time, so long lines wrap exactly where the exported card wraps them.
- Presets share one layout geometry, so switching a preset only changes colors. The list grows from four to eleven presets, with dark and light palettes for Tokyo Night, Dracula, Gruvbox, Nord, and Catppuccin Latte; the Minimal preset is removed. Presets now enable the window shadow by default.

### Fixed

- The card no longer clips at the bottom edge: window sizes are computed and floored outside the frame, and the bottom padding is back.
- Switching presets keeps the fields you customized instead of resetting them.

**Full changelog:** https://github.com/srnoob2570/termcard/compare/v0.1.2...v0.1.3

## [0.1.2] - 2026-09-09

### Added

- Card width setting: automatic (fits the longest line) or manual, a fixed card width in px (defaults to 160 when switching to manual). Applies to preview and PNG/SVG exports alike; a manual width below the content lets text overflow until the command is re-captured.

### Fixed

- The width input is now free-text: clearing the field no longer snaps back to 160 mid-edit, and values are committed on blur/Enter with clamping to the valid range.
- The version label in the sidebar reads the version from `package.json` at build time instead of a hardcoded string, so it can no longer drift from the released version.

**Full changelog:** https://github.com/srnoob2570/termcard/compare/v0.1.1...v0.1.2

## [0.1.0] - 2026-09-09

First release.

### Added

- Desktop app (Linux) that runs a shell command in a pseudo-terminal inside the window and exports the final screen as a macOS-style terminal card, in PNG (2×/3×/4×) or SVG.
- Live preview that renders the exporter's real SVG, so what you see is exactly what gets exported.
- Redaction at render time with editable regex rules: home path, username, and hostname are hidden by default, and editing a rule updates the preview without re-running the command. Exports are always redacted; the "show uncensored" toggle affects only the preview.
- Theme controls with presets (Dracula, GitHub Dark, Solarized Dark/Light, and more), custom colors, padding, and outer margin.
- Editable redaction rules (enable/disable per rule, regex or literal text, add/remove, per-rule and global "uncensored" toggles).
- Bilingual interface (Spanish/English) with the language detected from the system on first run and persisted across launches.
- Working directory selector, Stop button that kills a hanging command, capture caps (5 MB, 120 s), and one capture at a time.
- Colors, emoji, and tab-aligned columns captured faithfully (`TERM=xterm-256color`, `COLORTERM=truecolor`, display-width-aware run merging).
- Save dialog for PNG exports, with the file written by the Rust backend.

## [0.1.1] - 2026-09-09

### Fixed

- Removed the drop-shadow filter from the preview SVG, which rendered the card as a black box in the WebKitGTK webview (Tauri on Linux).
- Input fields no longer trigger autocapitalization, autocorrect, spellcheck, or grammar tools.

**Full changelog:** https://github.com/srnoob2570/termcard/compare/v0.1.0...v0.1.1
