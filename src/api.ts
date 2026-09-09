import { invoke } from "@tauri-apps/api/core";
import { detectLang, type Lang } from "@/i18n";

export interface Run {
    text: string;
    fg: Color | null;
    bg: Color | null;
    bold: boolean;
    italic: boolean;
    underline: boolean;
}

export interface Line {
    runs: Run[];
}

export interface Capture {
    cols: number;
    rows: number;
    commandLine: string;
    lines: Line[];
}

export type Color =
    { indexed: number } | { rgb: [number, number, number] } | "default" | "defaultInverted";

/** Result of `run_capture`: the unredacted capture plus process outcome flags. */
export interface CaptureResult {
    capture: Capture;
    exitCode: number | null;
    truncated: boolean;
    timedOut: boolean;
}

export interface RedactRule {
    pattern: string;
    replacement: string;
    enabled: boolean;
    isDefault: boolean;
}

export interface Theme {
    preset: string;
    backdrop: string;
    background: string;
    foreground: string;
    accent: string;
    title: string;
    showTrafficLights: boolean;
    showShadow: boolean;
    cornerRadius: number;
    outerMargin: number;
    padding: number;
    fontSize: number;
    promptSymbol: string;
}

export interface Prefs {
    command: string;
    cwd: string;
    rules: RedactRule[];
    theme: Theme;
    scale: number;
    uncensored: boolean;
    lang: Lang;
}

// TS mirror of `Theme::default` + `Prefs` in src-tauri/src/theme.rs and lib.rs.
// The "mac-dark" preset IS Theme::default(): a single definition in Rust;
// this copy only covers the first render before the first IPC.
export const DEFAULT_PREFS: Prefs = {
    command: "",
    cwd: "",
    rules: [],
    theme: {
        preset: "mac-dark",
        backdrop: "transparent",
        background: "#1e1e2e",
        foreground: "#cdd6f4",
        accent: "#cba6f7",
        title: "user@localhost: ~",
        showTrafficLights: true,
        showShadow: true,
        cornerRadius: 12,
        outerMargin: 16,
        padding: 24,
        fontSize: 14,
        promptSymbol: "❯",
    },
    scale: 2,
    uncensored: false,
    lang: detectLang(),
};

export const api = {
    getPrefs: (): Promise<Prefs> => invoke("get_prefs"),
    savePrefs: (prefs: Prefs): Promise<void> => invoke("save_prefs", { prefs }),
    runCapture: (command: string, cwd: string): Promise<CaptureResult> =>
        invoke("run_capture", { command, cwd: cwd || null }),
    stopCapture: (): Promise<void> => invoke("stop_capture"),
    exportPng: (
        capture: Capture,
        theme: Theme,
        rules: RedactRule[],
        scale: number
    ): Promise<string> => invoke("export_png", { capture, theme, rules, scale }),
    exportSvg: (
        capture: Capture,
        theme: Theme,
        rules: RedactRule[],
        showRaw: boolean
    ): Promise<string> => invoke("export_svg", { capture, theme, rules, showRaw }),
    presetTheme: (preset: string): Promise<Theme> => invoke("preset_theme", { preset }),
    defaultRules: (): Promise<RedactRule[]> => invoke("default_rules"),
    getHome: (): Promise<string> => invoke("get_home"),
    savePng: (base64Png: string, suggestedName: string): Promise<string> =>
        invoke("save_png", { base64Png, suggestedName }),
};
