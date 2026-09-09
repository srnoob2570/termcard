//! Diagnostic harness: renders a fixture to SVG and PNG with the real export
//! pipeline and writes the artifacts to /tmp/termcard-debug to measure
//! geometry (margin, radius, padding) against what the theme expects.
//!
//! Usage: cargo test -p termcard --test export_probe -- --nocapture
//! Not a regression test: it is removed after the diagnosis.

use termcard_lib::export::{layout, render_png, render_svg};
use termcard_lib::ir::{Capture, Color, Line, Run};
use termcard_lib::theme::{Palette, Theme};

fn fixture() -> (Capture, Theme, Palette) {
    // Imitates the user's card: prompt + lines with colored runs,
    // progress bar with bg, arrows and non-ASCII symbols, long line.
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
fn probe_no_shadow_case() {
    // Solarized/Minimal: show_shadow=false. The text must stay within the
    // right padding (no overflow) and the margin is still present.
    let (cap, mut theme, palette) = fixture();
    theme.show_shadow = false;
    let lay = layout(&cap, &theme);
    let svg = render_svg(&cap, &theme, &palette, 2);
    std::fs::create_dir_all("/tmp/termcard-debug").unwrap();
    std::fs::write("/tmp/termcard-debug/noshadow.svg", &svg).unwrap();
    println!("no-shadow layout: {}x{}", lay.width, lay.height);
    let win_inset = theme.outer_margin as f32; // no shadow gap
    println!(
        "win_inset={} → text area: {}..{} (px@2x: {}..{})",
        win_inset,
        win_inset + theme.padding as f32,
        lay.width - win_inset - theme.padding as f32,
        (win_inset + theme.padding as f32) * 2.0,
        (lay.width - win_inset - theme.padding as f32) * 2.0
    );
    let png = render_png(&cap, &theme, &palette, 2).expect("png");
    std::fs::write("/tmp/termcard-debug/noshadow.png", &png).unwrap();
    let img = image::load_from_memory(&png).unwrap().to_rgba8();
    let (w, h) = img.dimensions();
    let bg = {
        let s = theme.background.trim_start_matches('#');
        (
            u8::from_str_radix(&s[0..2], 16).unwrap(),
            u8::from_str_radix(&s[2..4], 16).unwrap(),
            u8::from_str_radix(&s[4..6], 16).unwrap(),
        )
    };
    let px = |x: u32, y: u32| img.get_pixel(x, y).0;
    let is_text = |p: [u8; 4]| {
        p[3] > 200
            && (p[0].abs_diff(bg.0) as u32
                + p[1].abs_diff(bg.1) as u32
                + p[2].abs_diff(bg.2) as u32)
                > 30
    };
    let mut mx = 0u32;
    for y in 32..h - 32 {
        for x in (32..w - 32).rev() {
            if is_text(px(x, y)) {
                mx = mx.max(x);
                break;
            }
        }
    }
    let pad_right_px = (lay.width - win_inset - theme.padding as f32) * 2.0;
    println!("max text x = {mx}, right pad edge px = {pad_right_px}");
    assert!(
        (mx as f32) <= pad_right_px + 1.0,
        "text overflows the right padding: {mx} > {pad_right_px}"
    );
}

#[test]
fn probe_dumps_artifacts() {
    let (cap, theme, palette) = fixture();
    let lay = layout(&cap, &theme);
    println!("logical layout: {}x{}", lay.width, lay.height);
    println!(
        "expected: win_inset={} (margin {} + shadow 8), pad={}, radius={}",
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
        println!("svg scale {scale}: {} bytes → {path}", svg.len());
        // Header with dimensions to verify the viewBox.
        let head: String = svg.chars().take(220).collect();
        println!("svg header {scale}: {head}");
    }

    let png = render_png(&cap, &theme, &palette, 2).expect("png");
    let path = "/tmp/termcard-debug/scale2.png";
    std::fs::write(path, &png).unwrap();
    println!("png scale 2: {} bytes → {path}", png.len());

    // Geometric measurement with the image crate (dev-dependency).
    let img = image::load_from_memory(&png).unwrap().to_rgba8();
    let (w, h) = img.dimensions();
    println!(
        "png dims: {w}x{h} (expected {}x{})",
        lay.width * 2.0,
        lay.height * 2.0
    );

    let alpha_at = |x: u32, y: u32| img.get_pixel(x, y).0[3];
    // Margin: at (2,2) it must be transparent (transparent backdrop + 16*2 margin).
    println!("alpha(2,2)={:?} (margin: expected 0)", alpha_at(2, 2));
    // Window corner: (win_inset*2, win_inset*2) falls OUTSIDE the radius → 0.
    let wi = (theme.outer_margin + 8) * 2;
    println!(
        "alpha({wi},{wi})={:?} (rounded corner: expected 0)",
        alpha_at(wi, wi)
    );
    // Just inside the right edge of the window: opaque.
    let win_right = (w as f32 / 2.0 - (theme.outer_margin + 8) as f32) as u32 - 2;
    let mid_y = h / 2;
    println!(
        "alpha({win_right},{mid_y})={:?} (inside window: expected 255)",
        alpha_at(win_right, mid_y)
    );
    // Left edge of the window (inside, past the radius): opaque.
    println!(
        "alpha({},{mid_y})={:?} (inside window: expected 255)",
        wi + 4,
        alpha_at(wi + 4, mid_y)
    );

    // First opaque column at the mid row → actual rendered margin.

    let mut first_opaque = None;
    for x in 0..w {
        if alpha_at(x, mid_y) > 8 {
            first_opaque = Some(x);
            break;
        }
    }
    println!(
        "first opaque column at y={mid_y}: {:?} (expected ~{})",
        first_opaque,
        wi + 4
    );

    // Text bbox: pixels that are neither backdrop nor window background,
    // sampled at the prompt line (y = (win_inset + chrome + 0.5*line_h)*2).
    // Approximation: scan the whole image and report extents of pixels
    // different from the window background.
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
        "content bbox x: [{min_x},{max_x}] (window: [{},{}/2-{}])",
        wi + 4,
        w,
        wi + 4
    );
}
