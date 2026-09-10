# Repository Atlas: termcard

## Project Responsibility

Termcard is a Tauri 2 desktop tool (termshot-style): type a shell command, run it in a PTY inside the app, capture the final terminal screen, redact sensitive strings (home path, username, hostname by default), and export a macOS-style terminal card as PNG (2x/3x/4x) or SVG. UI is bilingual Spanish/English via hand-rolled i18n. Core principle: **single renderer** — the live preview IS the exported SVG; the card is never re-rendered as HTML.

## System Entry Points

- `src/main.tsx` → `src/App.tsx` — React 19 frontend, entire UI in one component.
- `src-tauri/src/main.rs` → `termcard_lib::run()` — thin shim over the lib crate.
- `package.json` / `bun.lock` — Bun-managed dependency manifest (`bun run app` for desktop dev).
- `docs/superpowers/specs/2026-09-09-termcard-design.md` — authoritative design spec.

## Directory Map (Aggregated)

| Directory            | Responsibility Summary                                                                                                                                                                   | Detailed Map                          |
| -------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------- |
| `src/`               | Renderer layer: single-component React UI, live SVG preview, typed IPC bridges, hand-rolled es/en i18n, Tailwind v4 styles.                                                              | [View Map](src/codemap.md)            |
| `src/components/`    | `ColorField` — alpha-aware color picker (react-colorful in a Popover; "transparent"/#rrggbb/rgba() ↔ #rrggbbaa).                                                                         | [View Map](src/components/codemap.md) |
| `src/lib/`           | Utility re-exports (`cn` from the npm package `cn`, not clsx+tailwind-merge).                                                                                                            | [View Map](src/lib/codemap.md)        |
| `src/components/ui/` | Vendored shadcn `base-nova` primitives on `@base-ui/react` — boilerplate, not project logic.                                                                                             | —                                     |
| `src-tauri/`         | Desktop shell: `tauri.conf.json`, capabilities (core + opener + dialog), `Cargo.toml`, embedded JetBrains Mono fonts.                                                                    | [View Map](src-tauri/codemap.md)      |
| `src-tauri/src/`     | Rust backend (`termcard_lib`): PTY capture (portable-pty + vt100), terminal IR, regex redaction, themes/presets, SVG/PNG export (resvg), 12 IPC commands, `once_store` JSON persistence. | [View Map](src-tauri/src/codemap.md)  |

## Core Data Flow

```
App.tsx ──api.runCapture(command, cwd)──▶ capture.rs (PTY, spawn_blocking) ─▶ vt100 ─▶ ir::from_vt100 ─▶ Capture (UNREDACTED)
App.tsx ──api.exportSvg(capture, theme, rules, uncensored)──▶ redact_capture_value ─▶ export::render_svg ─▶ SVG string
       └─ injected via dangerouslySetInnerHTML = the preview
App.tsx ──api.exportPng(...scale 2..=4)──▶ render_png (resvg + fontdb) ─▶ base64 ──save_png──▶ dialog + fs write
Prefs: once_store JSON at app_config_dir/termcard-store.json (frontend-only fields ride along)
```

Redaction is re-applied at render time, so editing rules updates the preview without re-capturing; `show_raw` (uncensored) affects only the preview, never a real export.

## Cross-boundary Contract

Adding an IPC command touches `src-tauri/src/lib.rs` (command + `generate_handler![]`) **and** `src/api.ts` (struct mirror + invoke wrapper); missing either side fails `tsc` or the runtime invoke. Theme defaults are duplicated in `Theme::default` (theme.rs) and `DEFAULT_PREFS` (api.ts) — change both.
