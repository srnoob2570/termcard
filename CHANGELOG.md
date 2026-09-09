# Changelog

All notable changes to termcard are documented in this file. The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and the project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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
