use crate::ir::{Capture, Color, Line};
use crate::theme::{Palette, Theme};
use base64::Engine as _;
use std::sync::LazyLock;

/// Métricas de JetBrains Mono: ancho de avance = 0.6 × tamaño de fuente,
/// alto de línea recomendado = 1.2 × tamaño de fuente.
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

/// Dimensiones lógicas de la tarjeta (unidades del SVG base).
pub struct Layout {
    pub width: f32,
    pub height: f32,
}

pub fn layout(capture: &Capture, theme: &Theme) -> Layout {
    let line_h = theme.font_size as f32 * LINE_HEIGHT;
    // Líneas visibles: prompt + salida (se recortan filas vacías finales).
    let visible = capture.visible_lines().count() + 1; // + línea de prompt
    let chrome = if theme.show_traffic_lights {
        44.0
    } else {
        16.0
    };
    let pad = theme.padding as f32;
    // Ancho automático: línea más larga (incluido el prompt) + padding.
    let prompt_len = capture.command_line.chars().count() + 2; // "❯ "
    let longest = capture
        .visible_lines()
        .map(|l| l.runs.iter().map(|r| r.text.chars().count()).sum::<usize>())
        .chain(std::iter::once(prompt_len))
        .max()
        .unwrap_or(20) as f32;
    let width = (pad + longest * theme.font_size as f32 * CH_WIDTH + pad).max(420.0);
    let height = (pad + chrome + line_h * visible as f32 + pad).max(160.0);
    Layout { width, height }
}

impl Capture {
    /// Filas sin las vacías al final (la terminal siempre reporta las 30).
    pub fn visible_lines(&self) -> impl Iterator<Item = &Line> {
        let last_content = self
            .lines
            .iter()
            .rposition(|l| !l.runs.is_empty())
            .map_or(0, |i| i + 1);
        self.lines[..last_content].iter()
    }
}

/// Genera el SVG completo de la tarjeta. `scale` multiplica dimensiones.
pub fn render_svg(capture: &Capture, theme: &Theme, palette: &Palette, scale: u32) -> String {
    let layout = layout(capture, theme);
    let w = layout.width * scale as f32;
    let h = layout.height * scale as f32;
    let fs = theme.font_size as f32 * scale as f32;
    let line_h = fs * LINE_HEIGHT;
    let pad = theme.padding as f32 * scale as f32;
    let chrome = if theme.show_traffic_lights {
        44.0 * scale as f32
    } else {
        16.0 * scale as f32
    };
    let radius = theme.corner_radius as f32 * scale as f32;

    let mut svg = String::with_capacity(64 * 1024);
    svg.push_str(&format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{w}" height="{h}" viewBox="0 0 {w} {h}">"#
    ));
    svg.push_str(&font_defs(scale));
    svg.push_str(&format!(
        r#"<defs><clipPath id="win"><rect width="{w}" height="{h}" rx="{radius}"/></clipPath></defs>"#,
    ));

    // Fondo exterior (backdrop).
    let backdrop = if theme.backdrop == "transparent" {
        "none"
    } else {
        &theme.backdrop
    };
    svg.push_str(&format!(
        r#"<rect width="{w}" height="{h}" fill="{backdrop}"/>"#
    ));

    // Sombra.
    if theme.show_shadow {
        svg.push_str(&format!(
            r#"<rect x="8" y="8" width="{bw}" height="{bh}" rx="{radius}" fill="black" opacity="0.35" filter="url(#blur)"/>"#,
            bw = w - 16.0,
            bh = h - 16.0,
        ));
        svg.push_str(r#"<defs><filter id="blur" x="-10%" y="-10%" width="120%" height="120%"><feGaussianBlur stdDeviation="6"/></filter></defs>"#);
    }

    // Ventana del terminal.
    let win_inset = if theme.show_shadow {
        8.0 * scale as f32
    } else {
        0.0
    };
    let ww = w - win_inset * 2.0;
    let wh = h - win_inset * 2.0;
    svg.push_str(&format!(
        r#"<g clip-path="url(#win)"><rect x="{xi}" y="{yi}" width="{ww}" height="{wh}" rx="{radius}" fill="{bg}"/>"#,
        xi = win_inset,
        yi = win_inset,
        bg = theme.background,
    ));

    // Barra de título + traffic lights.
    if theme.show_traffic_lights {
        let cy = win_inset + 20.0 * scale as f32;
        let lx = win_inset + pad;
        for (i, color) in ["#ff5f57", "#febc2e", "#28c840"].iter().enumerate() {
            let cx = lx + (12.0 * scale as f32) * i as f32;
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
                scale,
                &TextStyle {
                    bold: false,
                    italic: false,
                    anchor: "middle",
                },
            ));
        }
    }

    // Línea de prompt.
    let mut y = win_inset + chrome + line_h * 0.8;
    let prompt = format!("{} {}", theme.prompt_symbol, capture.command_line);
    svg.push_str(&text(
        win_inset + pad,
        y,
        &escape_xml(&prompt),
        &theme.accent,
        scale,
        &TextStyle {
            bold: true,
            italic: false,
            anchor: "start",
        },
    ));
    y += line_h;

    // Cuerpo: runs de la captura.
    for line in capture.visible_lines() {
        let mut x = win_inset + pad;
        for run in &line.runs {
            let w_run = run.text.chars().count() as f32 * fs * CH_WIDTH;
            if let Some(bg) = run.bg.as_ref() {
                let fill = resolve_color(bg, theme, palette, &theme.foreground);
                svg.push_str(&format!(
                    r#"<rect x="{x}" y="{ybg}" width="{w_run}" height="{line_h}" fill="{fill}"/>"#,
                    ybg = y - fs * 0.8,
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
                scale,
                &TextStyle {
                    bold: run.bold,
                    italic: run.italic,
                    anchor: "start",
                },
            ));
            if run.underline {
                svg.push_str(&format!(
                    r#"<line x1="{x}" y1="{yu}" x2="{}" y2="{yu}" stroke="{fill}" stroke-width="{}"/>"#,
                    x + w_run,
                    fs * 0.06,
                    yu = y + fs * 0.15,
                ));
            }
            x += w_run;
        }
        y += line_h;
    }

    svg.push_str("</g></svg>");
    svg
}

fn text(x: f32, y: f32, content: &str, fill: &str, scale: u32, style: &TextStyle) -> String {
    let family = font_family();
    format!(
        r#"<text x="{x}" y="{y}" font-family="{family}" font-weight="{weight}" font-style="{style}" font-size="{fs}" fill="{fill}" text-anchor="{anchor}">{content}</text>"#,
        fs = 14.0 * scale as f32,
        weight = if style.bold { "bold" } else { "normal" },
        style = if style.italic { "italic" } else { "normal" },
        anchor = style.anchor,
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

fn font_defs(scale: u32) -> String {
    let f = &*FONTS;
    let engine = base64::engine::general_purpose::STANDARD;
    let face = |data: &[u8], weight: &str, style: &str| {
        format!(
            r#"@font-face {{ font-family: '{fam}'; font-weight: {weight}; font-style: {style}; src: url(data:font/ttf;base64,{b64}) format('truetype'); }}"#,
            fam = font_family(),
            b64 = engine.encode(data),
        )
    };
    format!(
        r#"<defs><style>{} {} {} {}</style></defs>"#,
        face(f.regular, "normal", "normal"),
        face(f.bold, "bold", "normal"),
        face(f.italic, "normal", "italic"),
        face(f.bold_italic, "bold", "italic"),
    )
    .replace("{fs}", &format!("{}", 14 * scale))
}

fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// Rasteriza la captura a PNG con resvg. `scale` es 2, 3 o 4.
pub fn render_png(
    capture: &Capture,
    theme: &Theme,
    palette: &Palette,
    scale: u32,
) -> Result<Vec<u8>, String> {
    let svg = render_svg(capture, theme, palette, scale);

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
    let tree = usvg::Tree::from_str(&svg, &opt).map_err(|e| format!("SVG inválido: {e}"))?;

    let size = tree.size();
    let mut pixmap =
        tiny_skia::Pixmap::new(size.width().round() as u32, size.height().round() as u32)
            .ok_or("dimensiones de pixmap inválidas")?;

    resvg::render(
        &tree,
        tiny_skia::Transform::identity(),
        &mut pixmap.as_mut(),
    );
    pixmap
        .encode_png()
        .map_err(|e| format!("fallo al codificar PNG: {e}"))
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
        let (mut cap, theme, palette) = sample();
        cap.command_line = "echo \"<b>&\">".into();
        let svg = render_svg(&cap, &theme, &palette, 1);
        assert!(svg.contains("&lt;b&gt;&amp;"));
        assert!(!svg.contains("<b>"));
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
        // El ancho de la ventana es float; 2x/3x pueden diferir ±1 px por redondeo.
        assert!(((w2 as i64) - (w1 as i64) * 2).abs() <= 1);
        assert!(((h2 as i64) - (h1 as i64) * 2).abs() <= 1);
        assert!(((w3 as i64) - (w1 as i64) * 3).abs() <= 1);
        assert!(((h3 as i64) - (h1 as i64) * 3).abs() <= 1);
    }

    /// Lee (width, height) del IHDR de un PNG.
    fn png_dims(png: &[u8]) -> (u32, u32) {
        let be = |b: &[u8]| u32::from_be_bytes([b[0], b[1], b[2], b[3]]);
        (be(&png[16..20]), be(&png[20..24]))
    }
}
