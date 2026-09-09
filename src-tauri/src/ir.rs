use serde::{Deserialize, Serialize};
use vt100::Color as VtColor;

/// Color de una celda en el IR. `Default` = color de primer plano del tema;
/// `DefaultInverted` = fondo del tema (video inverso).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Color {
    Indexed(u8),
    Rgb(u8, u8, u8),
    Default,
    DefaultInverted,
}

/// Tramo de texto contiguo con estilo uniforme.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Run {
    pub text: String,
    pub fg: Option<Color>,
    pub bg: Option<Color>,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Line {
    pub runs: Vec<Run>,
}

/// Captura completa: grilla lógica + línea de comando a mostrar.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Capture {
    pub cols: u16,
    pub rows: u16,
    pub command_line: String,
    pub lines: Vec<Line>,
}

impl Line {
    pub fn from_runs(runs: Vec<Run>) -> Self {
        Line { runs }
    }

    pub fn empty() -> Self {
        Line { runs: vec![] }
    }
}

/// Convierte la pantalla final de un `vt100::Parser` en el IR.
///
/// Recorre celda por celda (`Screen::cell` es el único acceso público),
/// omite continuaciones de caracteres anchos y fusiona celdas contiguas
/// con estilo idéntico en un solo run.
pub fn from_vt100(parser: &vt100::Parser, command_line: String) -> Capture {
    let screen = parser.screen();
    let (rows, cols) = screen.size();
    let mut lines = Vec::with_capacity(usize::from(rows));

    for row in 0..rows {
        let mut runs: Vec<Run> = Vec::new();
        for col in 0..cols {
            let Some(cell) = screen.cell(row, col) else {
                continue;
            };
            if !cell.has_contents() || cell.is_wide_continuation() {
                continue;
            }
            let (fg, bg) = convert_colors(cell.fgcolor(), cell.bgcolor(), cell.inverse());
            let run = Run {
                text: cell.contents().to_string(),
                fg,
                bg,
                bold: cell.bold(),
                italic: cell.italic(),
                underline: cell.underline(),
            };
            merge_run(&mut runs, run);
        }
        lines.push(Line::from_runs(runs));
    }

    Capture {
        cols,
        rows,
        command_line,
        lines,
    }
}

/// Mapea los colores vt100 al IR. Con video inverso, primer plano y fondo
/// se intercambian; `Default` invertido se representa con `DefaultInverted`
/// para que ambos renderizadores lo resuelvan contra el tema.
fn convert_colors(fg: VtColor, bg: VtColor, inverse: bool) -> (Option<Color>, Option<Color>) {
    if inverse {
        (to_color(bg, true), to_color(fg, true))
    } else {
        (to_color(fg, false), to_color(bg, false))
    }
}

fn to_color(c: VtColor, inverted: bool) -> Option<Color> {
    match c {
        VtColor::Default if inverted => Some(Color::DefaultInverted),
        VtColor::Default => None,
        VtColor::Idx(i) => Some(Color::Indexed(i)),
        VtColor::Rgb(r, g, b) => Some(Color::Rgb(r, g, b)),
    }
}

/// Fusiona runs adyacentes con estilo idéntico para reducir el IR.
fn merge_run(runs: &mut Vec<Run>, next: Run) {
    if let Some(last) = runs.last_mut() {
        if last.fg == next.fg
            && last.bg == next.bg
            && last.bold == next.bold
            && last.italic == next.italic
            && last.underline == next.underline
        {
            last.text.push_str(&next.text);
            return;
        }
    }
    runs.push(next);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(input: &[u8], rows: u16, cols: u16) -> vt100::Parser {
        let mut p = vt100::Parser::new(rows, cols, 0);
        p.process(input);
        p
    }

    #[test]
    fn plain_text_single_run() {
        let p = parse(b"hello world\n", 24, 80);
        let cap = from_vt100(&p, "hello world".into());
        assert_eq!(cap.lines[0].runs.len(), 1);
        assert_eq!(cap.lines[0].runs[0].text, "hello world");
        assert_eq!(cap.lines[0].runs[0].fg, None);
        assert!(!cap.lines[0].runs[0].bold);
    }

    #[test]
    fn bold_creates_split_runs() {
        let p = parse(b"\x1b[1mbold\x1b[0m normal\n", 24, 80);
        let cap = from_vt100(&p, "cmd".into());
        let runs = &cap.lines[0].runs;
        assert_eq!(runs.len(), 2);
        assert!(runs[0].bold);
        assert_eq!(runs[0].text, "bold");
        assert!(!runs[1].bold);
        assert_eq!(runs[1].text, " normal");
    }

    #[test]
    fn truecolor_preserved() {
        let p = parse(b"\x1b[38;2;255;100;50mrgb\x1b[0m\n", 24, 80);
        let cap = from_vt100(&p, "cmd".into());
        assert_eq!(cap.lines[0].runs[0].fg, Some(Color::Rgb(255, 100, 50)));
    }

    #[test]
    fn indexed_256_preserved() {
        let p = parse(b"\x1b[38;5;196mred\x1b[0m\n", 24, 80);
        let cap = from_vt100(&p, "cmd".into());
        assert_eq!(cap.lines[0].runs[0].fg, Some(Color::Indexed(196)));
    }

    #[test]
    fn background_preserved() {
        let p = parse(b"\x1b[44;37m blue bg \x1b[0m\n", 24, 80);
        let cap = from_vt100(&p, "cmd".into());
        let run = &cap.lines[0].runs[0];
        assert_eq!(run.bg, Some(Color::Indexed(4)));
    }

    #[test]
    fn inverse_swaps_default_colors() {
        let p = parse(b"\x1b[7minv\x1b[0m\n", 24, 80);
        let cap = from_vt100(&p, "cmd".into());
        let run = &cap.lines[0].runs[0];
        assert_eq!(run.fg, Some(Color::DefaultInverted));
        assert_eq!(run.bg, Some(Color::DefaultInverted));
        assert_eq!(run.text, "inv");
    }

    #[test]
    fn empty_row_yields_empty_line() {
        let p = parse(b"a\n\nb\n", 24, 80);
        let cap = from_vt100(&p, "cmd".into());
        assert!(cap.lines[1].runs.is_empty());
        assert_eq!(cap.lines[2].runs[0].text, "b");
    }

    #[test]
    fn command_line_carried() {
        let p = parse(b"out\n", 24, 80);
        let cap = from_vt100(&p, "ls -la".into());
        assert_eq!(cap.command_line, "ls -la");
    }
}
