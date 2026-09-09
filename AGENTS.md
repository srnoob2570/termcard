# Repository Guidelines

## Project Overview

Termcard is a Tauri 2 desktop tool (termshot-style): type a shell command, run it in a PTY inside the app, capture the final terminal screen, redact sensitive strings (home path, username, hostname by default), and export a macOS-style terminal card as PNG (2x/3x/4x) or SVG. UI text is bilingual Spanish/English via hand-rolled i18n (`src/i18n.ts`; no i18n library), language detected from the system on first run and persisted in `Prefs.lang`. Source prose (comments, doc comments, assert messages, backend error strings) is English. The authoritative doc is the approved design spec at `docs/superpowers/specs/2026-09-09-termcard-design.md` (translated to English); `README.md` describes the project in English.

## Architecture & Data Flow

Single-renderer principle: the live preview IS the exported SVG. `src/preview.ts` was deleted; there is no frontend renderer. Never re-render the card as HTML in the preview.

```
App.tsx ──api.runCapture(command, cwd)──▶ lib.rs run_capture (tokio spawn_blocking)
                                   └▶ capture.rs run_command (portable-pty, $SHELL -c)
                                        └▶ vt100::Parser ─▶ ir::from_vt100 ─▶ Capture (UNREDACTED JSON)
App.tsx ──api.exportSvg(capture, theme, rules, uncensored)──▶ export_svg command
                              └▶ redact_capture_value (unless show_raw) ─▶ export::render_svg ─▶ SVG string
App.tsx injects it via dangerouslySetInnerHTML  ◀── the preview
App.tsx ──api.exportPng(capture, theme, rules, scale 2..=4)──▶ render_png (resvg) ─▶ base64
        ──api.savePng(base64, name)──▶ Rust opens save dialog via AppHandle + std::fs::write
Prefs: hand-rolled once_store JSON blob at app_config_dir/termcard-store.json (opaque from Rust: frontend-only fields like `lang` ride along without Rust changes)
```

- `run_capture` returns the capture **unredacted**. Redaction (`redact_capture_value`, `src-tauri/src/lib.rs:110`) is re-applied at render time on the JSON (`commandLine` + `lines[].runs[].text`), so editing rules updates the preview without re-capturing. `export_png` always redacts; `export_svg` honors `show_raw` (the "uncensored" toggle, `prefs.uncensored`), which affects only the preview, never a real export.
- Blocking work always goes through `tokio::task::spawn_blocking` (capture) / `tauri::async_runtime::spawn_blocking` (render), guarded by `AppState { stop: Arc<AtomicBool>, running: tokio::sync::Mutex<bool> }` (single capture at a time, 5MB buffer cap, 120s timeout, stop flag kills the PTY).
- `run_capture` takes `cwd: Option<String>`.

Rust backend (`termcard_lib`, flat modules): `lib.rs` (10 `#[tauri::command]`s + `AppState` + `once_store`), `capture.rs` (PTY), `ir.rs` (Capture/Line/Run/Color, `from_vt100` with run merging, `Capture::trimmed`), `redact.rs` (regex rules + LazyLock cache), `theme.rs` (Theme + `Preset::theme()` as single source of truth, ANSI-256 `Palette`), `export.rs` (hand-built SVG with base64-embedded JetBrains Mono, `CH_WIDTH 0.6` / `LINE_HEIGHT 1.2`, + resvg PNG).

Frontend: `main.tsx` → `App.tsx` (entire UI, one component; no router/store/context) → `api.ts` (10 typed `invoke()` wrappers + TS mirrors of Rust structs). Theme preset changes fetch the COMPLETE theme from Rust via `presetTheme` and replace it wholesale (no partial overrides survive a preset switch). `Ctrl/Cmd+Enter` runs the capture. UI strings come from `src/i18n.ts` (`translate()`, es/en dictionaries, `en: typeof es` so tsc enforces key parity); the language selector persists `prefs.lang`, App.tsx reads it through a `msg()` helper (never name it `t`: collides with the theme alias). Backend error strings are English; the frontend wraps them (`Error: {detail}`).

## Key Directories

- `src/` — React 19 frontend: `App.tsx`, `api.ts`, `i18n.ts` (es/en UI dictionaries, `detectLang`), `components/color-field.tsx` (custom react-colorful alpha picker in a Popover, handles "transparent"/#rrggbb/rgba() ↔ #rrggbbaa; takes translated `ariaLabel`/`hexAriaLabel`/`transparentLabel` props), `components/ui/` (shadcn `base-nova` on `@base-ui/react`), `styles.css` (Tailwind v4)
- `src-tauri/src/` — Rust backend modules above; `main.rs` is a thin shim calling `termcard_lib::run()`
- `src-tauri/fonts/` — JetBrains Mono TTFs, embedded via `include_bytes!` + `LazyLock`
- `src-tauri/tests/export_probe.rs` — self-declared diagnostic harness (renders a fixture to `/tmp/termcard-debug`, pixel-measures geometry); explicitly not a regression test, slated for deletion
- `docs/superpowers/specs/` — design spec (read before architectural changes)

## Development Commands

Package manager is **Bun** (`bun.lock`). Cargo/rustup live under `~/.cargo/bin`, **not on the default PATH** here.

```bash
bun install
bun run app        # full desktop dev (tauri dev; prepends ~/.cargo/bin to PATH)
bun run dev        # web-only Vite dev server (no Tauri shell), port 1420 strictPort
bun run build      # tsc && vite build — tsc IS the typecheck gate
bun run preview    # serve production build
bun run tauri build # release bundle (.deb/AppImage)
cd src-tauri && cargo test   # 37 tests (35 inline + 2 in export_probe.rs)
```

- Plain `bun run tauri dev` fails: `bun run app` exists precisely to prepend `~/.cargo/bin`.
- No ESLint/biome. Formatting: Prettier (4-space, width 100, double quotes, es5 trailing commas) + rustfmt (edition 2021).

## Code Conventions & Common Patterns

**Rust**

- Everything crossing IPC carries `#[serde(rename_all = "camelCase")]`; keep it on new IPC structs (`ir.rs`, `theme.rs`, `redact.rs`).
- Errors: `Result<_, String>` at the command boundary (`.map_err(|e| e.to_string())`). The one real error type is `CaptureError(pub String)` in `capture.rs`.
- State: `.manage(AppState { stop, running })`; `LazyLock` statics for fonts and regex cache; blocking work under `spawn_blocking`.
- Redaction mutates the serialized `serde_json::Value` directly (`redact_capture_value` in `lib.rs`) rather than round-tripping through structs.
- Comments, doc comments, assert messages, and backend error strings: English. Keep it consistent. User-facing UI strings go through `src/i18n.ts` in both languages.
- Persistence is the hand-rolled `once_store` module (`OnceLock<PathBuf>`, whole-JSON `get_prefs`/`save_prefs`, atomic write via tmp+rename). `tauri-plugin-store` was removed from `Cargo.toml` and the `@tauri-apps/plugin-store`/`@tauri-apps/plugin-dialog` JS packages from `package.json` — don't add plugin-store calls. The Rust `tauri-plugin-dialog` crate stays: Rust opens the save dialog itself via `AppHandle`.
- `[lib] name = "termcard_lib"`: the `_lib` suffix avoids a lib/bin name collision (Windows cargo issue); don't rename.

**TypeScript/React**

- Strict TS, `noUnusedLocals`/`noUnusedParameters`, `noFallthroughCasesInSwitch`, `allowImportingTsExtensions`. Path alias `@` → `./src` (declared in **both** `tsconfig.json` and `vite.config.ts` — update both).
- Imports use `@/api`, `@/components/ui/button`.
- State: plain `useState` + `useCallback`. Persistence pattern: `patchPrefs`/`patchTheme` shallow-merge state only; a `useEffect` on the prefs state persists with `void api.savePrefs(next)` (fire-and-forget, StrictMode-safe identity check, no save on first mount). Init: `getPrefs`; if the saved object has `command`, merge over `DEFAULT_PREFS` (theme merged keywise) and fill empty rules from `defaultRules()` (also as fallback if `getPrefs` fails); otherwise `defaultPrefs(getHome(), defaultRules())`.
- `src/lib/utils.ts` is `export { cn } from "cn"` — `cn` is an npm package, not the usual clsx + tailwind-merge combo.
- shadcn/ui on `@base-ui/react` (style `base-nova` per `components.json`), cva, lucide-react icons, `data-slot` attributes. Tailwind v4 CSS-first (`@theme inline`, oklch vars, `@custom-variant dark`); app is dark-only (hardcoded `class="dark"` in `index.html`).
- SVG imports: `*.svg?raw` typed in `src/vite-env.d.ts` (used for the GitHub mark in `src/assets/`).

**Cross-boundary contract**: adding an IPC command touches `lib.rs` (command + `generate_handler![]`) and `api.ts` (struct mirror + invoke wrapper). Missing either side fails `tsc` or the runtime invoke. Current 10 commands: `get_prefs`, `save_prefs`, `run_capture`, `stop_capture`, `export_png`, `export_svg`, `preset_theme`, `default_rules`, `get_home`, `save_png`.

## Important Files

- `src-tauri/src/lib.rs` — all IPC commands, `AppState`, redaction-at-render logic, `once_store` persistence
- `src-tauri/src/ir.rs` — the IR contract; changes ripple through capture, redact, theme, export, and api.ts
- `src/api.ts` — TS mirror of every Rust IPC struct plus the `api` object and `DEFAULT_PREFS`
- `src/App.tsx` — entire UI; `PRESET_LABELS` maps preset ids → i18n MessageKeys (actual preset definitions live in `theme.rs::Preset::theme`)
- `vite.config.ts`, `tsconfig.json`, `components.json` (shadcn: base-nova, Tailwind v4 at `src/styles.css`)

## Runtime/Tooling Preferences

- Bun for all JS tooling; `tauri.conf.json` `beforeDevCommand`/`beforeBuildCommand` use `bun run`.
- ESM throughout (`"type": "module"`, `moduleResolution: "bundler"`); no `.env` files, no `import.meta.env` usage.
- Rust edition 2021; release profile: `lto`, `codegen-units=1`, `opt-level=3`, `panic=abort`, `strip`. Lib crate named `termcard_lib`.
- Frontend dev server pinned to port 1420 (`strictPort`); Tauri expects `devUrl http://localhost:1420`. Vite ignores `**/src-tauri/**`, `clearScreen: false` so Rust errors stay visible.
- Capabilities (`src-tauri/capabilities/default.json`): only `core:default`, `opener:default`, `dialog:default` on window `main`. `csp: null`.

## Testing & QA

- Rust-only suite, 37 tests: 35 inline `#[cfg(test)] mod tests` at file bottom with `use super::*` (capture 6, ir 10, redact 9, theme 7, export 3) + 2 in `tests/export_probe.rs`. No frontend tests, no coverage tooling.
- Naming: snake_case behavioral names (`svg_escapes_xml`, `disabled_rules_skipped`). Fixtures are in-module helper functions, not files.
- `capture.rs` tests are **not hermetic**: they spawn real shell commands and depend on PATH, GNU `ls --color=always`, and `/tmp`. `stop_flag_kills_hanging_command` is the slowest/flakiest (150ms timing race, spawns `sleep 60`).
- Husky pre-commit (`.husky/pre-commit`): `bunx lint-staged` (prettier on web files, rustfmt on `src-tauri/**/*.rs`) **and** `cd src-tauri && cargo fmt --check && cargo test` — every commit runs the full Rust suite and needs cargo on PATH.
- Known invariants to keep in sync:
    - Theme defaults are duplicated: `Theme::default` (theme.rs) and `DEFAULT_PREFS` (api.ts). Change both.
    - Preset definitions live in `theme.rs::Preset::theme` only; `App.tsx::PRESET_LABELS` holds display labels.
    - The synthetic `❯ <command>` prompt line is rendered by both SVG and PNG renderers; the displayed command goes through redaction like captured text.
    - UI strings live only in `src/i18n.ts`: `es` is the key source, `en` typed `typeof es` (key parity is a tsc failure). Identical-in-both-languages literals ("Preset", "Padding", "regex", "Traffic lights", "termcard", "v0.1", "2×/3×/4×") stay inline.
