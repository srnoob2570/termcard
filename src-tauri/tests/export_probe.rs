//! Harness de diagnóstico: renderiza una fixture a SVG y PNG con el pipeline
//! real de export y escribe los artefactos en /tmp/termcard-debug para medir
//! geometría (margen, radio, padding) contra lo esperado del tema.
//!
//! Uso: cargo test -p termcard --test export_probe -- --nocapture
//! No es una prueba de regresión: se elimina tras el diagnóstico.

use termcard_lib::export::{layout, render_png, render_svg};
use termcard_lib::ir::{Capture, Color, Line, Run};
use termcard_lib::theme::{Palette, Theme};

fn fixture() -> (Capture, Theme, Palette) {
    // Imita la tarjeta del usuario: prompt + líneas con runs de color,
    // barra de progreso con bg, flechas y símbolos no-ASCII, línea larga.
    let run = |text: &str, fg: Option<Color>, bg: Option<Color>, bold: bool| Run {
        text: text.to_string(),
        fg,
        bg,
        bold,
        italic: false,
        underline: false,
    };
    let cap = Capture {
        cols: 100,
        rows: 10,
        command_line: "omp usage".into(),
        lines: vec![
            Line::from_runs(vec![run(
                "Usage · fetched 2m58s ago",
                Some(Color::Default),
                None,
                false,
            )]),
            Line::empty(),
            Line::from_runs(vec![
                run("Ollama Cloud", Some(Color::Indexed(4)), None, true),
                run(" — 1 account", Some(Color::Default), None, false),
            ]),
            Line::from_runs(vec![run(
                "  Quota fractions as reported by ollama.com /api/usage.",
                Some(Color::Default),
                None,
                false,
            )]),
            Line::from_runs(vec![
                run("   ● Session ", Some(Color::Default), None, false),
                run("████░░░░░░░░░░░░░░░░", None, Some(Color::Indexed(7)), false),
                run("  17.0% used", Some(Color::Default), None, false),
            ]),
            Line::from_runs(vec![run(
                "capacity: 5h → 0.17/1 account used (0.83× quota left)",
                Some(Color::Default),
                None,
                false,
            )]),
            Line::from_runs(vec![run(
                "  glm-5.3-flash (9589 requests) · kimi-k3 (51 requests)",
                Some(Color::Default),
                None,
                false,
            )]),
        ],
    };
    (cap, Theme::default(), Palette::default())
}

#[test]
fn probe_dumps_artifacts() {
    let (cap, theme, palette) = fixture();
    let lay = layout(&cap, &theme);
    println!("layout logico: {}x{}", lay.width, lay.height);
    println!(
        "esperado: win_inset={} (margin {} + sombra 8), pad={}, radius={}",
        theme.outer_margin + 8,
        theme.outer_margin,
        theme.padding,
        theme.corner_radius
    );

    for scale in [1u32, 2u32] {
        let svg = render_svg(&cap, &theme, &palette, scale);
        let path = format!("/tmp/termcard-debug/scale{scale}.svg");
        std::fs::create_dir_all("/tmp/termcard-debug").unwrap();
        std::fs::write(&path, &svg).unwrap();
        println!("svg escala {scale}: {} bytes → {path}", svg.len());
        // Cabecera con dimensiones para verificar viewBox.
        let head: String = svg.chars().take(220).collect();
        println!("cabecera svg {scale}: {head}");
    }

    let png = render_png(&cap, &theme, &palette, 2).expect("png");
    let path = "/tmp/termcard-debug/scale2.png";
    std::fs::write(path, &png).unwrap();
    println!("png escala 2: {} bytes → {path}", png.len());

    // Medición geométrica con el crate image (dev-dependency).
    let img = image::load_from_memory(&png).unwrap().to_rgba8();
    let (w, h) = img.dimensions();
    println!(
        "png dims: {w}x{h} (esperado {}x{})",
        lay.width * 2.0,
        lay.height * 2.0
    );

    let alpha_at = |x: u32, y: u32| img.get_pixel(x, y).0[3];
    // Margen: en (2,2) debe ser transparente (backdrop transparent + margen 16*2).
    println!("alpha(2,2)={:?} (margen: esperado 0)", alpha_at(2, 2));
    // Esquina de la ventana: (win_inset*2, win_inset*2) cae FUERA del radio → 0.
    let wi = (theme.outer_margin + 8) * 2;
    println!(
        "alpha({wi},{wi})={:?} (esquina redondeada: esperado 0)",
        alpha_at(wi, wi)
    );
    // Justo dentro del borde derecho de la ventana: opaco.
    let win_right = (w as f32 / 2.0 - (theme.outer_margin + 8) as f32) as u32 - 2;
    let mid_y = h / 2;
    println!(
        "alpha({win_right},{mid_y})={:?} (dentro de ventana: esperado 255)",
        alpha_at(win_right, mid_y)
    );
    // Borde izquierdo de la ventana (dentro, tras el radio): opaco.
    println!(
        "alpha({},{mid_y})={:?} (dentro de ventana: esperado 255)",
        wi + 4,
        alpha_at(wi + 4, mid_y)
    );

    // Primera columna opaca por fila central → margen real renderizado.
    let mut first_opaque = None;
    for x in 0..w {
        if alpha_at(x, mid_y) > 8 {
            first_opaque = Some(x);
            break;
        }
    }
    println!(
        "primera columna opaca en y={mid_y}: {:?} (esperado ~{})",
        first_opaque,
        wi + 4
    );

    // Bbox del texto: píxeles que no son ni backdrop ni bg de la ventana,
    // muestreado en la línea del prompt (y = (win_inset + chrome + 0.5*line_h)*2).
    // Aproximación: escanear toda la imagen y reportar extents de píxeles
    // distintos del bg de ventana.
    let bg = theme.background.clone();
    let hex = |s: &str| -> (u8, u8, u8) {
        let s = s.trim_start_matches('#');
        (
            u8::from_str_radix(&s[0..2], 16).unwrap(),
            u8::from_str_radix(&s[2..4], 16).unwrap(),
            u8::from_str_radix(&s[4..6], 16).unwrap(),
        )
    };
    let (br, bgn, bb) = hex(&bg);
    let mut min_x = u32::MAX;
    let mut max_x = 0u32;
    for y in 0..h {
        for x in 0..w {
            let p = img.get_pixel(x, y).0;
            if p[3] > 8
                && (p[0].abs_diff(br) > 12 || p[1].abs_diff(bgn) > 12 || p[2].abs_diff(bb) > 12)
            {
                if x < min_x {
                    min_x = x;
                }
                if x > max_x {
                    max_x = x;
                }
            }
        }
    }
    println!(
        "bbox contenido x: [{min_x},{max_x}] (ventana: [{},{}/2-{}])",
        wi + 4,
        w,
        wi + 4
    );
}
