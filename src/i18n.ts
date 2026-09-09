/**
 * Minimal i18n for the UI: flat typed dictionaries for Spanish and
 * English. No library (two languages, fifty-odd keys don't justify
 * a dependency). The language lives in `Prefs.lang` (JSON opaque to Rust),
 * is detected from the system on first run and persisted in the store.
 * Rust backend error strings are in English on purpose: the
 * frontend wraps them with `statusError`/`statusExportError`.
 */

export type Lang = "es" | "en";

export const LANGS: { id: Lang; label: string }[] = [
    { id: "es", label: "Español" },
    { id: "en", label: "English" },
];

/** System language: es-* → "es", anything else → "en" (fallback). */
export function detectLang(): Lang {
    const l = (navigator.languages?.[0] ?? navigator.language ?? "en").toLowerCase();
    return l.startsWith("es") ? "es" : "en";
}

// Spanish is the key source; typing `en` against `typeof es`
// forces key parity: tsc fails if a dictionary diverges.
const es = {
    statusLoading: "Cargando…",
    statusReady: "Listo.",
    statusPrefsError: "No se pudieron cargar preferencias.",
    statusEmptyCommand: "Escribe un comando primero.",
    statusRunning: "Ejecutando…",
    statusCaptureDone: "Captura lista: {n} líneas con contenido.",
    statusTruncated: "Salida truncada (límite 5 MB)",
    statusTimedOut: "Timeout: el proceso no terminó",
    statusExitCode: "código de salida {n}",
    statusError: "Error: {detail}",
    statusNoCapture: "No hay captura para exportar.",
    statusGeneratingPng: "Generando PNG ({scale}×)…",
    statusSavedTo: "Guardado en {path}",
    statusExportCancelled: "Exportación cancelada.",
    statusExportError: "Error exportando: {detail}",
    sectionCapture: "Captura",
    labelDirectory: "Directorio",
    actionRun: "Ejecutar",
    actionStop: "Detener",
    sectionRedaction: "Redacción",
    titleAddRule: "Añadir regla",
    titleRestoreRules: "Restaurar por defecto",
    placeholderReplacement: "reemplazo",
    titleDefaultRuleNoDelete: "Las reglas por defecto se desactivan, no borran",
    titleDeleteRule: "Eliminar",
    actionUncensoredShow: "Mostrar sin censura",
    actionUncensoredHide: "Ocultar datos",
    sectionTheme: "Tema",
    labelPromptSymbol: "Símbolo prompt",
    labelAccent: "Acento",
    labelBackdrop: "Fondo ext.",
    labelTitle: "Título",
    labelFontSize: "Fuente",
    labelRadius: "Radio",
    labelMargin: "Margen",
    labelShadow: "Sombra",
    transparentText: "transparente",
    ariaAccent: "Color de acento",
    ariaBackdrop: "Fondo exterior de la tarjeta",
    githubTitle: "termcard v0.1 — Repositorio en GitHub",
    labelScale: "Escala",
    actionExportPng: "Exportar PNG",
    statusGenerating: "Generando…",
    emptyNoCapture: "Sin captura todavía",
    emptyRunCommand: "Ejecuta un comando para generar la tarjeta.",
    emptyLoadingPrefs: "Recuperando preferencias guardadas.",
    colorTransparent: "Transparente",
    ariaColorHex: "Color en formato hexadecimal con alfa",
    presetMacDark: "Mac oscuro",
    presetMacLight: "Mac claro",
    presetMinimal: "Minimal",
    presetSolarized: "Solarized",
};

const en: typeof es = {
    statusLoading: "Loading…",
    statusReady: "Ready.",
    statusPrefsError: "Could not load preferences.",
    statusEmptyCommand: "Type a command first.",
    statusRunning: "Running…",
    statusCaptureDone: "Capture ready: {n} lines with content.",
    statusTruncated: "Output truncated (5 MB limit)",
    statusTimedOut: "Timeout: the process did not finish",
    statusExitCode: "exit code {n}",
    statusError: "Error: {detail}",
    statusNoCapture: "There is no capture to export.",
    statusGeneratingPng: "Generating PNG ({scale}×)…",
    statusSavedTo: "Saved to {path}",
    statusExportCancelled: "Export cancelled.",
    statusExportError: "Error exporting: {detail}",
    sectionCapture: "Capture",
    labelDirectory: "Directory",
    actionRun: "Run",
    actionStop: "Stop",
    sectionRedaction: "Redaction",
    titleAddRule: "Add rule",
    titleRestoreRules: "Restore defaults",
    placeholderReplacement: "replacement",
    titleDefaultRuleNoDelete: "Default rules are disabled, not deleted",
    titleDeleteRule: "Delete",
    actionUncensoredShow: "Show uncensored",
    actionUncensoredHide: "Hide sensitive data",
    sectionTheme: "Theme",
    labelPromptSymbol: "Prompt symbol",
    labelAccent: "Accent",
    labelBackdrop: "Backdrop",
    labelTitle: "Title",
    labelFontSize: "Font",
    labelRadius: "Radius",
    labelMargin: "Margin",
    labelShadow: "Shadow",
    transparentText: "transparent",
    ariaAccent: "Accent color",
    ariaBackdrop: "Card outer background",
    githubTitle: "termcard v0.1 — GitHub repository",
    labelScale: "Scale",
    actionExportPng: "Export PNG",
    statusGenerating: "Generating…",
    emptyNoCapture: "No capture yet",
    emptyRunCommand: "Run a command to generate the card.",
    emptyLoadingPrefs: "Loading saved preferences.",
    colorTransparent: "Transparent",
    ariaColorHex: "Color in hexadecimal with alpha",
    presetMacDark: "Mac dark",
    presetMacLight: "Mac light",
    presetMinimal: "Minimal",
    presetSolarized: "Solarized",
};

export type MessageKey = keyof typeof es;

/** Simple interpolation: "{n}" → params.n. Missing parameter → empty string. */
export function translate(
    lang: Lang,
    key: MessageKey,
    params?: Record<string, string | number>
): string {
    return (lang === "en" ? en : es)[key].replace(/\{(\w+)\}/g, (_, k: string) =>
        String(params?.[k] ?? "")
    );
}
