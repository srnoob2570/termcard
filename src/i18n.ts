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
    labelDirectory: "Directorio",
    browseDirectory: "Elegir carpeta…",
    actionRun: "Ejecutar",
    actionStop: "Detener",
    modeInteractive: "Interactivo",
    modeManual: "Manual",
    labelCommand: "Comando",
    labelOutput: "Salida",
    placeholderOutput: "Pega la salida del comando (texto plano o HTML)",
    actionGenerate: "Generar",
    statusEmptyOutput: "Escribe o pega la salida del comando.",
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
    labelWidth: "Ancho",
    widthAuto: "Auto",
    widthManual: "Manual",
    transparentText: "transparente",
    ariaAccent: "Color de acento",
    ariaBackdrop: "Fondo exterior de la tarjeta",
    githubTitle: "termcard v{version} — Repositorio en GitHub",
    labelScale: "Escala",
    actionExportPng: "Exportar PNG",
    statusGenerating: "Generando…",
    emptyNoCapture: "Sin captura todavía",
    emptyRunCommand: "Ejecuta un comando para generar la tarjeta.",
    emptyLoadingPrefs: "Recuperando preferencias guardadas.",
    colorTransparent: "Transparente",
    ariaColorHex: "Color en formato hexadecimal con alfa",
    presetMacDark: "Mac oscuro",
    presetTokyoNight: "Tokyo Night",
    presetDracula: "Dracula",
    presetGruvboxDark: "Gruvbox oscuro",
    presetNord: "Nord",
    presetSolarized: "Solarized",
    presetMacLight: "Mac claro",
    presetLatte: "Catppuccin Latte",
    presetSolarizedLight: "Solarized claro",
    presetGruvboxLight: "Gruvbox claro",
    presetNordLight: "Nord claro",
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
    labelDirectory: "Directory",
    browseDirectory: "Browse folder…",
    actionRun: "Run",
    actionStop: "Stop",
    modeInteractive: "Interactive",
    modeManual: "Manual",
    labelCommand: "Command",
    labelOutput: "Output",
    placeholderOutput: "Paste the command output (plain text or HTML)",
    actionGenerate: "Generate",
    statusEmptyOutput: "Type or paste the command output first.",
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
    labelWidth: "Width",
    widthAuto: "Auto",
    widthManual: "Manual",
    ariaBackdrop: "Card outer background",
    githubTitle: "termcard v{version} — GitHub repository",
    labelScale: "Scale",
    actionExportPng: "Export PNG",
    statusGenerating: "Generating…",
    emptyNoCapture: "No capture yet",
    emptyRunCommand: "Run a command to generate the card.",
    emptyLoadingPrefs: "Loading saved preferences.",
    colorTransparent: "Transparent",
    ariaColorHex: "Color in hexadecimal with alpha",
    presetMacDark: "Mac dark",
    presetTokyoNight: "Tokyo Night",
    presetDracula: "Dracula",
    presetGruvboxDark: "Gruvbox dark",
    presetNord: "Nord",
    presetSolarized: "Solarized",
    presetMacLight: "Mac light",
    presetLatte: "Catppuccin Latte",
    presetSolarizedLight: "Solarized light",
    presetGruvboxLight: "Gruvbox light",
    presetNordLight: "Nord light",
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
