import { useCallback, useEffect, useState } from "react";
import {
  Camera,
  Download,
  Eye,
  EyeOff,
  OctagonX,
  Palette,
  Plus,
  RotateCcw,
  Terminal,
  Trash2,
} from "lucide-react";

import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
  Field,
  FieldGroup,
  FieldLabel,
} from "@/components/ui/field";
import {
  Empty,
  EmptyDescription,
  EmptyHeader,
  EmptyMedia,
  EmptyTitle,
} from "@/components/ui/empty";
import { Spinner } from "@/components/ui/spinner";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Separator } from "@/components/ui/separator";
import { Switch } from "@/components/ui/switch";
import { Textarea } from "@/components/ui/textarea";
import {
  DEFAULT_PREFS,
  api,
  type Capture,
  type Prefs,
  type RedactRule,
  type Theme,
} from "@/api";
import { defaultPrefs, esc, renderCaptureHtml } from "@/preview";

const PRESET_LABELS: Record<string, string> = {
  "mac-dark": "Mac oscuro",
  "mac-light": "Mac claro",
  minimal: "Minimal",
  solarized: "Solarized",
};

const PRESET_OVERRIDES: Record<string, Partial<Theme>> = {
  "mac-dark": {},
  "mac-light": {
    backdrop: "transparent",
    background: "#f5f5f4",
    foreground: "#3f3f46",
    accent: "#d20f39",
    showTrafficLights: true,
    showShadow: true,
    cornerRadius: 12,
  },
  minimal: {
    backdrop: "transparent",
    background: "#1e1e2e",
    foreground: "#cdd6f4",
    accent: "#89b4fa",
    showTrafficLights: false,
    showShadow: false,
    cornerRadius: 6,
  },
  solarized: {
    backdrop: "#002b36",
    background: "#002b36",
    foreground: "#93a1a1",
    accent: "#b58900",
    showTrafficLights: true,
    showShadow: false,
    cornerRadius: 8,
  },
};

const ANSI_PALETTES: Record<string, string[]> = {
  "mac-dark": [
    "#45475a", "#f38ba8", "#a6e3a1", "#f9e2af",
    "#89b4fa", "#f5c2e7", "#94e2d5", "#bac2de",
    "#585b70", "#f38ba8", "#a6e3a1", "#f9e2af",
    "#89b4fa", "#f5c2e7", "#94e2d5", "#a6adc8",
  ],
  "mac-light": [
    "#5c5f77", "#d20f39", "#40a02b", "#df8e1d",
    "#1e66f5", "#ea76cb", "#179299", "#8c8fa1",
    "#6c6f85", "#d20f39", "#40a02b", "#df8e1d",
    "#1e66f5", "#ea76cb", "#179299", "#9ca0b0",
  ],
  minimal: [
    "#585858", "#d75f5f", "#5faf5f", "#d7af5f",
    "#5f87d7", "#d787d7", "#5fafaf", "#bcbcbc",
    "#6c6c6c", "#ff5f5f", "#87d787", "#ffffaf",
    "#87afff", "#ffafff", "#87d7d7", "#e4e4e4",
  ],
  solarized: [
    "#073642", "#dc322f", "#859900", "#b58900",
    "#268bd2", "#d33682", "#2aa198", "#eee8d5",
    "#002b36", "#cb4b16", "#586e75", "#657b83",
    "#839496", "#6c71c4", "#93a1a1", "#fdf6e3",
  ],
};

function applyPalette(preset: string): void {
  const palette = ANSI_PALETTES[preset] ?? ANSI_PALETTES["mac-dark"];
  palette.forEach((c, i) => document.documentElement.style.setProperty(`--ansi-${i}`, c));
}

function cardVars(t: Theme): React.CSSProperties {
  return {
    "--tc-bg": t.background,
    "--tc-fg": t.foreground,
    "--tc-accent": t.accent,
    "--tc-radius": `${t.cornerRadius}px`,
    "--tc-padding": `${t.padding}px`,
    "--tc-font-size": `${t.fontSize}px`,
    "--tc-shadow": t.showShadow ? "0 8px 32px rgba(0,0,0,0.35)" : "none",
  } as React.CSSProperties;
}

export default function App() {
  const [prefs, setPrefs] = useState<Prefs>(DEFAULT_PREFS);
  const [capture, setCapture] = useState<Capture | null>(null);
  const [running, setRunning] = useState(false);
  const [exporting, setExporting] = useState(false);
  const [status, setStatusState] = useState<{ msg: string; err: boolean }>({ msg: "Cargando…", err: false });
  const setStatus = (msg: string, err = false) => setStatusState({ msg, err });
  const [ready, setReady] = useState(false);

  // ---- init ----
  useEffect(() => {
    void (async () => {
      try {
        const saved = await api.getPrefs();
        let next = DEFAULT_PREFS;
        if (saved && typeof saved === "object" && "command" in saved) {
          next = {
            ...DEFAULT_PREFS,
            ...saved,
            theme: { ...DEFAULT_PREFS.theme, ...saved.theme },
          };
          if (!next.rules.length) next.rules = await api.defaultRules();
        } else {
          next = defaultPrefs(await api.getHome(), await api.defaultRules());
        }
        setPrefs(next);
      } catch {
        setStatus("No se pudieron cargar preferencias.", true);
      } finally {
        setReady(true);
        setStatus("Listo.");
      }
    })();
  }, []);

  useEffect(() => applyPalette(prefs.theme.preset), [prefs.theme.preset]);

  const patchPrefs = useCallback((patch: Partial<Prefs>) => {
    setPrefs((p) => {
      const next = { ...p, ...patch };
      void api.savePrefs(next);
      return next;
    });
  }, []);

  const patchTheme = useCallback((patch: Partial<Theme>) => {
    setPrefs((p) => {
      const next = { ...p, theme: { ...p.theme, ...patch } };
      void api.savePrefs(next);
      return next;
    });
  }, []);

  // ---- captura ----
  const run = useCallback(async () => {
    if (running) return;
    if (!prefs.command.trim()) {
      setStatus("Escribe un comando primero.", true);
      return;
    }
    setRunning(true);
    setStatus("Ejecutando…");
    try {
      const cap = await api.runCapture(prefs.command, prefs.cwd, prefs.rules);
      setCapture(cap);
      const n = cap.lines.filter((l) => l.runs.length > 0).length;
      setStatus(`Captura lista: ${n} líneas con contenido.`);
    } catch (e) {
      setStatus(`Error: ${e}`, true);
    } finally {
      setRunning(false);
    }
  }, [prefs, running]);

  useEffect(() => {
    const h = (e: KeyboardEvent) => {
      if ((e.ctrlKey || e.metaKey) && e.key === "Enter") {
        e.preventDefault();
        void run();
      }
    };
    document.addEventListener("keydown", h);
    return () => document.removeEventListener("keydown", h);
  }, [run]);

  // ---- export ----
  const exportPng = useCallback(async () => {
    if (!capture) {
      setStatus("No hay captura para exportar.", true);
      return;
    }
    setExporting(true);
    setStatus("Generando PNG…");
    try {
      const b64 = await api.exportPng(capture, prefs.theme, prefs.scale);
      const saved = await api.savePng(b64, `termcard-${Date.now()}.png`);
      setStatus(saved ? `Guardado en ${saved}` : "Exportación cancelada.");
    } catch (e) {
      setStatus(`Error exportando: ${e}`, true);
    } finally {
      setExporting(false);
    }
  }, [capture, prefs]);

  // ---- reglas ----
  const setRule = (i: number, patch: Partial<RedactRule>) => {
    patchPrefs({
      rules: prefs.rules.map((r, j) => (j === i ? { ...r, ...patch } : r)),
    });
  };
  const removeRule = (i: number) => {
    patchPrefs({ rules: prefs.rules.filter((_, j) => j !== i) });
  };
  const addRule = () => {
    patchPrefs({
      rules: [...prefs.rules, { pattern: "", replacement: "", enabled: true, isDefault: false }],
    });
  };
  const restoreRules = async () => {
    patchPrefs({ rules: await api.defaultRules() });
  };

  const t = prefs.theme;
  const body = capture ? renderCaptureHtml(capture, t) : "";

  return (
    <div className="flex h-screen overflow-hidden">
      {/* Panel izquierdo */}
      <aside className="flex w-[360px] shrink-0 flex-col overflow-y-auto border-r bg-sidebar p-4">
        <div className="mb-4 flex items-center gap-2">
          <Terminal className="size-5 text-primary" />
          <h1 className="text-base font-semibold tracking-tight">termcard</h1>
          <Badge variant="secondary" className="ml-auto font-mono text-[10px]">
            v0.1
          </Badge>
        </div>

        {/* Captura */}
        <section>
          <FieldGroup>
            <Field>
              <FieldLabel className="text-xs uppercase text-muted-foreground tracking-wide">
                Captura
              </FieldLabel>
              <Textarea
                id="command"
                rows={3}
                className="font-mono text-[13px]"
                placeholder="ls --color=auto -la"
                value={prefs.command}
                onChange={(e) => patchPrefs({ command: e.target.value })}
              />
            </Field>
            <Field>
              <FieldLabel htmlFor="cwd" className="text-xs">
                Directorio
              </FieldLabel>
              <Input
                id="cwd"
                className="h-8 font-mono text-xs"
                placeholder="~"
                value={prefs.cwd}
                onChange={(e) => patchPrefs({ cwd: e.target.value })}
              />
            </Field>
          </FieldGroup>
          <div className="mt-3 flex gap-2">
            <Button onClick={() => void run()} disabled={running} className="flex-1">
              {running ? (
                <Spinner data-icon="inline-start" />
              ) : (
                <Camera data-icon="inline-start" />
              )}
              {running ? "Ejecutando…" : "Ejecutar"}
            </Button>
            {running && (
              <Button variant="outline" onClick={() => void api.stopCapture()}>
                <OctagonX data-icon="inline-start" />
                Detener
              </Button>
            )}
          </div>
        </section>

        <Separator className="my-4" />

        {/* Redacción */}
        <section>
          <div className="flex items-center justify-between">
            <Label className="text-xs uppercase text-muted-foreground tracking-wide">
              Redacción
            </Label>
            <div className="flex gap-1">
              <Button size="icon-xs" variant="ghost" onClick={addRule} title="Añadir regla">
                <Plus />
              </Button>
              <Button
                size="icon-xs"
                variant="ghost"
                onClick={() => void restoreRules()}
                title="Restaurar por defecto"
              >
                <RotateCcw />
              </Button>
            </div>
          </div>
          <div className="mt-2 space-y-2">
            {prefs.rules.map((rule, i) => (
              <div key={i} className="flex items-center gap-1.5">
                <Checkbox
                  checked={rule.enabled}
                  onCheckedChange={(v) => setRule(i, { enabled: v === true })}
                  className="size-4 shrink-0"
                />
                <Input
                  className="h-7 flex-[1.4] px-2 font-mono text-[11px]"
                  placeholder="regex"
                  value={rule.pattern}
                  onChange={(e) => setRule(i, { pattern: e.target.value })}
                />
                <span className="text-xs text-muted-foreground">→</span>
                <Input
                  className="h-7 flex-1 px-2 text-[11px]"
                  placeholder="reemplazo"
                  value={rule.replacement}
                  onChange={(e) => setRule(i, { replacement: e.target.value })}
                />
                <Button
                  size="icon-xs"
                  variant="ghost"
                  disabled={rule.isDefault}
                  onClick={() => removeRule(i)}
                  title={rule.isDefault ? "Las reglas por defecto se desactivan, no borran" : "Eliminar"}
                >
                  <Trash2 />
                </Button>
              </div>
            ))}
          </div>
          <Button
            variant="ghost"
            size="sm"
            className="mt-2 w-full text-muted-foreground"
            onClick={() => patchPrefs({ uncensored: !prefs.uncensored })}
          >
            {prefs.uncensored ? <EyeOff data-icon="inline-start" /> : <Eye data-icon="inline-start" />}
            {prefs.uncensored ? "Ocultar datos" : "Mostrar sin censura"}
          </Button>
        </section>

        <Separator className="my-4" />

        {/* Tema */}
        <section>
          <div className="flex items-center gap-1.5">
            <Palette className="size-3.5 text-muted-foreground" />
            <Label className="text-xs uppercase text-muted-foreground tracking-wide">
              Tema
            </Label>
          </div>
          <div className="mt-2 grid grid-cols-2 gap-2">
            <div>
              <Label className="text-xs">Preset</Label>
              <Select
                value={t.preset}
                onValueChange={(v) => { if (v) patchTheme({ preset: v, ...PRESET_OVERRIDES[v] }); }}
              >
                <SelectTrigger className="mt-1 h-8 w-full">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  {Object.entries(PRESET_LABELS).map(([v, l]) => (
                    <SelectItem key={v} value={v}>
                      {l}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </div>
            <div>
              <Label className="text-xs">Símbolo prompt</Label>
              <Input
                className="mt-1 h-8 text-center font-mono"
                maxLength={3}
                value={t.promptSymbol}
                onChange={(e) => patchTheme({ promptSymbol: e.target.value })}
              />
            </div>
          </div>
          <div className="mt-2 grid grid-cols-2 gap-2">
            <div>
              <Label className="text-xs">Acento</Label>
              <Input
                type="color"
                className="mt-1 h-8 p-1"
                value={t.accent}
                onChange={(e) => patchTheme({ accent: e.target.value })}
              />
            </div>
            <div>
              <Label className="text-xs">Fondo ext.</Label>
              <Input
                type="color"
                className="mt-1 h-8 p-1"
                value={t.backdrop === "transparent" ? "#000000" : t.backdrop}
                onChange={(e) => patchTheme({ backdrop: e.target.value })}
              />
            </div>
          </div>
          <div className="mt-2">
            <Label className="text-xs">Título</Label>
            <Input
              className="mt-1 h-8"
              value={t.title}
              onChange={(e) => patchTheme({ title: e.target.value })}
            />
          </div>
          <div className="mt-2 grid grid-cols-3 gap-2">
            <div>
              <Label className="text-xs">Fuente</Label>
              <Input
                type="number"
                className="mt-1 h-8"
                min={8}
                max={32}
                value={t.fontSize}
                onChange={(e) => patchTheme({ fontSize: Number(e.target.value) || 14 })}
              />
            </div>
            <div>
              <Label className="text-xs">Radio</Label>
              <Input
                type="number"
                className="mt-1 h-8"
                min={0}
                max={40}
                value={t.cornerRadius}
                onChange={(e) => patchTheme({ cornerRadius: Number(e.target.value) || 0 })}
              />
            </div>
            <div>
              <Label className="text-xs">Padding</Label>
              <Input
                type="number"
                className="mt-1 h-8"
                min={0}
                max={80}
                value={t.padding}
                onChange={(e) => patchTheme({ padding: Number(e.target.value) || 0 })}
              />
            </div>
          </div>
          <div className="mt-3 flex items-center gap-4">
            <label className="flex items-center gap-1.5 text-xs">
              <Switch
                checked={t.showTrafficLights}
                onCheckedChange={(v) => patchTheme({ showTrafficLights: v })}
              />
              Traffic lights
            </label>
            <label className="flex items-center gap-1.5 text-xs">
              <Switch
                checked={t.showShadow}
                onCheckedChange={(v) => patchTheme({ showShadow: v })}
              />
              Sombra
            </label>
          </div>
        </section>
      </aside>

      {/* Stage */}
      <main className="flex min-w-0 flex-1 flex-col">
        <div className="flex items-center gap-2 border-b px-4 py-2.5">
          <Label className="text-xs">Escala</Label>
          <Select
            value={String(prefs.scale)}
            onValueChange={(v) => patchPrefs({ scale: Number(v) })}
          >
            <SelectTrigger className="h-8 w-[72px]">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="2">2×</SelectItem>
              <SelectItem value="3">3×</SelectItem>
              <SelectItem value="4">4×</SelectItem>
            </SelectContent>
          </Select>
          <Button onClick={() => void exportPng()} disabled={exporting || !capture} className="ml-auto">
            {exporting ? (
              <Spinner data-icon="inline-start" />
            ) : (
              <Download data-icon="inline-start" />
            )}
            {exporting ? "Generando…" : "Exportar PNG"}
          </Button>
        </div>

        <div className="flex flex-1 items-start justify-center overflow-auto p-8 [background:repeating-conic-gradient(#1a1a24_0%_25%,#14141c_0%_50%)_0_0/24px_24px]">
          {capture ? (
            <div
              className="min-w-[420px] max-w-full overflow-hidden rounded-[var(--tc-radius)] font-mono leading-[1.2] shadow-[var(--tc-shadow)]"
              style={{
                ...cardVars(t),
                background: t.background,
                color: t.foreground,
                borderRadius: t.cornerRadius,
                fontSize: t.fontSize,
              }}
            >
              {t.showTrafficLights && (
                <div className="relative flex items-center px-[var(--tc-padding)] pt-3">
                  <div className="flex gap-2">
                    <span className="size-3 rounded-full bg-[#ff5f57]" />
                    <span className="size-3 rounded-full bg-[#febc2e]" />
                    <span className="size-3 rounded-full bg-[#28c840]" />
                  </div>
                  {t.title && (
                    <span
                      className="absolute left-1/2 -translate-x-1/2 text-[0.85em] opacity-65 whitespace-nowrap"
                      dangerouslySetInnerHTML={{ __html: esc(t.title) }}
                    />
                  )}
                </div>
              )}
              <div
                className="px-[var(--tc-padding)] pb-[var(--tc-padding)] pt-3 whitespace-pre"
                dangerouslySetInnerHTML={{ __html: body }}
              />
            </div>
          ) : (
            <Empty className="mt-16 border-none">
              <EmptyHeader>
                <EmptyMedia variant="icon">
                  <Camera />
                </EmptyMedia>
                <EmptyTitle>{ready ? "Sin captura todavía" : "Cargando…"}</EmptyTitle>
                <EmptyDescription>
                  {ready
                    ? "Ejecuta un comando para generar la tarjeta."
                    : "Recuperando preferencias guardadas."}
                </EmptyDescription>
              </EmptyHeader>
            </Empty>
          )}
        </div>

        <footer
          className={`min-h-9 border-t px-4 py-2 text-xs ${
            status.err ? "text-destructive" : "text-muted-foreground"
          }`}
        >
          {status.msg}
        </footer>
      </main>
    </div>
  );
}