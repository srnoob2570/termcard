import type { Capture, Color, Run } from "@/api";

/**
 * Manual mode: builds a `Capture` from text the user typed or pasted
 * instead of a PTY run. The result feeds the same export pipeline
 * (`export_svg`/`export_png`), so redaction, theming and rendering need
 * no changes. Output starting with a tag is parsed as HTML; anything
 * else is plain text (one default-styled run per line).
 */

type Style = Pick<Run, "fg" | "bg" | "bold" | "italic" | "underline">;

const DEFAULT_STYLE: Style = {
    fg: null,
    bg: null,
    bold: false,
    italic: false,
    underline: false,
};

const BLOCK_TAGS = new Set([
    "ADDRESS",
    "ARTICLE",
    "ASIDE",
    "BLOCKQUOTE",
    "DD",
    "DETAILS",
    "DIV",
    "DL",
    "DT",
    "FIGCAPTION",
    "FIGURE",
    "FOOTER",
    "H1",
    "H2",
    "H3",
    "H4",
    "H5",
    "H6",
    "HEADER",
    "LI",
    "MAIN",
    "NAV",
    "OL",
    "P",
    "PRE",
    "SECTION",
    "TABLE",
    "TD",
    "TH",
    "TR",
    "UL",
]);

export function buildManualCapture(command: string, output: string): Capture {
    return output.trimStart().startsWith("<")
        ? fromHtml(command, output)
        : fromText(command, output);
}

function fromText(command: string, text: string): Capture {
    const lines = text
        .replace(/\t/g, "    ")
        .replace(/\r\n?/g, "\n")
        .split("\n")
        .map((line) => (line ? [run(line, DEFAULT_STYLE)] : []));
    return makeCapture(command, lines);
}

function fromHtml(command: string, html: string): Capture {
    const doc = new DOMParser().parseFromString(html, "text/html");
    const lines: Run[][] = [];
    let cur: Run[] = [];
    let inPre = false;

    const walk = (node: Node, style: Style) => {
        if (node.nodeType === Node.TEXT_NODE) {
            const raw = node.nodeValue ?? "";
            if (inPre) {
                // Inside <pre> newlines are real line breaks; tabs become
                // spaces like the plain-text path.
                const parts = raw.replace(/\t/g, "    ").split("\n");
                parts.forEach((seg, i) => {
                    if (i > 0) {
                        lines.push(cur);
                        cur = [];
                    }
                    if (seg) cur.push(run(seg, style));
                });
                return;
            }
            const text = raw.replace(/\s+/g, " ");
            const trimmed = cur.length ? text : text.trimStart();
            if (trimmed) cur.push(run(trimmed, style));
            return;
        }
        if (node.nodeType !== Node.ELEMENT_NODE) return;
        const el = node as Element;
        if (el.tagName === "BR") {
            lines.push(cur);
            cur = [];
            return;
        }
        const block = BLOCK_TAGS.has(el.tagName);
        if (block && cur.length) {
            lines.push(cur);
            cur = [];
        }
        const wasPre = inPre;
        if (el.tagName === "PRE") inPre = true;
        const next = mergeStyle(style, el);
        for (const child of Array.from(el.childNodes)) walk(child, next);
        inPre = wasPre;
        if (block) {
            lines.push(cur);
            cur = [];
        }
    };

    walk(doc.body, DEFAULT_STYLE);
    if (cur.length) lines.push(cur);
    return makeCapture(command, lines);
}

function makeCapture(command: string, lines: Run[][]): Capture {
    // Mimics Rust `Capture::trimmed`: drop trailing empty lines and fit
    // `cols` to the widest content. ponytail: width counts code points,
    // so wide CJK chars under-count columns (2 cols each in real cells).
    while (lines.length && lines[lines.length - 1].length === 0) lines.pop();
    const cpWidth = (runs: Run[]) => runs.reduce((n, r) => n + [...r.text].length, 0);
    const widest = lines.reduce(
        (m, runs) => Math.max(m, cpWidth(runs)),
        Math.max([...command].length, 1)
    );
    return {
        cols: Math.min(65535, widest + 1),
        rows: Math.min(65535, lines.length),
        commandLine: command,
        lines: lines.map((runs) => ({ runs })),
    };
}

function run(text: string, style: Style): Run {
    return { text, ...style };
}

function mergeStyle(style: Style, el: Element): Style {
    const s: Style = { ...style };
    const tag = el.tagName;
    if (tag === "B" || tag === "STRONG") s.bold = true;
    if (tag === "I" || tag === "EM") s.italic = true;
    if (tag === "U") s.underline = true;

    const st = el.getAttribute("style");
    if (!st) return s;
    for (const decl of st.split(";")) {
        const i = decl.indexOf(":");
        if (i < 0) continue;
        const prop = decl.slice(0, i).trim().toLowerCase();
        const val = decl
            .slice(i + 1)
            .trim()
            .toLowerCase();
        if (prop === "color") {
            const c = cssColor(val);
            if (c) s.fg = c;
        } else if (prop === "background-color") {
            const c = cssColor(val);
            if (c) s.bg = c;
        } else if (prop === "font-weight") {
            const n = Number.parseInt(val, 10);
            s.bold ||= val === "bold" || (Number.isNaN(n) === false && n >= 600);
        } else if (prop === "font-style") {
            s.italic ||= val === "italic";
        } else if (prop === "text-decoration" || prop === "text-decoration-line") {
            s.underline ||= val.includes("underline");
        }
    }
    return s;
}

/** #hex (3/4/6/8 digits) and rgb()/rgba() with numeric channels. */
function cssColor(value: string): Color | null {
    const v = value.trim().toLowerCase();
    if (v.startsWith("#")) {
        const hex = v.slice(1);
        if (hex.length === 3 || hex.length === 4) {
            return {
                rgb: [
                    Number.parseInt(hex[0] + hex[0], 16),
                    Number.parseInt(hex[1] + hex[1], 16),
                    Number.parseInt(hex[2] + hex[2], 16),
                ],
            };
        }
        if (hex.length === 6 || hex.length === 8) {
            return {
                rgb: [
                    Number.parseInt(hex.slice(0, 2), 16),
                    Number.parseInt(hex.slice(2, 4), 16),
                    Number.parseInt(hex.slice(4, 6), 16),
                ],
            };
        }
        return null;
    }
    const m = /^rgba?\(\s*(\d+)\s*[, ]\s*(\d+)\s*[, ]\s*(\d+)/.exec(v);
    return m ? { rgb: [Number(m[1]), Number(m[2]), Number(m[3])] } : null;
}
