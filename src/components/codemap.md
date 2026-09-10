# src/components/

## Responsibility

`color-field.tsx` exports one component, `ColorField`: a color input for the theme editor that supports a real alpha channel. Its public value is a CSS string in one of three forms: `"transparent"`, `"#rrggbb"`, or `"rgba(r,g,b,a)"` (decimal alpha 0..1). Internally the picker state is `#rrggbbaa` (hex8), the native format of react-colorful's `HexAlphaColorPicker`. The component also owns the "transparent" convention: a gray/white checkerboard swatch (Chrome DevTools / Figma style) plus a `Transparent` button that resets to `#00000000`.

`src/components/ui/` is vendored shadcn boilerplate (`base-nova` preset on `@base-ui/react`) and is not documented here.

## Design

**Parsing and normalization.** Four module-level helpers handle all string conversion; no color math library is used:

- `toHex8(v)` normalizes any accepted input to hex8 for the picker: `"transparent"` → `#00000000`; `rgba(r,g,b,a)` → alpha `Math.round(a * 255)` appended as two hex digits; `#rrggbb` → appends `ff`; `#rrggbbaa` passes through; anything else falls back to `#000000ff`.
- `fromHex8(h8)` is the inverse, and it is deliberately lossy in two ways: `#00000000` maps back to the literal `"transparent"` (so round-tripping preserves the special value), and full alpha `...ff` is emitted as a bare `#rrggbb` (the compact form the theme store already uses). Intermediate alphas emit `rgba(r,g,b,a)` with alpha rounded to two decimals via `toFixed(2)`. A malformed hex8 input (e.g. a half-typed value in the text box) returns `#000000` as a safe fallback.
- `alphaOf(v)` extracts alpha 0..255, accepting both `rgba()` (via regex capture group 1) and hex8; unknown forms report 255. Used only for the `transparent` display check.

All `rgba()` parsing uses one regex, `/rgba\(\s*(\d+)\s*,\s*(\d+)\s*,\s*(\d+)\s*,\s*([\d.]+)\s*\)/`, tolerant of whitespace. Hex validation is strict (`^#[0-9a-fA-F]{6}$`, `^#[0-9a-fA-F]{8}$`); there is no support for `rgb()` arrays, `hsl()`, or named colors.

**State and popover sync.** Two pieces of state: `hex8` (picker value) and `open` (Popover). The prop `value` is the source of truth from the outside, but `hex8` is only re-synced from it in a `useEffect` guarded by `!open`. This avoids clobbering the picker mid-drag, which would make the color handle jump back to the previous position while presets or undo change the value underneath. The effect deps are `[value, open]`, so closing the popover also re-syncs.

`commit(h8)` is the single write path: it sets local `hex8` and calls `onChange(fromHex8(h8))`. Both the picker's `onChange` and the hex text `Input` route through it. The text input accepts partial typing (`setHex8(v)` without committing) and commits only when the trimmed string matches the strict 8-digit hex test.

**Accessibility props pattern.** The component takes no hardcoded UI strings. All labels come in as props from the caller: `ariaLabel` (swatch trigger, applied to `PopoverTrigger`), `hexAriaLabel` (the hex `Input`), and `transparentLabel` (the reset `Button`'s visible text). This keeps `ColorField` i18n-agnostic: `App.tsx` passes `msg()` translations from `src/i18n.ts`. The trigger also carries `title={value}` so hovering shows the current CSS value.

**Checkerboard.** `CHECKERBOARD` is a module-level `CSSProperties` constant: four 45° `linear-gradient` layers in a repeating 8px tile over white, applied to the inner `<span>` of the trigger whenever `value === "transparent" || alphaOf(value) === 0`.

## Flow

Input string → parse → react-colorful state → onChange → `#rrggbbaa` output:

1. A theme value (e.g. `"rgba(31,41,55,0.5)"`) arrives as the `value` prop.
2. On mount, `useState(() => toHex8(value))` lazily normalizes it to `#1f293780`.
3. While the popover is open, every drag on `HexAlphaColorPicker` calls `commit(hex8)`; `commit` stores the hex8 locally and emits `fromHex8(hex8)` back up through `onChange`. Because `fromHex8` emits `#rrggbb` for alpha `ff` and keeps rgba only for true partial alphas, the parent's theme state stays in the compact convention.
4. The parent re-renders with the new `value`; the `useEffect` skips the sync (`open` is true), so the picker state stays authoritative during the interaction.
5. On close, the effect re-runs and `hex8` is set from the (possibly externally modified) prop value.
6. Text-entry path: a keystroke updates `hex8` only; a valid `#rrggbbaa` string additionally commits, producing the same `onChange` payload.

## Integration

- Consumed by: `src/App.tsx` theme editor; uses shadcn ui/ primitives from `src/components/ui/` (`Popover`/`PopoverContent`/`PopoverTrigger`, `Button`, `Input`) on `@base-ui/react`, plus `cn` from `src/lib/utils`.
- Props `ariaLabel`, `hexAriaLabel`, `transparentLabel` are supplied by `App.tsx` via `msg()` from `src/i18n.ts`; the component never imports i18n itself.
- The emitted CSS strings flow into the theme object that is sent verbatim to the Rust backend (`export_svg`/`render_png`), so every value this component produces must stay parseable by `theme.rs` (`"transparent"`, `#rrggbb`, `rgba(r,g,b,a)`).
