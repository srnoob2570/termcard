use serde::{Deserialize, Serialize};

/// Card theme: plain parameters interpreted by both the web preview
/// and the SVG exporter.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Theme {
    pub preset: String,
    /// Outer backdrop of the card (behind the window).
    pub backdrop: String,
    /// Terminal window background.
    pub background: String,
    /// Default foreground color of the text.
    pub foreground: String,
    /// Accent color (prompt, active title).
    pub accent: String,
    pub title: String,
    pub show_traffic_lights: bool,
    pub show_shadow: bool,
    pub corner_radius: u32,
    /// Window inner padding (around the text).
    pub padding: u32,
    /// Transparent outer margin around the card.
    pub outer_margin: u32,
    /// Font size in px (base render unit; the export multiplies it).
    pub font_size: u32,
    /// Fixed card (terminal window) width in px; `None` fits the width to the
    /// longest line (automatic).
    pub card_width: Option<u32>,
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
            title: "user@localhost: ~".into(),
            show_traffic_lights: true,
            show_shadow: true,
            corner_radius: 12,
            outer_margin: 8,
            padding: 24,
            font_size: 14,
            card_width: None,
            prompt_symbol: "❯".into(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Preset {
    // Dark presets.
    MacDark,
    TokyoNight,
    Dracula,
    GruvboxDark,
    Nord,
    Solarized,
    // Light presets.
    MacLight,
    Latte,
    SolarizedLight,
    GruvboxLight,
    NordLight,
}

impl Preset {
    /// All presets, in UI order.
    pub const ALL: [Preset; 11] = [
        Preset::MacDark,
        Preset::TokyoNight,
        Preset::Dracula,
        Preset::GruvboxDark,
        Preset::Nord,
        Preset::Solarized,
        Preset::MacLight,
        Preset::Latte,
        Preset::SolarizedLight,
        Preset::GruvboxLight,
        Preset::NordLight,
    ];

    /// Preset from its theme identifier (`preset` in the JSON).
    pub fn from_id(id: &str) -> Option<Preset> {
        Preset::ALL.iter().copied().find(|p| p.id() == id)
    }

    /// Stable identifier of the preset (matches `theme.preset`).
    pub fn id(self) -> &'static str {
        match self {
            Preset::MacDark => "mac-dark",
            Preset::TokyoNight => "tokyo-night",
            Preset::Dracula => "dracula",
            Preset::GruvboxDark => "gruvbox-dark",
            Preset::Nord => "nord",
            Preset::Solarized => "solarized",
            Preset::MacLight => "mac-light",
            Preset::Latte => "catppuccin-latte",
            Preset::SolarizedLight => "solarized-light",
            Preset::GruvboxLight => "gruvbox-light",
            Preset::NordLight => "nord-light",
        }
    }

    /// Full theme of each preset. SINGLE source of truth: the UI requests it
    /// over IPC (`preset_theme`) and replaces its entire theme with the response.
    pub fn theme(self) -> Theme {
        let base = Theme::default();
        match self {
            Preset::MacDark => base,
            Preset::TokyoNight => Theme {
                preset: "tokyo-night".into(),
                background: "#1a1b26".into(),
                foreground: "#a9b1d6".into(),
                accent: "#7aa2f7".into(),
                ..base
            },
            Preset::Dracula => Theme {
                preset: "dracula".into(),
                background: "#282a36".into(),
                foreground: "#f8f8f2".into(),
                accent: "#bd93f9".into(),
                ..base
            },
            Preset::GruvboxDark => Theme {
                preset: "gruvbox-dark".into(),
                background: "#282828".into(),
                foreground: "#ebdbb2".into(),
                accent: "#fe8019".into(),
                ..base
            },
            Preset::Nord => Theme {
                preset: "nord".into(),
                background: "#2e3440".into(),
                foreground: "#d8dee9".into(),
                accent: "#88c0d0".into(),
                ..base
            },
            Preset::Solarized => Theme {
                preset: "solarized".into(),
                background: "#002b36".into(),
                foreground: "#93a1a1".into(),
                accent: "#b58900".into(),
                ..base
            },
            Preset::MacLight => Theme {
                preset: "mac-light".into(),
                background: "#f5f5f4".into(),
                foreground: "#3f3f46".into(),
                accent: "#d20f39".into(),
                ..base
            },
            Preset::Latte => Theme {
                preset: "catppuccin-latte".into(),
                background: "#eff1f5".into(),
                foreground: "#4c4f69".into(),
                accent: "#8839ef".into(),
                ..base
            },
            Preset::SolarizedLight => Theme {
                preset: "solarized-light".into(),
                background: "#fdf6e3".into(),
                foreground: "#586e75".into(),
                accent: "#268bd2".into(),
                ..base
            },
            Preset::GruvboxLight => Theme {
                preset: "gruvbox-light".into(),
                background: "#fbf1c7".into(),
                foreground: "#3c3836".into(),
                accent: "#b57614".into(),
                ..base
            },
            Preset::NordLight => Theme {
                preset: "nord-light".into(),
                background: "#eceff4".into(),
                foreground: "#2e3440".into(),
                accent: "#5e81ac".into(),
                ..base
            },
        }
    }
}

/// Palette of 16 ANSI colors (0-15) to resolve `Color::Indexed` > 15.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Palette {
    pub normal: [String; 8],
    pub bright: [String; 8],
}

impl Default for Palette {
    fn default() -> Self {
        // Catppuccin Mocha by default. ANSI-256 indexed colors are resolved
        // against this default palette everywhere (both export commands
        // hardcode it); per-preset palettes are not wired yet.
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
    /// Resolves an ANSI index 0-255 to a CSS color.
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

    /// New presets use their official scheme colors.
    #[test]
    fn presets_use_official_palette() {
        let cases: &[(&str, &str, &str, &str)] = &[
            ("tokyo-night", "#1a1b26", "#a9b1d6", "#7aa2f7"),
            ("dracula", "#282a36", "#f8f8f2", "#bd93f9"),
            ("gruvbox-dark", "#282828", "#ebdbb2", "#fe8019"),
            ("nord", "#2e3440", "#d8dee9", "#88c0d0"),
            ("catppuccin-latte", "#eff1f5", "#4c4f69", "#8839ef"),
            ("solarized-light", "#fdf6e3", "#586e75", "#268bd2"),
            ("gruvbox-light", "#fbf1c7", "#3c3836", "#b57614"),
            ("nord-light", "#eceff4", "#2e3440", "#5e81ac"),
        ];
        for (id, bg, fg, accent) in cases {
            let preset = Preset::from_id(id).unwrap();
            let t = preset.theme();
            assert_eq!(t.background, *bg, "{}", id);
            assert_eq!(t.foreground, *fg, "{}", id);
            assert_eq!(t.accent, *accent, "{}", id);
        }
    }

    /// All presets share the normalized layout geometry; only colors and
    /// chrome differ between them.
    #[test]
    fn presets_share_normalized_layout() {
        for preset in Preset::ALL {
            let t = preset.theme();
            assert_eq!(t.font_size, 14, "{}", preset.id());
            assert_eq!(t.corner_radius, 12, "{}", preset.id());
            assert_eq!(t.padding, 24, "{}", preset.id());
            assert_eq!(t.outer_margin, 8, "{}", preset.id());
            assert_eq!(t.backdrop, "transparent", "{}", preset.id());
            assert!(t.show_shadow, "{}", preset.id());
        }
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
