use serde::{Deserialize, Serialize};
use vt100::Color as VtColor;

/// Color of a cell in the IR. `Default` = the theme's foreground color;
/// `DefaultInverted` = the theme's background (reverse video).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Color {
    Indexed(u8),
    Rgb(u8, u8, u8),
    /// Not produced by `from_vt100` (a default fg serializes as `None` on
    /// runs); kept for round-trip deserialization of captures saved by
    /// older versions of the app.
    Default,
    DefaultInverted,
}

/// Contiguous span of text with uniform style.
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

/// Full capture: logical grid + command line to display.
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

/// Converts the final screen of a `vt100::Parser` into the IR.
///
/// Walks cell by cell (`Screen::cell` is the only public access),
/// skips wide-character continuations and merges contiguous cells
/// with identical style into a single run. Gaps of unwritten cells
/// between written cells (tabs, cursor movement) are preserved as
/// literal spaces styled with the gap cell's own attributes.
pub fn from_vt100(parser: &vt100::Parser, command_line: String) -> Capture {
    let screen = parser.screen();
    let (rows, cols) = screen.size();
    let mut lines = Vec::with_capacity(usize::from(rows));

    for row in 0..rows {
        let mut runs: Vec<Run> = Vec::new();
        let mut last_col: Option<u16> = None;
        for col in 0..cols {
            let Some(cell) = screen.cell(row, col) else {
                continue;
            };
            if !cell.has_contents() || cell.is_wide_continuation() {
                continue;
            }
            // Preserve the gap of unwritten cells before this one as literal
            // spaces (each carrying its cell's style), so cursor jumps don't
            // glue the surrounding text together.
            if let Some(prev) = last_col {
                for gap_col in prev + 1..col {
                    let Some(gap_cell) = screen.cell(row, gap_col) else {
                        continue;
                    };
                    if gap_cell.is_wide_continuation() {
                        continue;
                    }
                    merge_run(&mut runs, cell_run(gap_cell, " ".to_string()));
                }
            }
            merge_run(&mut runs, cell_run(cell, cell.contents().to_string()));
            last_col = Some(col);
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

impl Capture {
    /// Trims the capture to its content: removes trailing empty rows and
    /// adjusts `cols` to the real content width. `rows` ends up with the
    /// rows that have content.
    pub fn trimmed(mut self) -> Self {
        let last_content = self
            .lines
            .iter()
            .rposition(|l| !l.runs.is_empty())
            .map_or(0, |i| i + 1);
        self.lines.truncate(last_content);
        let max_len = self
            .lines
            .iter()
            .map(|l| l.runs.iter().map(|r| str_width(&r.text)).sum::<usize>())
            .chain(std::iter::once(str_width(&self.command_line)))
            .max()
            .unwrap_or(1);
        self.cols = (max_len as u16 + 1).min(self.cols);
        self.rows = self.lines.len().min(usize::from(u16::MAX)) as u16;
        self
    }
}

/// Display width of a string in terminal columns (wide CJK chars count 2).
pub fn str_width(s: &str) -> usize {
    unicode_width::UnicodeWidthStr::width(s)
}

/// Maps vt100 colors to the IR. With reverse video, foreground and background
/// are swapped; an inverted `Default` is represented as `DefaultInverted`
/// so both renderers resolve it against the theme.
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

/// Builds a run from a cell's contents and its converted style.
fn cell_run(cell: &vt100::Cell, text: String) -> Run {
    let (fg, bg) = convert_colors(cell.fgcolor(), cell.bgcolor(), cell.inverse());
    Run {
        text,
        fg,
        bg,
        bold: cell.bold(),
        italic: cell.italic(),
        underline: cell.underline(),
    }
}

/// Merges adjacent runs with identical style to shrink the IR.
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

    #[test]
    fn tab_gap_preserved() {
        let p = parse(b"a\tb\n", 24, 80);
        let cap = from_vt100(&p, "cmd".into());
        let joined = cap.lines[0]
            .runs
            .iter()
            .map(|r| r.text.as_str())
            .collect::<String>();
        // Tab moves the cursor to column 8: 'b' lands after 7 blank columns.
        assert_eq!(joined, format!("a{}b", " ".repeat(7)));
        assert!(cap.lines[0].runs.len() >= 1);
    }

    #[test]
    fn trimmed_counts_wide_char_width() {
        let cap = Capture {
            cols: 80,
            rows: 1,
            command_line: "x".into(),
            lines: vec![Line::from_runs(vec![Run {
                text: "你好".into(),
                fg: None,
                bg: None,
                bold: false,
                italic: false,
                underline: false,
            }])],
        };
        // Two CJK chars occupy 4 columns, plus 1 => 5 (not 3 by char count).
        assert_eq!(cap.trimmed().cols, 5);
    }
}
