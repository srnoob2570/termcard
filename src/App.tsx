import { useCallback, useEffect, useState } from "react";
import {
    Camera,
    Download,
    Eye,
    EyeOff,
    Languages,
    OctagonX,
    Palette,
    Plus,
    RotateCcw,
    Terminal,
    Trash2,
} from "lucide-react";
import githubMark from "@/assets/github-mark.svg?raw";

// Octicon `mark-github-16` oficial de Primer: lucide-react ya no trae iconos de
// marca. Va inline (?raw) para que `fill="currentColor"` herede el color del
// enlace; como <img> el SVG no ve el color del padre y se pinta negro.
function GithubIcon() {
    return (
        <span
            className="size-4 [&>svg]:size-full"
            dangerouslySetInnerHTML={{ __html: githubMark }}
            aria-hidden
        />
    );
}

import { Button } from "@/components/ui/button";
import { ColorField } from "@/components/color-field";
import { Checkbox } from "@/components/ui/checkbox";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Field, FieldGroup, FieldLabel } from "@/components/ui/field";
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
import { DEFAULT_PREFS, api, type Capture, type Prefs, type RedactRule, type Theme } from "@/api";

/** Prefs iniciales para un primer arranque sin store previo. */
function defaultPrefs(cwd: string, rules: Prefs["rules"]): Prefs {
    return { ...DEFAULT_PREFS, cwd, rules };
}

/** Etiquetas de idioma para el selector WIP (solo UI, aún sin i18n real). */
const LANGS = [
    { id: "es", label: "Español" },
    { id: "en", label: "English" },
];

const PRESET_LABELS: Record<string, string> = {
    "mac-dark": "Mac oscuro",
    "mac-light": "Mac claro",
    minimal: "Minimal",
    solarized: "Solarized",
};

export default function App() {
    const [prefs, setPrefs] = useState<Prefs>(DEFAULT_PREFS);
    const [capture, setCapture] = useState<Capture | null>(null);
    const [running, setRunning] = useState(false);
    const [svg, setSvg] = useState<string | null>(null);
    const [exporting, setExporting] = useState(false);
    const [status, setStatusState] = useState<{ msg: string; err: boolean }>({
        msg: "Cargando…",
        err: false,
    });
    const setStatus = (msg: string, err = false) => setStatusState({ msg, err });
    const [ready, setReady] = useState(false);
    // Idioma de la UI (WIP): de momento solo guarda el estado, sin i18n real.
    const [lang, setLang] = useState("es");

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

    // El preview ES el SVG que se exporta: cero divergencia posible.
    // Se regenera al cambiar la captura, el tema, las reglas de redacción o
    // el toggle "mostrar sin censura" (solo preview; el export nunca).
    useEffect(() => {
        if (!capture) {
            setSvg(null);
            return;
        }
        let alive = true;
        void api
            .exportSvg(capture, prefs.theme, prefs.rules, prefs.uncensored)
            .then((s) => alive && setSvg(s))
            .catch(() => alive && setSvg(null));
        return () => {
            alive = false;
        };
    }, [capture, prefs.theme, prefs.rules, prefs.uncensored]);

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
            const cap = await api.runCapture(prefs.command, prefs.cwd);
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
    // El render corre en spawn_blocking en Rust: la UI sigue viva y este
    // feedback sí llega a pintarse mientras se genera el PNG.
    const exportPng = useCallback(async () => {
        if (!capture) {
            setStatus("No hay captura para exportar.", true);
            return;
        }
        setExporting(true);
        setStatus(`Generando PNG (${prefs.scale}×)…`);
        try {
            const b64 = await api.exportPng(capture, prefs.theme, prefs.rules, prefs.scale);
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
            rules: [
                ...prefs.rules,
                { pattern: "", replacement: "", enabled: true, isDefault: false },
            ],
        });
    };

    const restoreRules = async () => {
        patchPrefs({ rules: await api.defaultRules() });
    };

    const t = prefs.theme;

    return (
        <div className="flex h-screen overflow-hidden">
            {/* Panel izquierdo */}
            <aside className="flex w-[360px] shrink-0 flex-col overflow-y-auto border-r bg-sidebar p-4">
                <div className="mb-4 flex items-center gap-2">
                    <Terminal className="size-5 text-primary" />
                    <h1 className="text-base font-semibold tracking-tight">termcard</h1>
                    {/* Selector de idiomas: WIP, de momento solo cambia el estado. */}
                    <Select value={lang} onValueChange={(v) => v && setLang(v)}>
                        <SelectTrigger
                            size="sm"
                            className="ml-auto gap-1 border-none bg-transparent shadow-none hover:bg-muted"
                        >
                            <Languages className="size-3.5 text-muted-foreground" />
                            <SelectValue />
                        </SelectTrigger>
                        <SelectContent>
                            {LANGS.map((l) => (
                                <SelectItem key={l.id} value={l.id}>
                                    {l.label}
                                </SelectItem>
                            ))}
                        </SelectContent>
                    </Select>
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
                        <Button
                            onClick={() => void run()}
                            disabled={running || exporting}
                            className="flex-1"
                        >
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
                            <Button
                                size="icon-xs"
                                variant="ghost"
                                onClick={addRule}
                                title="Añadir regla"
                            >
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
                                    title={
                                        rule.isDefault
                                            ? "Las reglas por defecto se desactivan, no borran"
                                            : "Eliminar"
                                    }
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
                        {prefs.uncensored ? (
                            <EyeOff data-icon="inline-start" />
                        ) : (
                            <Eye data-icon="inline-start" />
                        )}
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
                                onValueChange={(v) => {
                                    if (!v) return;
                                    // El preset trae el tema COMPLETO desde Rust:
                                    // nunca quedan restos del preset anterior.
                                    void api.presetTheme(v).then((full) => {
                                        patchTheme(full);
                                    });
                                }}
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
                            <div className="mt-1 flex h-8 items-center">
                                <ColorField
                                    value={t.accent}
                                    onChange={(v) => patchTheme({ accent: v })}
                                    ariaLabel="Color de acento"
                                />
                                <span className="ml-2 font-mono text-xs text-muted-foreground">
                                    {t.accent}
                                </span>
                            </div>
                        </div>
                        <div>
                            <Label className="text-xs">Fondo ext.</Label>
                            <div className="mt-1 flex h-8 items-center">
                                <ColorField
                                    value={t.backdrop}
                                    onChange={(v) => patchTheme({ backdrop: v })}
                                    ariaLabel="Fondo exterior de la tarjeta"
                                />
                                <span className="ml-2 font-mono text-xs text-muted-foreground">
                                    {t.backdrop === "transparent" ? "transparente" : t.backdrop}
                                </span>
                            </div>
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
                    <div className="mt-2 grid grid-cols-4 gap-2">
                        <div>
                            <Label className="text-xs">Fuente</Label>
                            <Input
                                type="number"
                                className="mt-1 h-8"
                                min={8}
                                max={32}
                                value={t.fontSize}
                                onChange={(e) =>
                                    patchTheme({ fontSize: Number(e.target.value) || 14 })
                                }
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
                                onChange={(e) =>
                                    patchTheme({ cornerRadius: Number(e.target.value) || 0 })
                                }
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
                                onChange={(e) =>
                                    patchTheme({ padding: Number(e.target.value) || 0 })
                                }
                            />
                        </div>
                        <div>
                            <Label className="text-xs">Margen</Label>
                            <Input
                                type="number"
                                className="mt-1 h-8"
                                min={0}
                                max={80}
                                value={t.outerMargin}
                                onChange={(e) =>
                                    patchTheme({ outerMargin: Number(e.target.value) || 0 })
                                }
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
                <div className="mt-auto flex items-center pt-3">
                    <a
                        href="https://github.com/srnoob2570/termcard"
                        target="_blank"
                        rel="noreferrer"
                        title="termcard v0.1 — Repositorio en GitHub"
                        className="inline-flex h-6 items-center gap-1.5 rounded-4xl border border-border bg-secondary px-2 text-[10px] font-medium text-foreground transition-colors hover:bg-muted"
                    >
                        <GithubIcon />
                        <span className="font-mono">v0.1</span>
                    </a>
                </div>
            </aside>

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
                    <Button
                        onClick={() => void exportPng()}
                        disabled={exporting || !capture}
                        className="ml-auto"
                    >
                        {exporting ? (
                            <Spinner data-icon="inline-start" />
                        ) : (
                            <Download data-icon="inline-start" />
                        )}
                        {exporting ? "Generando…" : "Exportar PNG"}
                    </Button>
                </div>

                <div className="flex flex-1 items-start justify-center overflow-auto p-8 [background:repeating-conic-gradient(#1a1a24_0%_25%,#14141c_0%_50%)_0_0/24px_24px]">
                    {svg ? (
                        <div
                            className="max-w-full [&>svg]:h-auto [&>svg]:max-w-full [&>svg]:drop-shadow-2xl"
                            dangerouslySetInnerHTML={{ __html: svg }}
                        />
                    ) : (
                        <Empty className="mt-16 border-none">
                            <EmptyHeader>
                                <EmptyMedia variant="icon">
                                    <Camera />
                                </EmptyMedia>
                                <EmptyTitle>
                                    {ready ? "Sin captura todavía" : "Cargando…"}
                                </EmptyTitle>
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
