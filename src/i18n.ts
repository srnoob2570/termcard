/**
 * i18n minimalista de la UI: diccionarios planos tipados para español e
 * inglés. Sin librería (dos idiomas, medio centenar de claves no justifican
 * una dependencia). El idioma vive en `Prefs.lang` (JSON opaco para Rust),
 * se detecta del sistema en el primer arranque y persiste en el store.
 * Las cadenas de error del backend Rust quedan en español a propósito: el
 * frontend las envuelve con `statusError`/`statusExportError`.
 */

export type Lang = "es" | "en";

export const LANGS: { id: Lang; label: string }[] = [
    { id: "es", label: "Español" },
    { id: "en", label: "English" },
];

/** Idioma del sistema: es-* → "es", cualquier otra cosa → "en" (fallback). */
export function detectLang(): Lang {
    const l = (navigator.languages?.[0] ?? navigator.language ?? "en").toLowerCase();
    return l.startsWith("es") ? "es" : "en";
}

// El español es la fuente de las claves; tipar `en` contra `typeof es`
// obliga a paridad de claves: tsc falla si un diccionario diverge.
const es = {
    statusLoading: "Cargando…",
    statusReady: "Listo.",
    statusPrefsError: "No se pudieron cargar preferencias.",
    statusEmptyCommand: "Escribe un comando primero.",
    statusRunning: "Ejecutando…",
    statusCaptureDone: "Captura lista: {n} líneas con contenido.",
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

/** Interpolación simple: "{n}" → params.n. Parámetro ausente → cadena vacía. */
export function translate(
    lang: Lang,
    key: MessageKey,
    params?: Record<string, string | number>
): string {
    return (lang === "en" ? en : es)[key].replace(/\{(\w+)\}/g, (_, k: string) =>
        String(params?.[k] ?? "")
    );
}
