//! Repro del bug PNG vacío: corre `cargo run --example export_repro`.
//! Rojo = PNG sin texto (píxeles del cuerpo idénticos al fondo).
use termcard_lib::export::render_png;
use termcard_lib::ir::{Capture, Line, Run};
use termcard_lib::theme::{Palette, Preset};

fn main() {
    let cap = Capture {
        cols: 40,
        rows: 2,
        command_line: "echo hola".into(),
        lines: vec![Line::from_runs(vec![Run {
            text: "hola mundo".into(),
            fg: None,
            bg: None,
            bold: false,
            italic: false,
            underline: false,
        }])],
    };
    let theme = Preset::MacDark.theme();
    let png = render_png(&cap, &theme, &Palette::default(), 2).expect("png");
    std::fs::write("/tmp/repro.png", &png).unwrap();

    // Cuenta píxeles que difieren del fondo (#1e1e2e) en la franja del cuerpo.
    let img = image::load_from_memory(&png).unwrap().to_rgb8();
    let (w, h) = (img.width(), img.height());
    let mut text_px = 0u64;
    // Color de texto por defecto del tema MacDark: #cdd6f4.
    // Cualquier píxel cercano a ese valor en la franja central = texto renderizado.
    for y in (h / 3)..(2 * h / 3) {
        for x in 0..w {
            let [r, g, b] = img.get_pixel(x, y).0;
            let fg = [0xcd, 0xd6, 0xf4];
            let d = (i32::from(r) - fg[0]).abs()
                + (i32::from(g) - fg[1]).abs()
                + (i32::from(b) - fg[2]).abs();
            if d < 90 {
                text_px += 1;
            }
        }
    }
    println!("FRANJA_CUERPO pixeles_de_texto={text_px} png={w}x{h}");
    if text_px < 50 {
        std::process::exit(1); // ROJO: cuerpo sin texto
    }
    println!("VERDE: hay texto en el cuerpo");
}
