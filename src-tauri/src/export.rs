use crate::ir::{self, Capture, Color};
use crate::theme::{Palette, Theme};
use base64::Engine as _;
use std::sync::LazyLock;

/// JetBrains Mono metrics: advance width = 0.6 × font size,
/// recommended line height = 1.2 × font size.
const CH_WIDTH: f32 = 0.6;
const LINE_HEIGHT: f32 = 1.2;

static FONTS: LazyLock<Fonts> = LazyLock::new(Fonts::load);

struct Fonts {
    regular: &'static [u8],
    bold: &'static [u8],
    italic: &'static [u8],
    bold_italic: &'static [u8],
}

impl Fonts {
    fn load() -> Self {
        Fonts {
            regular: include_bytes!("../fonts/JetBrainsMono-Regular.ttf"),
            bold: include_bytes!("../fonts/JetBrainsMono-Bold.ttf"),
            italic: include_bytes!("../fonts/JetBrainsMono-Italic.ttf"),
            bold_italic: include_bytes!("../fonts/JetBrainsMono-BoldItalic.ttf"),
        }
    }
}

pub fn font_family() -> &'static str {
    "JetBrains Mono"
}

/// Logical dimensions of the card (units of the base SVG).
pub struct Layout {
    pub width: f32,
    pub height: f32,
}

pub fn layout(capture: &Capture, theme: &Theme) -> Layout {
    let line_h = theme.font_size as f32 * LINE_HEIGHT;
    // Visible lines: prompt + output. Captures arrive already trimmed
    // (`Capture::trimmed` at capture time), so `lines` has no trailing empties.
    let visible = capture.lines.len() + 1; // + prompt line
    let chrome = if theme.show_traffic_lights {
        44.0
    } else {
        16.0
    };
    let pad = theme.padding as f32;
    // Gap reserved around the window so the shadow can breathe.
    let shadow_gap = if theme.show_shadow { 8.0 } else { 0.0 };
    let frame = theme.outer_margin as f32 * 2.0 + shadow_gap * 2.0; // per side
                                                                    // Automatic width: longest line (prompt included) + padding.
    let prompt_len = ir::str_width(&theme.prompt_symbol) + 1 + ir::str_width(&capture.command_line);
    let longest = capture
        .lines
        .iter()
        .map(|l| l.runs.iter().map(|r| ir::str_width(&r.text)).sum::<usize>())
        .chain(std::iter::once(prompt_len))
        .max()
        .unwrap_or(20) as f32;
    let width = (pad + longest * theme.font_size as f32 * CH_WIDTH + pad + frame).max(420.0);
    let height = (pad + chrome + line_h * visible as f32 + pad + frame).max(160.0);
    Layout { width, height }
}

/// Renders the full SVG of the card. `scale` multiplies dimensions.
pub fn render_svg(capture: &Capture, theme: &Theme, palette: &Palette, scale: u32) -> String {
    render_svg_inner(capture, theme, palette, scale, true)
}

/// `embed_fonts` toggles the `@font-face` CSS: it is only needed by browsers
/// (the preview). resvg resolves fonts through `fontdb` (see `render_png`),
/// so the PNG path skips ~1.5 MB of dead base64 CSS.
fn render_svg_inner(
    capture: &Capture,
    theme: &Theme,
    palette: &Palette,
    scale: u32,
    embed_fonts: bool,
) -> String {
    let layout = layout(capture, theme);
    let w = layout.width * scale as f32;
    let h = layout.height * scale as f32;
    let fs = theme.font_size as f32 * scale as f32;
    let line_h = fs * LINE_HEIGHT;
    let pad = theme.padding as f32 * scale as f32;
    let margin = theme.outer_margin as f32 * scale as f32;
    let chrome = if theme.show_traffic_lights {
        44.0 * scale as f32
    } else {
        16.0 * scale as f32
    };
    let radius = theme.corner_radius as f32 * scale as f32;
    // Window origin: outer margin + shadow gap.
    let win_inset = margin
        + if theme.show_shadow {
            8.0 * scale as f32
        } else {
            0.0
        };

    let mut svg = String::with_capacity(64 * 1024);
    svg.push_str(&format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{w}" height="{h}" viewBox="0 0 {w} {h}">"#
    ));
    if embed_fonts {
        svg.push_str(&font_defs());
    }
    svg.push_str(&format!(
        r#"<defs><clipPath id="win"><rect x="{xi}" y="{yi}" width="{ww}" height="{wh}" rx="{radius}"/></clipPath></defs>"#,
        xi = win_inset,
        yi = win_inset,
        ww = w - win_inset * 2.0,
        wh = h - win_inset * 2.0,
    ));

    // Outer background (backdrop). The theme stores the fill as a string: "transparent",
    // "#rrggbb" or "rgba(r,g,b,a)". resvg and Chromium resolve all three natively
    // (verified: transparent → alpha 0; rgba → alpha preserved), so nothing
    // needs translating: an unpainted rect would be equivalent to alpha 0.
    svg.push_str(&format!(
        r#"<rect width="{w}" height="{h}" fill="{}"/>"#,
        escape_xml(&theme.backdrop)
    ));

    // Shadow: grows 8px around the window, inside the margin.
    if theme.show_shadow {
        svg.push_str(&format!(
            r#"<rect x="{sx}" y="{sy}" width="{sw}" height="{sh}" rx="{radius}" fill="black" opacity="0.35" filter="url(#blur)"/>"#,
            sx = win_inset - 4.0 * scale as f32,
            sy = win_inset + 4.0 * scale as f32,
            sw = w - win_inset * 2.0 + 8.0 * scale as f32,
            sh = h - win_inset * 2.0 + 8.0 * scale as f32,
        ));
        svg.push_str(&format!(
            r#"<defs><filter id="blur" x="-10%" y="-10%" width="120%" height="120%"><feGaussianBlur stdDeviation="{blur}"/></filter></defs>"#,
            blur = 6.0 * f64::from(scale),
        ));
    }

    svg.push_str(&format!(
        r#"<g clip-path="url(#win)"><rect x="{xi}" y="{yi}" width="{ww}" height="{wh}" rx="{radius}" fill="{bg}"/>"#,
        xi = win_inset,
        yi = win_inset,
        ww = w - win_inset * 2.0,
        wh = h - win_inset * 2.0,
        bg = escape_xml(&theme.background),
    ));

    // Title bar + traffic lights.
    if theme.show_traffic_lights {
        let cy = win_inset + 20.0 * scale as f32;
        let lx = win_inset + pad;
        for (i, color) in ["#ff5f57", "#febc2e", "#28c840"].iter().enumerate() {
            let cx = lx + (16.0 * scale as f32) * i as f32;
            svg.push_str(&format!(
                r#"<circle cx="{cx}" cy="{cy}" r="{}" fill="{color}"/>"#,
                6.0 * scale as f32
            ));
        }
        if !theme.title.is_empty() {
            let ty = cy + fs * 0.35;
            let tx = w / 2.0;
            svg.push_str(&text(
                tx,
                ty,
                &escape_xml(&theme.title),
                &theme.foreground,
                fs * 0.85,
                &TextStyle {
                    bold: false,
                    italic: false,
                    anchor: "middle",
                },
            ));
        }
    }

    // Prompt line.
    let mut y = win_inset + chrome + line_h * 0.8;
    let prompt = format!("{} {}", theme.prompt_symbol, capture.command_line);
    svg.push_str(&text(
        win_inset + pad,
        y,
        &escape_xml(&prompt),
        &theme.accent,
        fs,
        &TextStyle {
            bold: true,
            italic: false,
            anchor: "start",
        },
    ));
    y += line_h;

    // Body: runs of the capture.
    for line in &capture.lines {
        let mut x = win_inset + pad;
        for run in &line.runs {
            let w_run = ir::str_width(&run.text) as f32 * fs * CH_WIDTH;
            if let Some(bg) = run.bg.as_ref() {
                let fill = resolve_color(bg, theme, palette, &theme.foreground);
                svg.push_str(&format!(
                    r#"<rect x="{x}" y="{ybg}" width="{w_run}" height="{line_h}" fill="{fill}"/>"#,
                    ybg = y - fs * 0.8,
                    fill = escape_xml(&fill),
                ));
            }
            let fill = run
                .fg
                .as_ref()
                .map(|c| resolve_color(c, theme, palette, &theme.foreground))
                .unwrap_or_else(|| theme.foreground.clone());
            svg.push_str(&text(
                x,
                y,
                &escape_xml(&run.text),
                &fill,
                fs,
                &TextStyle {
                    bold: run.bold,
                    italic: run.italic,
                    anchor: "start",
                },
            ));
            if run.underline {
                svg.push_str(&format!(
                    r#"<line x1="{x}" y1="{yu}" x2="{}" y2="{yu}" stroke="{stroke}" stroke-width="{}"/>"#,
                    x + w_run,
                    fs * 0.06,
                    yu = y + fs * 0.15,
                    stroke = escape_xml(&fill),
                ));
            }
            x += w_run;
        }
        y += line_h;
    }

    svg.push_str("</g></svg>");
    svg
}

fn text(x: f32, y: f32, content: &str, fill: &str, font_px: f32, style: &TextStyle) -> String {
    let family = font_family();
    format!(
        r#"<text xml:space="preserve" x="{x}" y="{y}" font-family="{family}" font-weight="{weight}" font-style="{style}" font-size="{font_px}" fill="{fill}" text-anchor="{anchor}">{content}</text>"#,
        weight = if style.bold { "bold" } else { "normal" },
        style = if style.italic { "italic" } else { "normal" },
        anchor = style.anchor,
        fill = escape_xml(fill),
    )
}

struct TextStyle {
    bold: bool,
    italic: bool,
    anchor: &'static str,
}

fn resolve_color(color: &Color, theme: &Theme, palette: &Palette, default_fg: &str) -> String {
    match color {
        Color::Indexed(i) => palette.resolve(*i),
        Color::Rgb(r, g, b) => format!("#{r:02x}{g:02x}{b:02x}"),
        Color::Default => default_fg.to_string(),
        Color::DefaultInverted => theme.background.clone(),
    }
}

/// Base64 of the 4 embedded faces, encoded once (they are static includes).
static FONT_B64: LazyLock<[String; 4]> = LazyLock::new(|| {
    let f = &*FONTS;
    let engine = base64::engine::general_purpose::STANDARD;
    [
        engine.encode(f.regular),
        engine.encode(f.bold),
        engine.encode(f.italic),
        engine.encode(f.bold_italic),
    ]
});

fn font_defs() -> String {
    let face = |b64: &str, weight: &str, style: &str| {
        format!(
            r#"@font-face {{ font-family: '{}'; font-weight: {weight}; font-style: {style}; src: url(data:font/ttf;base64,{b64}) format('truetype'); }}"#,
            font_family(),
        )
    };
    let [regular, bold, italic, bold_italic] = &*FONT_B64;
    format!(
        r#"<defs><style>{} {} {} {}</style></defs>"#,
        face(regular, "normal", "normal"),
        face(bold, "bold", "normal"),
        face(italic, "normal", "italic"),
        face(bold_italic, "bold", "italic"),
    )
}

/// Escapes text and quoted-attribute values (`"`/`'` included, so a theme
/// string can never break out of `fill="..."`).
fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

/// Rasterizes the capture to PNG with resvg. `scale` is 2, 3 or 4.
pub fn render_png(
    capture: &Capture,
    theme: &Theme,
    palette: &Palette,
    scale: u32,
) -> Result<Vec<u8>, String> {
    // Fonts come from `fontdb` below; the @font-face CSS is browser-only.
    let svg = render_svg_inner(capture, theme, palette, scale, false);

    let mut fontdb = fontdb::Database::new();
    let f = &*FONTS;
    for data in [f.regular, f.bold, f.italic, f.bold_italic] {
        fontdb.load_font_data(data.to_vec());
    }

    let opt = usvg::Options {
        font_family: font_family().to_string(),
        fontdb: std::sync::Arc::new(fontdb),
        ..Default::default()
    };
    let tree = usvg::Tree::from_str(&svg, &opt).map_err(|e| format!("invalid SVG: {e}"))?;

    let size = tree.size();
    let mut pixmap =
        tiny_skia::Pixmap::new(size.width().round() as u32, size.height().round() as u32)
            .ok_or("invalid pixmap dimensions")?;

    resvg::render(
        &tree,
        tiny_skia::Transform::identity(),
        &mut pixmap.as_mut(),
    );
    pixmap
        .encode_png()
        .map_err(|e| format!("failed to encode PNG: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::{Line, Run};

    fn sample() -> (Capture, Theme, Palette) {
        let cap = Capture {
            cols: 20,
            rows: 3,
            command_line: "ls".into(),
            lines: vec![
                Line::from_runs(vec![Run {
                    text: "src".into(),
                    fg: Some(Color::Indexed(1)),
                    bg: None,
                    bold: true,
                    italic: false,
                    underline: false,
                }]),
                Line::empty(),
            ],
        };
        (cap, Theme::default(), Palette::default())
    }

    #[test]
    fn svg_contains_prompt_and_text() {
        let (cap, theme, palette) = sample();
        let svg = render_svg(&cap, &theme, &palette, 1);
        assert!(svg.contains("ls"));
        assert!(svg.contains("src"));
        assert!(svg.contains("#ff5f57")); // traffic light
    }

    #[test]
    fn svg_escapes_xml() {
        let (mut cap, mut theme, palette) = sample();
        cap.command_line = "echo \"<b>&\">".into();
        // A hostile theme string must not break out of a quoted attribute.
        theme.background = "#1e1e2e\" onload=\"pwn".into();
        let svg = render_svg(&cap, &theme, &palette, 1);
        assert!(svg.contains("&lt;b&gt;&amp;"));
        assert!(!svg.contains("<b>"));
        assert!(svg.contains("&quot;"));
        assert!(!svg.contains("fill=\"#1e1e2e\" onload"));
    }

    #[test]
    fn png_scales_within_rounding() {
        let (cap, theme, palette) = sample();
        let png1 = render_png(&cap, &theme, &palette, 1).unwrap();
        let png2 = render_png(&cap, &theme, &palette, 2).unwrap();
        let png3 = render_png(&cap, &theme, &palette, 3).unwrap();
        let (w1, h1) = png_dims(&png1);
        let (w2, h2) = png_dims(&png2);
        let (w3, h3) = png_dims(&png3);
        // The window width is float; 2x/3x may differ ±1 px from rounding.
        assert!(((w2 as i64) - (w1 as i64) * 2).abs() <= 1);
        assert!(((h2 as i64) - (h1 as i64) * 2).abs() <= 1);
        assert!(((w3 as i64) - (w1 as i64) * 3).abs() <= 1);
        assert!(((h3 as i64) - (h1 as i64) * 3).abs() <= 1);
    }

    /// Reads (width, height) from a PNG's IHDR.
    fn png_dims(png: &[u8]) -> (u32, u32) {
        let be = |b: &[u8]| u32::from_be_bytes([b[0], b[1], b[2], b[3]]);
        (be(&png[16..20]), be(&png[20..24]))
    }
}
