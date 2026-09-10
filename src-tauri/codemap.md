# src-tauri/

## Responsibility

Tauri 2 desktop shell for Termcard: application configuration (`tauri.conf.json`), capability/permission grants (`capabilities/default.json`), dependency manifest (`Cargo.toml`), the cargo build hook (`build.rs`), and the embedded font assets (`fonts/`). This directory owns packaging and security policy; the actual Tauri backend code lives in `src/` (see its own codemap).

## Design

- **`Cargo.toml`** — package `termcard` v0.1.2, edition 2021. The `[lib]` section sets `name = "termcard_lib"` with `crate-type = ["staticlib", "cdylib", "rlib"]`. The `_lib` suffix exists because cargo cannot have a lib and a bin with the same name — a Windows-only issue (rust-lang/cargo#8519) — but the rename is kept unconditionally. Don't rename.
- **Dependencies**: `tauri 2` + `tauri-plugin-opener 2` + `tauri-plugin-dialog 2.7.3` (Rust side opens the save dialog itself via `AppHandle`; there is no `tauri-plugin-store` — persistence is the hand-rolled `once_store` in `src/lib.rs`), IPC/serialization via `serde` (derive) + `serde_json`, terminal emulation via `portable-pty 0.9.0` + `vt100 0.16.2`, rendering via `resvg 0.48.1` / `usvg 0.48.1` / `tiny-skia 0.12.0` with `fontdb 0.24.0` for font resolution, `regex 1.13.1` (redaction rules, `LazyLock` cache), `unicode-width 0.2`, `base64 0.23.1` (font payload + PNG transfer), `parking_lot 0.12.5`, `tokio 1.53.1` (`rt`, `macros`). Build dependency: `tauri-build 2`.
- **`[profile.release]`**: `lto = true`, `codegen-units = 1`, `opt-level = 3`, `panic = "abort"`, `strip = true` — the Tauri size-optimization guideline configuration.
- **`tauri.conf.json`** (schema `config/2`): `productName: "termcard"`, `identifier: "com.srnoob.termcard"`, version `0.1.2`. Build block: `beforeDevCommand: "bun run dev"`, `devUrl: "http://localhost:1420"` (the port is pinned by `strictPort` in `vite.config.ts`; plain `bun run tauri dev` fails because cargo isn't on PATH — the root `bun run app` script wraps `tauri dev` and prepends `~/.cargo/bin`), `beforeBuildCommand: "bun run build"`, `frontendDist: "../dist"`. Single window `main`, 800×600, title `termcard`. `app.security.csp: null` (no CSP; content is local). Bundle: `targets: "all"`, icons under `icons/` (32x32.png, 128x128.png, 128x128@2x.png, icon.icns, icon.ico).
- **`capabilities/default.json`** — capability `default` scoped to window `main` with exactly three permission sets: `core:default`, `opener:default`, `dialog:default`. Nothing else is granted; new plugin permissions must be added here.
- **`build.rs`** — three lines calling `tauri_build::build()`; the standard Tauri codegen hook (generates `gen/` artifacts and the embedded config the `tauri` macros consume).
- **`fonts/`** — four JetBrains Mono TTFs (Regular, Bold, Italic, BoldItalic). They are NOT loaded from disk at runtime: `src/export.rs` embeds them at compile time via `include_bytes!` behind `LazyLock` statics. `font_css` serves the CSS `@font-face` (base64, ~1.4 MB) so the webview preview renders the same font, and `render_png` resolves the same TTFs through `fontdb`. The directory is asset storage for the Rust binary only.

## Flow

Build pipeline (frontend first, then shell):

1. `bun run build` (via `beforeBuildCommand`) → `tsc && vite build` → static assets in `../dist`.
2. `bun run tauri build` → cargo compiles the crate: `build.rs` runs `tauri_build::build()`; the binary target is `src/main.rs`, a thin shim that only calls `termcard_lib::run()` (plus the Windows `windows_subsystem = "windows"` attribute to suppress the release console window); all logic lives in the `termcard_lib` rlib.
3. Bundler produces `.deb` / AppImage (`targets: "all"`) with the configured icons.

Dev flow: `bun run app` → `tauri dev` → `beforeDevCommand` starts Vite on port 1420 → Tauri opens the `main` window pointed at `devUrl`, hot-reloading from the dev server while cargo watches the Rust side.
