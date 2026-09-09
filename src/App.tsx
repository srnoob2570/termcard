import { useCallback, useEffect, useRef, useState } from "react";
import { openUrl } from "@tauri-apps/plugin-opener";
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

// Primer's official Octicon `mark-github-16`: lucide-react no longer ships
// brand icons. Inlined (?raw) so `fill="currentColor"` inherits the link's
// color; as <img> the SVG can't see the parent color and renders black.
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
import { LANGS, translate, type Lang, type MessageKey } from "@/i18n";

const PRESET_LABELS: Record<string, MessageKey> = {
    "mac-dark": "presetMacDark",
    "mac-light": "presetMacLight",
    minimal: "presetMinimal",
    solarized: "presetSolarized",
};

export default function App() {
    const [prefs, setPrefs] = useState<Prefs>(DEFAULT_PREFS);
    const [capture, setCapture] = useState<Capture | null>(null);
    const [running, setRunning] = useState(false);
    const [svg, setSvg] = useState<string | null>(null);
    const [exporting, setExporting] = useState(false);
    const [status, setStatusState] = useState<{ msg: string; err: boolean }>({
        msg: translate(DEFAULT_PREFS.lang, "statusLoading"),
        err: false,
    });
    const setStatus = (msg: string, err = false) => setStatusState({ msg, err });
    const [ready, setReady] = useState(false);
    // UI translator: the language lives in prefs.lang (single source, no duplicated state).
    const msg = useCallback(
        (key: MessageKey, params?: Record<string, string | number>) =>
            translate(prefs.lang, key, params),
        [prefs.lang]
    );

    // Preview fonts: the SVG carries `font-family: 'JetBrains Mono'` only.
    // The @font-face payload (~1.4 MB base64) is installed ONCE at document
    // level; embedding it per render made every keystroke re-parse the blob.
    useEffect(() => {
        void api
            .fontCss()
            .then((css) => {
                const style = document.createElement("style");
                style.textContent = css;
                document.head.appendChild(style);
            })
            .catch(() => {
                // Without the face the preview falls back to a local
                // monospace font: cosmetic, not a failure.
            });
    }, []);

    // ---- init ----
    useEffect(() => {
        void (async () => {
            // L2: `msg` is captured from the first render (detected lang);
            // status messages here use the RESOLVED language instead.
            let lang = DEFAULT_PREFS.lang;
            try {
                const saved = await api.getPrefs();
                let next = DEFAULT_PREFS;
                if (saved && typeof saved === "object" && "command" in saved) {
                    next = {
                        ...DEFAULT_PREFS,
                        ...saved,
                        theme: { ...DEFAULT_PREFS.theme, ...saved.theme },
                    };
                    // The store isn't trusted: a corrupt lang would break the Select.
                    const storedLang = next.lang as string;
                    if (storedLang !== "es" && storedLang !== "en") {
                        next.lang = DEFAULT_PREFS.lang;
                    }
                    if (!next.rules.length) next.rules = await api.defaultRules();
                } else {
                    next = {
                        ...DEFAULT_PREFS,
                        cwd: await api.getHome(),
                        rules: await api.defaultRules(),
                    };
                }
                lang = next.lang;
                setPrefs(next);
            } catch {
                setStatus(translate(lang, "statusPrefsError"), true);
                // M5: without the store the rules list is empty, so previews
                // would render unredacted; fall back to the default rules.
                try {
                    const rules = await api.defaultRules();
                    setPrefs((p) => ({ ...p, rules }));
                } catch {
                    // No rules either: the warning above still shows.
                }
            } finally {
                setReady(true);
                setStatus(translate(lang, "statusReady"));
            }
        })();
    }, []);

    // index.html ships lang="es" hardcoded: fixed on mount and on language change.
    useEffect(() => {
        document.documentElement.lang = prefs.lang;
    }, [prefs.lang]);

    // The preview IS the SVG that gets exported: zero possible divergence.
    // Regenerated whenever the capture, theme, redaction rules or the
    // "show uncensored" toggle change (preview only; never on export).
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
        setPrefs((p) => ({ ...p, ...patch }));
    }, []);

    const patchTheme = useCallback((patch: Partial<Theme>) => {
        setPrefs((p) => ({ ...p, theme: { ...p.theme, ...patch } }));
    }, []);

    // Persistence (fire-and-forget) lives here, not inside the state updaters:
    // StrictMode invokes updaters twice in dev, duplicating every IPC. Identity
    // check against the last saved snapshot skips the initial mount AND the
    // StrictMode remount without spuriously saving the defaults.
    const savedPrefs = useRef<Prefs>(DEFAULT_PREFS);
    useEffect(() => {
        if (savedPrefs.current === prefs) return;
        savedPrefs.current = prefs;
        void api.savePrefs(prefs);
    }, [prefs]);

    // ---- capture ----
    const run = useCallback(async () => {
        if (running) return;
        if (!prefs.command.trim()) {
            setStatus(msg("statusEmptyCommand"), true);
            return;
        }
        setRunning(true);
        setStatus(msg("statusRunning"));
        // The "uncensored" toggle is temporary: only for the current preview.
        if (prefs.uncensored) patchPrefs({ uncensored: false });
        try {
            const res = await api.runCapture(prefs.command, prefs.cwd);
            setCapture(res.capture);
            const n = res.capture.lines.filter((l) => l.runs.length > 0).length;
            let done = msg("statusCaptureDone", { n });
            if (res.truncated) done += ` · ${msg("statusTruncated")}`;
            if (res.timedOut) done += ` · ${msg("statusTimedOut")}`;
            if (res.exitCode !== null && res.exitCode !== 0) {
                done += ` · ${msg("statusExitCode", { n: res.exitCode })}`;
            }
            setStatus(done);
        } catch (e) {
            setStatus(msg("statusError", { detail: String(e) }), true);
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
    // The render runs in spawn_blocking in Rust: the UI stays alive and this
    // feedback does get painted while the PNG is being generated.
    const exportPng = useCallback(async () => {
        if (!capture) {
            setStatus(msg("statusNoCapture"), true);
            return;
        }
        setExporting(true);
        setStatus(msg("statusGeneratingPng", { scale: prefs.scale }));
        try {
            const b64 = await api.exportPng(capture, prefs.theme, prefs.rules, prefs.scale);
            const saved = await api.savePng(b64, `termcard-${Date.now()}.png`);
            setStatus(saved ? msg("statusSavedTo", { path: saved }) : msg("statusExportCancelled"));
        } catch (e) {
            setStatus(msg("statusExportError", { detail: String(e) }), true);
        } finally {
            setExporting(false);
        }
    }, [capture, prefs]);

    // ---- rules ----
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
            {/* Left panel */}
            <aside className="flex w-90 shrink-0 flex-col overflow-y-auto border-r bg-sidebar p-4">
                <div className="mb-4 flex items-center gap-2">
                    <Terminal className="size-5 text-primary" />
                    <h1 className="text-base font-semibold tracking-tight">termcard</h1>
                    <Select
                        value={prefs.lang}
                        onValueChange={(v) => v && patchPrefs({ lang: v as Lang })}
                    >
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

                {/* Capture */}
                <section>
                    <FieldGroup>
                        <Field>
                            <FieldLabel className="text-xs uppercase text-muted-foreground tracking-wide">
                                {msg("sectionCapture")}
                            </FieldLabel>
                            <Textarea
                                id="command"
                                rows={3}
                                className="font-mono text-[13px]"
                                placeholder="ls --color=auto -la"
                                value={prefs.command}
                                onChange={(e) => patchPrefs({ command: e.target.value })}
                                autoCapitalize="off"
                                autoCorrect="off"
                                autoComplete="off"
                                spellCheck={false}
                                data-gramm="false"
                                data-enable-grammar="false"
                            />
                        </Field>
                        <Field>
                            <FieldLabel htmlFor="cwd" className="text-xs">
                                {msg("labelDirectory")}
                            </FieldLabel>
                            <Input
                                id="cwd"
                                className="h-8 font-mono text-xs"
                                placeholder="~/"
                                value={prefs.cwd}
                                onChange={(e) => patchPrefs({ cwd: e.target.value })}
                                autoCapitalize="off"
                                autoCorrect="off"
                                autoComplete="off"
                                spellCheck={false}
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
                            {running ? msg("statusRunning") : msg("actionRun")}
                        </Button>
                        {running && (
                            <Button variant="outline" onClick={() => void api.stopCapture()}>
                                <OctagonX data-icon="inline-start" />
                                {msg("actionStop")}
                            </Button>
                        )}
                    </div>
                </section>

                <Separator className="my-4" />

                {/* Redaction */}
                <section>
                    <div className="flex items-center justify-between">
                        <Label className="text-xs uppercase text-muted-foreground tracking-wide">
                            {msg("sectionRedaction")}
                        </Label>
                        <div className="flex gap-1">
                            <Button
                                size="icon-xs"
                                variant="ghost"
                                onClick={addRule}
                                title={msg("titleAddRule")}
                            >
                                <Plus />
                            </Button>
                            <Button
                                size="icon-xs"
                                variant="ghost"
                                onClick={() => void restoreRules()}
                                title={msg("titleRestoreRules")}
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
                                    placeholder={msg("placeholderReplacement")}
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
                                            ? msg("titleDefaultRuleNoDelete")
                                            : msg("titleDeleteRule")
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
                        {prefs.uncensored
                            ? msg("actionUncensoredHide")
                            : msg("actionUncensoredShow")}
                    </Button>
                </section>

                <Separator className="my-4" />

                {/* Theme */}
                <section>
                    <div className="flex items-center gap-1.5">
                        <Palette className="size-3.5 text-muted-foreground" />
                        <Label className="text-xs uppercase text-muted-foreground tracking-wide">
                            {msg("sectionTheme")}
                        </Label>
                    </div>
                    <div className="mt-2 grid grid-cols-2 gap-2">
                        <div>
                            <Label className="text-xs">Preset</Label>
                            <Select
                                value={t.preset}
                                onValueChange={(v) => {
                                    if (!v) return;
                                    // The preset brings the COMPLETE theme from Rust:
                                    // no leftovers from the previous preset ever remain.
                                    void api.presetTheme(v).then((full) => {
                                        patchTheme(full);
                                    });
                                }}
                            >
                                <SelectTrigger className="mt-1 h-8 w-full">
                                    <SelectValue />
                                </SelectTrigger>
                                <SelectContent>
                                    {Object.entries(PRESET_LABELS).map(([v, k]) => (
                                        <SelectItem key={v} value={v}>
                                            {msg(k)}
                                        </SelectItem>
                                    ))}
                                </SelectContent>
                            </Select>
                        </div>
                        <div>
                            <Label className="text-xs">{msg("labelPromptSymbol")}</Label>
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
                            <Label className="text-xs">{msg("labelAccent")}</Label>
                            <div className="mt-1 flex h-8 items-center">
                                <ColorField
                                    value={t.accent}
                                    onChange={(v) => patchTheme({ accent: v })}
                                    ariaLabel={msg("ariaAccent")}
                                    hexAriaLabel={msg("ariaColorHex")}
                                    transparentLabel={msg("colorTransparent")}
                                />
                                <span className="ml-2 font-mono text-xs text-muted-foreground">
                                    {t.accent}
                                </span>
                            </div>
                        </div>
                        <div>
                            <Label className="text-xs">{msg("labelBackdrop")}</Label>
                            <div className="mt-1 flex h-8 items-center">
                                <ColorField
                                    value={t.backdrop}
                                    onChange={(v) => patchTheme({ backdrop: v })}
                                    ariaLabel={msg("ariaBackdrop")}
                                    hexAriaLabel={msg("ariaColorHex")}
                                    transparentLabel={msg("colorTransparent")}
                                />
                                <span className="ml-2 font-mono text-xs text-muted-foreground">
                                    {t.backdrop === "transparent"
                                        ? msg("transparentText")
                                        : t.backdrop}
                                </span>
                            </div>
                        </div>
                    </div>
                    <div className="mt-2">
                        <Label className="text-xs">{msg("labelTitle")}</Label>
                        <Input
                            className="mt-1 h-8"
                            value={t.title}
                            onChange={(e) => patchTheme({ title: e.target.value })}
                        />
                    </div>
                    <div className="mt-2 grid grid-cols-4 gap-2">
                        <div>
                            <Label className="text-xs">{msg("labelFontSize")}</Label>
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
                            <Label className="text-xs">{msg("labelRadius")}</Label>
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
                            <Label className="text-xs">{msg("labelMargin")}</Label>
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
                            {msg("labelShadow")}
                        </label>
                    </div>
                </section>
                <div className="mt-auto flex items-center pt-3">
                    <a
                        href="https://github.com/srnoob2570/termcard"
                        target="_blank"
                        rel="noreferrer"
                        onClick={(e) => {
                            // target="_blank" is unreliable inside the webview.
                            e.preventDefault();
                            void openUrl("https://github.com/srnoob2570/termcard");
                        }}
                        title={msg("githubTitle")}
                        className="inline-flex h-6 items-center gap-1.5 rounded-4xl border border-border bg-secondary px-2 text-[10px] font-medium text-foreground transition-colors hover:bg-muted"
                    >
                        <GithubIcon />
                        <span className="font-mono">v0.1</span>
                    </a>
                </div>
            </aside>

            <div className="flex min-w-0 flex-1 flex-col">
                <div className="flex items-center gap-2 border-b px-4 py-2.5">
                    <Label className="text-xs">{msg("labelScale")}</Label>
                    <Select
                        value={String(prefs.scale)}
                        onValueChange={(v) => patchPrefs({ scale: Number(v) })}
                    >
                        <SelectTrigger className="h-8 w-18">
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
                        {exporting ? msg("statusGenerating") : msg("actionExportPng")}
                    </Button>
                </div>

                <div className="flex flex-1 items-start justify-center overflow-auto p-8 [background:repeating-conic-gradient(#1a1a24_0%_25%,#14141c_0%_50%)_0_0/24px_24px]">
                    {svg ? (
                        <div
                            className="max-w-full [&>svg]:h-auto [&>svg]:max-w-full"
                            dangerouslySetInnerHTML={{ __html: svg }}
                        />
                    ) : (
                        <Empty className="mt-16 border-none">
                            <EmptyHeader>
                                <EmptyMedia variant="icon">
                                    <Camera />
                                </EmptyMedia>
                                <EmptyTitle>
                                    {ready ? msg("emptyNoCapture") : msg("statusLoading")}
                                </EmptyTitle>
                                <EmptyDescription>
                                    {ready ? msg("emptyRunCommand") : msg("emptyLoadingPrefs")}
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
            </div>
        </div>
    );
}
