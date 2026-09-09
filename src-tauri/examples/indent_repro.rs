//! Repro de sangrías perdidas: línea con espacios iniciales + separación interna.
use termcard_lib::export::render_png;
use termcard_lib::ir::{Capture, Line, Run};
use termcard_lib::theme::{Palette, Preset};

fn main() {
    let cap = Capture {
        cols: 40,
        rows: 2,
        command_line: "echo test".into(),
        lines: vec![Line::from_runs(vec![Run {
            text: "    indented".into(),
            fg: None,
            bg: None,
            bold: false,
            italic: false,
            underline: false,
        }])],
    };
    let png = render_png(&cap, &Preset::MacDark.theme(), &Palette::default(), 2).expect("png");
    let img = image::load_from_memory(&png).unwrap().to_rgb8();
    // Primer píxel claro en la fila del cuerpo: donde empieza el texto.
    let h = img.height();
    let mut first_light_x = None;
    'outer: for y in (h / 2)..(h - 20) {
        for x in 0..img.width() {
            let [r, g, b] = img.get_pixel(x, y).0;
            if i32::from(r) + i32::from(g) + i32::from(b) > 300 {
                first_light_x = Some(x);
                break 'outer;
            }
        }
    }
    let fx = first_light_x.expect("no text found");
    println!("PRIMER_PIXEL_X={fx}");
    // Con sangría preservada el texto arranca > 60 px (4 espacios × 16.8px a 2x).
    if fx < 60 {
        std::process::exit(1); // ROJO: sangría colapsada
    }
    println!("VERDE: sangría preservada");
}
