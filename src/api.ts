import { invoke } from "@tauri-apps/api/core";

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
  | { indexed: number }
  | { rgb: [number, number, number] }
  | "default"
  | "defaultInverted";

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
}

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
    title: "usuario@localhost: ~",
    showTrafficLights: true,
    showShadow: true,
    cornerRadius: 12,
    padding: 24,
    fontSize: 14,
    promptSymbol: "❯",
  },
  scale: 2,
  uncensored: false,
};

export const api = {
  getPrefs: (): Promise<Prefs> => invoke("get_prefs"),
  savePrefs: (prefs: Prefs): Promise<void> => invoke("save_prefs", { prefs }),
  runCapture: (
    command: string,
    cwd: string,
    rules: RedactRule[],
  ): Promise<Capture> =>
    invoke("run_capture", { command, cwd, rules }),
  stopCapture: (): Promise<void> => invoke("stop_capture"),
  exportPng: (
    capture: Capture,
    theme: Theme,
    scale: number,
  ): Promise<string> => invoke("export_png", { capture, theme, scale }),
  defaultRules: (): Promise<RedactRule[]> => invoke("default_rules"),
  getHome: (): Promise<string> => invoke("get_home"),
  savePng: (base64Png: string, suggestedName: string): Promise<string> =>
    invoke("save_png", { base64Png, suggestedName }),
};