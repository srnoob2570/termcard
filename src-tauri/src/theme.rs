use serde::{Deserialize, Serialize};

/// Tema de la tarjeta: parámetros planos que interpretan tanto el preview
/// web como el exportador SVG.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Theme {
    pub preset: String,
    /// Fondo exterior de la tarjeta (detrás de la ventana).
    pub backdrop: String,
    /// Fondo de la ventana del terminal.
    pub background: String,
    /// Color de primer plano por defecto del texto.
    pub foreground: String,
    /// Color de acento (prompt, título activo).
    pub accent: String,
    pub title: String,
    pub show_traffic_lights: bool,
    pub show_shadow: bool,
    pub corner_radius: u32,
    pub padding: u32,
    /// Tamaño de fuente en px (unidad base del render; el export multiplica).
    pub font_size: u32,
    pub prompt_symbol: String,
}

impl Default for Theme {
    fn default() -> Self {
        Theme {
            preset: "mac-dark".into(),
            backdrop: "transparent".into(),
            background: "#1e1e2e".into(),
            foreground: "#cdd6f4".into(),
            accent: "#cba6f7".into(),
            title: "usuario@localhost: ~".into(),
            show_traffic_lights: true,
            show_shadow: true,
            corner_radius: 12,
            padding: 24,
            font_size: 14,
            prompt_symbol: "❯".into(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Preset {
    MacDark,
    MacLight,
    Minimal,
    Solarized,
}

impl Preset {
    pub fn theme(self) -> Theme {
        let base = Theme::default();
        match self {
            Preset::MacDark => base,
            Preset::MacLight => Theme {
                preset: "mac-light".into(),
                background: "#f5f5f4".into(),
                foreground: "#3f3f46".into(),
                accent: "#d20f39".into(),
                ..base
            },
            Preset::Minimal => Theme {
                preset: "minimal".into(),
                show_traffic_lights: false,
                show_shadow: false,
                corner_radius: 6,
                title: String::new(),
                ..base
            },
            Preset::Solarized => Theme {
                preset: "solarized".into(),
                backdrop: "#002b36".into(),
                background: "#002b36".into(),
                foreground: "#93a1a1".into(),
                accent: "#b58900".into(),
                ..base
            },
        }
    }
}

/// Paleta de 16 colores ANSI (0-15) para resolver `Color::Indexed` > 15.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Palette {
    pub normal: [String; 8],
    pub bright: [String; 8],
}

impl Default for Palette {
    fn default() -> Self {
        // Catppuccin Mocha por defecto; cada preset trae el suyo.
        Palette {
            normal: [
                "#45475a".into(), // black
                "#f38ba8".into(), // red
                "#a6e3a1".into(), // green
                "#f9e2af".into(), // yellow
                "#89b4fa".into(), // blue
                "#f5c2e7".into(), // magenta
                "#94e2d5".into(), // cyan
                "#bac2de".into(), // white
            ],
            bright: [
                "#585b70".into(),
                "#f38ba8".into(),
                "#a6e3a1".into(),
                "#f9e2af".into(),
                "#89b4fa".into(),
                "#f5c2e7".into(),
                "#94e2d5".into(),
                "#a6adc8".into(),
            ],
        }
    }
}

impl Palette {
    /// Resuelve un índice ANSI 0-255 a color CSS.
    pub fn resolve(&self, index: u8) -> String {
        match index {
            0..=7 => self.normal[usize::from(index)].clone(),
            8..=15 => self.bright[usize::from(index) - 8].clone(),
            16..=231 => {
                let i = u32::from(index) - 16;
                let (r, g, b) = (i / 36, (i % 36) / 6, i % 6);
                let ch = |v: u32| if v == 0 { 0 } else { 55 + v * 40 };
                format!("#{:02x}{:02x}{:02x}", ch(r), ch(g), ch(b))
            }
            232..=255 => {
                let v = 8 + (u32::from(index) - 232) * 10;
                format!("#{v:02x}{v:02x}{v:02x}")
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preset_mac_light_overrides_colors() {
        let t = Preset::MacLight.theme();
        assert_eq!(t.background, "#f5f5f4");
        assert!(t.show_traffic_lights);
    }

    #[test]
    fn preset_minimal_hides_chrome() {
        let t = Preset::Minimal.theme();
        assert!(!t.show_traffic_lights);
        assert!(!t.show_shadow);
        assert!(t.title.is_empty());
    }

    #[test]
    fn palette_256_cube() {
        let p = Palette::default();
        assert_eq!(p.resolve(16), "#000000");
        assert_eq!(p.resolve(231), "#ffffff");
        assert_eq!(p.resolve(196), "#ff0000");
    }

    #[test]
    fn palette_grayscale_ramp() {
        let p = Palette::default();
        assert_eq!(p.resolve(232), "#080808");
        assert_eq!(p.resolve(255), "#eeeeee");
    }

    #[test]
    fn palette_base_uses_theme_colors() {
        let p = Palette::default();
        assert_eq!(p.resolve(0), "#45475a");
        assert_eq!(p.resolve(9), "#f38ba8");
    }

    #[test]
    fn theme_serde_roundtrip() {
        let t = Preset::Solarized.theme();
        let json = serde_json::to_string(&t).unwrap();
        let back: Theme = serde_json::from_str(&json).unwrap();
        assert_eq!(back, t);
    }
}
