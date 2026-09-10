# src/

## Responsibility

Tauri 2 renderer layer: single-component React UI for running shell commands (PTY or manually pasted output), live-previewing the terminal card as the exact exported SVG, and exporting it as PNG/SVG via typed IPC bridges to the Rust backend.

## Design

- **Single-component UI**: `App.tsx` holds the entire interface (no router, no store, no context); plain `useState` + `useCallback` for all state, DOM-keyboard listener for the `Ctrl/Cmd+Enter` capture trigger.
- **Single-renderer principle**: the live preview IS the exported SVG — `api.exportSvg()` output is injected verbatim via `dangerouslySetInnerHTML`, so preview and export can never diverge. Redaction is re-applied at render time in Rust, so editing rules re-renders the preview without re-capturing.
- **IPC boundary** (`api.ts`): 12 typed `invoke()` wrappers (`getPrefs`, `savePrefs`, `runCapture`, `stopCapture`, `exportPng`, `exportSvg`, `fontCss`, `presetTheme`, `defaultRules`, `getHome`, `savePng`, `pickDirectory`) plus TS mirrors of every Rust struct (`Capture`, `Line`, `Run`, `Color` discriminated union, `RedactRule`, `Theme`, `Prefs`, `CaptureResult` with camelCase-matched fields). `DEFAULT_PREFS` is a first-render-only copy of `Theme::default` + `Prefs`.
- **Persistence pattern**: `patchPrefs`/`patchTheme` shallow-merge state; a `useEffect` on `prefs` fire-and-forget saves via `api.savePrefs(next)`, guarded by a `useRef<Prefs>` identity check that skips the initial mount and React `StrictMode`'s dev double-invocation.
- **i18n without a library** (`i18n.ts`): flat dictionaries, `es` is the key source, `en: typeof es` makes key parity a `tsc` failure; `translate()` does `{param}` interpolation; `detectLang()` maps system language (`navigator.languages[0]`) to `es`/`en`. App consumes it through a `msg()` `useCallback` bound to `prefs.lang` (never named `t`, which collides with the theme alias).
- **Manual capture** (`manual.ts`): `buildManualCapture()` parses pasted plain text or HTML (`DOMParser` walk with `<pre>`/block-tag line breaks, inline style/`b/i/u` to `Run` styling) into the same `Capture` IR, mimicking Rust `Capture::trimmed` (trailing-blank trim, `cols` fitted to widest line), so manual output feeds the unchanged export pipeline.
- **Font delivery**: `font_css` command ships the JetBrains Mono `@font-face` (~1.4 MB base64) once at document level in a startup `<style>` tag; preview SVGs carry only `font-family`, never the embedded payload.
- **Trust boundaries**: saved prefs are validated on load (lang enum check, mode whitelist, empty rules refilled from `defaultRules`); the `uncensored` toggle is preview-only and forced off before each capture.
- **Styling**: Tailwind v4 CSS-first (`@theme inline`, oklch vars, `@custom-variant dark`) with shadcn tokens; app is dark-only; Geist Variable for chrome, JetBrains Mono for card content.

## Flow

1. `main.tsx` mounts `<App />` under `StrictMode`; a mount `useEffect` calls `api.fontCss()` and injects the `@font-face` CSS into `document.head` once.
2. Init `useEffect`: `api.getPrefs()` → if saved prefs have `command`, shallow-merge over `DEFAULT_PREFS` (theme merged keywise, lang/mode validated, empty rules filled from `api.defaultRules()`); otherwise build from `api.getHome()` + `api.defaultRules()`. On failure, fall back to default rules and set an error status; `ready` flips true.
3. Every prefs change persists via the `savePrefs` `useEffect` (identity-check + `void api.savePrefs`); a `useEffect` syncs `document.documentElement.lang` to `prefs.lang`.
4. User types a command (or switches to manual mode and pastes output), presses Run / Generate or `Ctrl/Cmd+Enter`: `run()` calls `api.runCapture(command, cwd)` → backend PTY capture → `res.capture` stored in state (plus `truncated`/`timedOut`/`exitCode` status flags); manual mode instead calls `buildManualCapture()`.
5. A `useEffect` keyed on `[capture, prefs.theme, prefs.rules, prefs.uncensored]` calls `api.exportSvg(capture, theme, rules, uncensored)` and injects the returned SVG string via `dangerouslySetInnerHTML`; `null` capture shows an `Empty` state.
6. Preset switch calls `api.presetTheme(id)` and replaces the theme **wholesale** via `patchTheme(full)` so no partial overrides survive a preset switch; individual fields edit through `patchTheme` (card width uses a draft-width input committed on blur/Enter, clamped 100–4096).
7. Redaction rules are edited in place (`setRule`/`removeRule`/`addRule`/`restoreRules` via `api.defaultRules()`), each mutation patching `prefs.rules` and triggering step 5's re-render.
8. Export: `exportPng()` → `api.exportPng(capture, theme, rules, scale)` (2×/3×/4×) returns base64 PNG → `api.savePng(b64, "termcard-<timestamp>.png")` → Rust opens the native save dialog and writes the file; status footer reports the saved path or cancellation.
9. While running, the Stop button calls `api.stopCapture()` to trip the backend stop flag; the GitHub link routes through `@tauri-apps/plugin-opener`'s `openUrl` (target=_blank is unreliable in the webview).

## Integration

- **Depends on**: `@tauri-apps/api/core` (`invoke`), `@tauri-apps/plugin-opener` (`openUrl`), `lucide-react` icons, `react` 19, Rust IPC commands (`get_prefs`, `save_prefs`, `run_capture`, `stop_capture`, `export_png`, `export_svg`, `font_css`, `preset_theme`, `default_rules`, `get_home`, `save_png`, `pick_directory`), Vite `define` global `__APP_VERSION__`, raw SVG imports (`*.svg?raw`).
- **Consumed by**: `main.tsx` (mounts `App`); `src/components/` (shadcn `base-nova` UI primitives, `ColorField`); `src/lib/utils.ts` (`cn`); sibling modules `api.ts`, `i18n.ts`, `manual.ts` (consumed by `App.tsx`).
