import type { Capture, Prefs, Theme } from "@/api";

/** Escapa HTML para insertar texto de runs de forma segura. */
export function esc(s: string): string {
  return s
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;");
}

/**
 * Renderiza la captura a nodos React-estilo: devuelve HTML string que el
 * card de preview inyecta. Mismo contrato que el exportador SVG: prompt
 * sintético `❯ comando` en color de acento.
 */
export function renderCaptureHtml(capture: Capture, theme: Theme): string {
  const lines: string[] = [];

  lines.push(
    `<div class="text-[color:var(--tc-accent)] font-bold"><span>${esc(theme.promptSymbol)}</span> ${esc(capture.commandLine)}</div>`,
  );

  for (const line of capture.lines) {
    if (line.runs.length === 0) {
      lines.push("<div>&nbsp;</div>");
      continue;
    }
    const runs = line.runs
      .map((run) => {
        const style = runStyle(run, theme);
        return `<span style="${style}">${esc(run.text)}</span>`;
      })
      .join("");
    lines.push(`<div>${runs || "&nbsp;"}</div>`);
  }

  return lines.join("");
}

type Run = Capture["lines"][0]["runs"][0];

function runStyle(run: Run, theme: Theme): string {
  const parts: string[] = [];
  if (run.bold) parts.push("font-weight:700");
  if (run.italic) parts.push("font-style:italic");
  if (run.underline) parts.push("text-decoration:underline");
  const bg = colorCss(run.bg, theme, true);
  if (bg) parts.push(`background:${bg}`);
  const fg = colorCss(run.fg, theme, false);
  if (fg) parts.push(`color:${fg}`);
  return parts.join(";");
}

export function colorCss(
  color: Run["fg"],
  theme: Theme,
  isBg: boolean,
): string | null {
  if (!color || color === "default") return isBg ? null : theme.foreground;
  if (color === "defaultInverted") return theme.background;
  if ("indexed" in color) return `var(--ansi-${color.indexed})`;
  if ("rgb" in color) {
    const [r, g, b] = color.rgb;
    return `rgb(${r},${g},${b})`;
  }
  return null;
}

export function defaultPrefs(cwd: string, rules: Prefs["rules"]): Prefs {
  return {
    command: "",
    cwd,
    rules,
    theme: defaultTheme(),
    scale: 2,
    uncensored: false,
  };
}

export function defaultTheme(): Theme {
  return {
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
  };
}