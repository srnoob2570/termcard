# Repository Guidelines

## Project Overview

Termcard is a Tauri 2 desktop tool (termshot-style): type a shell command, run it in a PTY inside the app, capture the final terminal screen, redact sensitive strings (home path, username, hostname by default), and export a macOS-style terminal card as PNG (2x/3x/4x) or SVG. UI text is entirely in Spanish, as are Rust doc comments and assert messages. The authoritative doc is the approved design spec at `docs/superpowers/specs/2026-09-09-termcard-design.md` (Spanish); `README.md` is untouched template boilerplate.

## Architecture & Data Flow

Single-IR principle: one intermediate representation feeds two renderers, so preview and export cannot diverge. Never re-render the card as HTML in the live preview.

```
App.tsx ──api.runCapture()──▶ lib.rs run_capture (tokio spawn_blocking)
                                   └▶ capture.rs run_command (portable-pty, $SHELL -c)
                                        └▶ vt100::Parser ─▶ ir::from_vt100 ─▶ Capture
                              redaction applied to serialized Capture JSON (lib.rs) before IPC
App.tsx ◀── Capture (camelCase JSON) ──
App.tsx ──api.exportSvg(capture, theme)──▶ export.rs render_svg ──▶ SVG string injected as preview
App.tsx ──exportPng ▶ savePng ▶ blocking save dialog ─▶ std::fs::write
Prefs: once_store JSON blob at app_config_dir/termcard-store.json
```

Rust backend (`termcard_lib`, flat modules): `lib.rs` (9 `#[tauri::command]`s + `AppState`), `capture.rs` (PTY, 120s timeout, 5MB cap, stop flag), `ir.rs` (Capture/Line/Run/Color), `redact.rs` (regex rules + LazyLock cache), `theme.rs` (Theme, 4 presets, 256-color Palette), `export.rs` (hand-built SVG + resvg PNG, embedded JetBrains Mono).

Frontend: `main.tsx` → `App.tsx` (entire UI, ~550 lines) → `api.ts` (typed invoke wrappers, TS mirrors of Rust structs) → `preview.ts` (esc, colorCss, runStyle).

## Key Directories

- `src/` — React 19 frontend: `App.tsx`, `api.ts`, `preview.ts`, `styles.css` (Tailwind v4), `components/ui/` (shadcn on @base-ui/react)
- `src-tauri/src/` — Rust backend modules listed above; `main.rs` is 3 lines calling `termcard_lib::run()`
- `docs/superpowers/specs/` — design spec (read before architectural changes)

## Development Commands

Package manager is **Bun** (`bun.lock`). Cargo/rustup live under `~/.cargo/bin`, not on the default PATH here.

```bash
bun install
bun run app        # full desktop dev (tauri dev; prepends ~/.cargo/bin to PATH)
bun run dev        # web-only Vite dev server (no Tauri shell), port 1420 strict
bun run build      # tsc && vite build — tsc IS the typecheck gate
bun run preview    # serve production build
bun run tauri build # release bundle (.deb/AppImage)
cd src-tauri && cargo test   # entire test suite (29 tests, inline)
```

No lint or formatter config exists (no eslint/prettier/biome). No CI. No git hooks.

## Code Conventions & Common Patterns

**Rust**

- Everything crossing IPC carries `#[serde(rename_all = "camelCase")]`; keep it on new IPC structs (`ir.rs`, `theme.rs`, `redact.rs`).
- Errors: `Result<_, String>` at the command boundary (`.map_err(|e| e.to_string())`). New commands should return `Result<_, String>`. The one real error type is `CaptureError(pub String)` in `capture.rs`.
- State: `.manage(AppState { stop: Arc<AtomicBool>, running: tokio::sync::Mutex })`; `LazyLock` statics for fonts and regex cache; blocking PTY work under `tokio::task::spawn_blocking`.
- Redaction mutates the serialized `serde_json::Value` directly (`apply_redaction_to_json`) rather than round-tripping through structs.
- Comments, doc comments, UI strings, and assert messages: Spanish. Keep this consistent.

**TypeScript/React**

- Strict TS, `noUnusedLocals`/`noUnusedParameters`. Path alias `@` → `./src` (declared in **both** `tsconfig.json` and `vite.config.ts` — update both).
- Imports use `@/api`, `@/components/ui/button`.
- State: plain `useState` + `useCallback`; no store or context. Persistence pattern: `patchPrefs`/`patchTheme` fire-and-forget `void api.savePrefs(next)` inside the state updater; init merges raw `getPrefs` JSON over `DEFAULT_PREFS`.
- shadcn/ui components (`src/components/ui/`), Tailwind v4 CSS-first (`src/styles.css`, `@theme inline`, oklch vars), lucide-react icons, `cn()` merge. ANSI preview colors are CSS vars: `var(--ansi-${color.indexed})`.

**Cross-boundary contract**: adding an IPC command touches both `lib.rs` (`generate_handler![]`) and `api.ts` (struct mirror + invoke wrapper). Missing either side fails the `tsc` build or the runtime invoke.

## Important Files

- `src-tauri/src/lib.rs` — all IPC commands, `AppState`, `once_store` persistence
- `src-tauri/src/ir.rs` — the IR contract; changes ripple through redact, theme, export, and api.ts
- `src/api.ts` — TS mirror of every Rust IPC struct plus the `api` object
- `src/App.tsx` — entire UI, including `PRESET_OVERRIDES`
- `vite.config.ts`, `tsconfig.json`, `components.json` (shadcn: base-nova style, Tailwind v4 at `src/styles.css`)

## Runtime/Tooling Preferences

- Bun for all JS tooling; `tauri.conf.json` `beforeDevCommand`/`beforeBuildCommand` use `bun run`.
- ESM throughout (`"type": "module"`, `moduleResolution: "bundler"`); no `.env` files, no `import.meta.env` usage.
- Rust edition 2021; release profile: `lto`, `codegen-units=1`, `panic=abort`, `strip`. Lib crate is named `termcard_lib` (bin-name collision documented in `Cargo.toml`).
- Frontend dev server is pinned to port 1420 (`strictPort`); Tauri expects `devUrl http://localhost:1420`.

## Testing & QA

- Rust-only suite, 29 tests, all inline `#[cfg(test)] mod tests` at file bottom with `use super::*`. No `tests/` dir, no frontend tests, no coverage tooling.
- Naming: snake_case behavioral names (`svg_escapes_xml`, `disabled_rules_skipped`). Fixtures are in-module helper functions, not files (e.g. `sample()` in `export.rs:330`, `rules()` in `redact.rs:106`).
- `capture.rs` tests are **not hermetic**: they spawn real shell commands and depend on PATH, GNU `ls --color=always`, and `/tmp`. `stop_flag_kills_hanging_command` (capture.rs:166) is the slowest/flakiest (150ms timing race, spawns `sleep 60`).
- Known invariants to keep in sync:
    - Theme defaults are triplicated: `Theme::default` (theme.rs), `DEFAULT_PREFS` (api.ts), `defaultTheme()` (preview.ts). Preset overrides duplicated in `theme.rs::Preset::theme` and `App.tsx::PRESET_OVERRIDES`. Change all copies.
    - The synthetic `❯ <command>` prompt line is rendered by both the SVG generator and any future renderer; the displayed command goes through redaction like captured text.
    - `tauri-plugin-store` is still a Cargo dependency, but persistence is the hand-rolled `once_store` module — don't add plugin-store calls.
- Design spec gotcha: the spec says "vanilla TS, no framework" but the code is React 19 + shadcn. Code is current; the spec is stale on this point.
