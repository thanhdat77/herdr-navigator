use std::{
    env, fs,
    path::{Path, PathBuf},
};

use ratatui::style::Color;

use crate::paths::home;

#[derive(Clone)]
pub(crate) struct Theme {
    pub(crate) accent: Color,
    pub(crate) panel_bg: Color,
    pub(crate) surface0: Color,
    pub(crate) surface1: Color,
    pub(crate) surface_dim: Color,
    pub(crate) overlay0: Color,
    pub(crate) overlay1: Color,
    pub(crate) text: Color,
    pub(crate) subtext0: Color,
    pub(crate) green: Color,
    pub(crate) yellow: Color,
    pub(crate) red: Color,
    pub(crate) blue: Color,
    pub(crate) teal: Color,
    pub(crate) mauve: Color,
    pub(crate) peach: Color,
}

impl Theme {
    fn catppuccin() -> Self {
        Self {
            accent: rgb(137, 180, 250), // blue
            panel_bg: rgb(24, 24, 37),
            surface0: rgb(49, 50, 68),
            surface1: rgb(69, 71, 90),
            surface_dim: rgb(30, 30, 46),
            overlay0: rgb(108, 112, 134),
            overlay1: rgb(127, 132, 156),
            text: rgb(205, 214, 244),
            subtext0: rgb(166, 173, 200),
            mauve: rgb(203, 166, 247),
            green: rgb(166, 227, 161),
            yellow: rgb(249, 226, 175),
            red: rgb(243, 139, 168),
            blue: rgb(137, 180, 250),
            teal: rgb(148, 226, 213),
            peach: rgb(250, 179, 135),
        }
    }

    fn catppuccin_latte() -> Self {
        Self {
            accent: rgb(30, 102, 245),
            panel_bg: rgb(239, 241, 245),
            surface0: rgb(204, 208, 218),
            surface1: rgb(188, 192, 204),
            surface_dim: rgb(230, 233, 239),
            overlay0: rgb(156, 160, 176),
            overlay1: rgb(140, 143, 161),
            text: rgb(76, 79, 105),
            subtext0: rgb(108, 111, 133),
            mauve: rgb(136, 57, 239),
            green: rgb(64, 160, 43),
            yellow: rgb(223, 142, 29),
            red: rgb(210, 15, 57),
            blue: rgb(30, 102, 245),
            teal: rgb(23, 146, 153),
            peach: rgb(254, 100, 11),
        }
    }

    fn terminal() -> Self {
        Self {
            accent: Color::Blue,
            panel_bg: Color::Reset,
            surface0: Color::Reset,
            surface1: Color::DarkGray,
            surface_dim: Color::DarkGray,
            overlay0: Color::Gray,
            overlay1: Color::White,
            text: Color::Reset,
            subtext0: Color::Gray,
            mauve: Color::Gray,
            green: Color::Green,
            yellow: Color::Yellow,
            red: Color::LightRed,
            blue: Color::Blue,
            teal: Color::Cyan,
            peach: Color::Yellow,
        }
    }

    fn tokyo_night() -> Self {
        Self {
            accent: rgb(122, 162, 247), // blue
            panel_bg: rgb(26, 27, 38),
            surface0: rgb(36, 40, 59),
            surface1: rgb(65, 72, 104),
            surface_dim: rgb(26, 27, 38),
            overlay0: rgb(86, 95, 137),
            overlay1: rgb(105, 113, 150),
            text: rgb(192, 202, 245),
            subtext0: rgb(169, 177, 214),
            mauve: rgb(187, 154, 247),
            green: rgb(158, 206, 106),
            yellow: rgb(224, 175, 104),
            red: rgb(247, 118, 142),
            blue: rgb(122, 162, 247),
            teal: rgb(125, 207, 255),
            peach: rgb(255, 158, 100),
        }
    }

    fn tokyo_night_day() -> Self {
        Self {
            accent: rgb(46, 125, 233),
            panel_bg: rgb(225, 226, 231),
            surface0: rgb(196, 200, 218),
            surface1: rgb(168, 174, 203),
            surface_dim: rgb(210, 211, 218),
            overlay0: rgb(137, 144, 179),
            overlay1: rgb(104, 112, 154),
            text: rgb(55, 96, 191),
            subtext0: rgb(97, 114, 176),
            mauve: rgb(120, 71, 189),
            green: rgb(88, 117, 57),
            yellow: rgb(140, 108, 62),
            red: rgb(245, 42, 101),
            blue: rgb(46, 125, 233),
            teal: rgb(17, 140, 116),
            peach: rgb(177, 92, 0),
        }
    }

    fn dracula() -> Self {
        Self {
            accent: rgb(189, 147, 249), // purple
            panel_bg: rgb(40, 42, 54),
            surface0: rgb(68, 71, 90),
            surface1: rgb(98, 114, 164),
            surface_dim: rgb(40, 42, 54),
            overlay0: rgb(98, 114, 164),
            overlay1: rgb(130, 140, 180),
            text: rgb(248, 248, 242),
            subtext0: rgb(210, 210, 220),
            mauve: rgb(255, 121, 198), // pink
            green: rgb(80, 250, 123),
            yellow: rgb(241, 250, 140),
            red: rgb(255, 85, 85),
            blue: rgb(139, 233, 253), // cyan-ish
            teal: rgb(139, 233, 253),
            peach: rgb(255, 184, 108),
        }
    }

    fn nord() -> Self {
        Self {
            accent: rgb(136, 192, 208), // frost
            panel_bg: rgb(46, 52, 64),
            surface0: rgb(59, 66, 82),
            surface1: rgb(67, 76, 94),
            surface_dim: rgb(46, 52, 64),
            overlay0: rgb(76, 86, 106),
            overlay1: rgb(100, 110, 130),
            text: rgb(236, 239, 244),
            subtext0: rgb(216, 222, 233),
            mauve: rgb(180, 142, 173),
            green: rgb(163, 190, 140),
            yellow: rgb(235, 203, 139),
            red: rgb(191, 97, 106),
            blue: rgb(129, 161, 193),
            teal: rgb(143, 188, 187),
            peach: rgb(208, 135, 112),
        }
    }

    fn gruvbox() -> Self {
        Self {
            accent: rgb(215, 153, 33), // yellow
            panel_bg: rgb(40, 40, 40),
            surface0: rgb(60, 56, 54),
            surface1: rgb(80, 73, 69),
            surface_dim: rgb(40, 40, 40),
            overlay0: rgb(146, 131, 116),
            overlay1: rgb(168, 153, 132),
            text: rgb(235, 219, 178),
            subtext0: rgb(213, 196, 161),
            mauve: rgb(211, 134, 155),
            green: rgb(184, 187, 38),
            yellow: rgb(250, 189, 47),
            red: rgb(251, 73, 52),
            blue: rgb(131, 165, 152),
            teal: rgb(142, 192, 124),
            peach: rgb(254, 128, 25),
        }
    }

    fn gruvbox_light() -> Self {
        Self {
            accent: rgb(7, 102, 120),
            panel_bg: rgb(251, 241, 199),
            surface0: rgb(235, 219, 178),
            surface1: rgb(213, 196, 161),
            surface_dim: rgb(242, 229, 188),
            overlay0: rgb(146, 131, 116),
            overlay1: rgb(124, 111, 100),
            text: rgb(60, 56, 54),
            subtext0: rgb(80, 73, 69),
            mauve: rgb(143, 63, 113),
            green: rgb(121, 116, 14),
            yellow: rgb(181, 118, 20),
            red: rgb(157, 0, 6),
            blue: rgb(7, 102, 120),
            teal: rgb(66, 123, 88),
            peach: rgb(175, 58, 3),
        }
    }

    fn one_dark() -> Self {
        Self {
            accent: rgb(97, 175, 239), // blue
            panel_bg: rgb(40, 44, 52),
            surface0: rgb(44, 49, 58),
            surface1: rgb(62, 68, 81),
            surface_dim: rgb(40, 44, 52),
            overlay0: rgb(92, 99, 112),
            overlay1: rgb(115, 122, 135),
            text: rgb(171, 178, 191),
            subtext0: rgb(150, 156, 168),
            mauve: rgb(198, 120, 221),
            green: rgb(152, 195, 121),
            yellow: rgb(229, 192, 123),
            red: rgb(224, 108, 117),
            blue: rgb(97, 175, 239),
            teal: rgb(86, 182, 194),
            peach: rgb(209, 154, 102),
        }
    }

    fn one_light() -> Self {
        Self {
            accent: rgb(64, 120, 242),
            panel_bg: rgb(250, 250, 250),
            surface0: rgb(240, 240, 241),
            surface1: rgb(229, 229, 230),
            surface_dim: rgb(245, 245, 246),
            overlay0: rgb(160, 161, 167),
            overlay1: rgb(104, 107, 119),
            text: rgb(56, 58, 66),
            subtext0: rgb(104, 107, 119),
            mauve: rgb(166, 38, 164),
            green: rgb(80, 161, 79),
            yellow: rgb(193, 132, 1),
            red: rgb(228, 86, 73),
            blue: rgb(64, 120, 242),
            teal: rgb(1, 132, 188),
            peach: rgb(152, 104, 1),
        }
    }

    fn solarized() -> Self {
        Self {
            accent: rgb(38, 139, 210), // blue
            panel_bg: rgb(0, 43, 54),
            surface0: rgb(7, 54, 66),
            surface1: rgb(88, 110, 117),
            surface_dim: rgb(0, 43, 54),
            overlay0: rgb(88, 110, 117),
            overlay1: rgb(101, 123, 131),
            text: rgb(147, 161, 161),
            subtext0: rgb(131, 148, 150),
            mauve: rgb(211, 54, 130),
            green: rgb(133, 153, 0),
            yellow: rgb(181, 137, 0),
            red: rgb(220, 50, 47),
            blue: rgb(38, 139, 210),
            teal: rgb(42, 161, 152),
            peach: rgb(203, 75, 22),
        }
    }

    fn solarized_light() -> Self {
        Self {
            accent: rgb(38, 139, 210),
            panel_bg: rgb(253, 246, 227),
            surface0: rgb(238, 232, 213),
            surface1: rgb(147, 161, 161),
            surface_dim: rgb(238, 232, 213),
            overlay0: rgb(147, 161, 161),
            overlay1: rgb(88, 110, 117),
            text: rgb(101, 123, 131),
            subtext0: rgb(131, 148, 150),
            mauve: rgb(211, 54, 130),
            green: rgb(133, 153, 0),
            yellow: rgb(181, 137, 0),
            red: rgb(220, 50, 47),
            blue: rgb(38, 139, 210),
            teal: rgb(42, 161, 152),
            peach: rgb(203, 75, 22),
        }
    }

    fn kanagawa() -> Self {
        Self {
            accent: rgb(126, 156, 216), // blue
            panel_bg: rgb(31, 31, 40),
            surface0: rgb(42, 42, 55),
            surface1: rgb(54, 54, 70),
            surface_dim: rgb(31, 31, 40),
            overlay0: rgb(114, 113, 105),
            overlay1: rgb(135, 134, 125),
            text: rgb(220, 215, 186),
            subtext0: rgb(200, 195, 170),
            mauve: rgb(149, 127, 184),
            green: rgb(118, 148, 106),
            yellow: rgb(192, 163, 110),
            red: rgb(195, 64, 67),
            blue: rgb(126, 156, 216),
            teal: rgb(127, 180, 202),
            peach: rgb(255, 160, 102),
        }
    }

    fn kanagawa_lotus() -> Self {
        Self {
            accent: rgb(77, 105, 155),
            panel_bg: rgb(242, 236, 188),
            surface0: rgb(220, 213, 172),
            surface1: rgb(201, 203, 209),
            surface_dim: rgb(213, 206, 163),
            overlay0: rgb(160, 156, 172),
            overlay1: rgb(138, 137, 128),
            text: rgb(84, 84, 100),
            subtext0: rgb(67, 67, 108),
            mauve: rgb(98, 76, 131),
            green: rgb(111, 137, 78),
            yellow: rgb(119, 113, 63),
            red: rgb(200, 64, 83),
            blue: rgb(77, 105, 155),
            teal: rgb(78, 140, 162),
            peach: rgb(204, 109, 0),
        }
    }

    fn rose_pine() -> Self {
        Self {
            accent: rgb(196, 167, 231), // iris
            panel_bg: rgb(25, 23, 36),
            surface0: rgb(31, 29, 46),
            surface1: rgb(38, 35, 58),
            surface_dim: rgb(25, 23, 36),
            overlay0: rgb(110, 106, 134),
            overlay1: rgb(144, 140, 170),
            text: rgb(224, 222, 244),
            subtext0: rgb(200, 197, 220),
            mauve: rgb(196, 167, 231),
            green: rgb(49, 116, 143),
            yellow: rgb(246, 193, 119),
            red: rgb(235, 111, 146),
            blue: rgb(49, 116, 143),
            teal: rgb(156, 207, 216),
            peach: rgb(234, 154, 151),
        }
    }

    fn rose_pine_dawn() -> Self {
        Self {
            accent: rgb(144, 122, 169),
            panel_bg: rgb(250, 244, 237),
            surface0: rgb(242, 233, 225),
            surface1: rgb(255, 250, 243),
            surface_dim: rgb(242, 233, 225),
            overlay0: rgb(152, 147, 165),
            overlay1: rgb(121, 117, 147),
            text: rgb(70, 66, 97),
            subtext0: rgb(121, 117, 147),
            mauve: rgb(144, 122, 169),
            green: rgb(40, 105, 131),
            yellow: rgb(234, 157, 52),
            red: rgb(180, 99, 122),
            blue: rgb(40, 105, 131),
            teal: rgb(86, 148, 159),
            peach: rgb(215, 130, 126),
        }
    }

    fn vesper() -> Self {
        Self {
            accent: rgb(255, 199, 153),
            panel_bg: rgb(26, 26, 26),
            surface0: rgb(35, 35, 35),
            surface1: rgb(40, 40, 40),
            surface_dim: rgb(16, 16, 16),
            overlay0: rgb(92, 92, 92),
            overlay1: rgb(126, 126, 126),
            text: rgb(255, 255, 255),
            subtext0: rgb(160, 160, 160),
            mauve: rgb(255, 209, 168),
            green: rgb(153, 255, 228),
            yellow: rgb(255, 199, 153),
            red: rgb(255, 128, 128),
            blue: rgb(176, 176, 176),
            teal: rgb(102, 221, 204),
            peach: rgb(255, 199, 153),
        }
    }

    pub(crate) fn load(inherit: bool) -> Self {
        if !inherit {
            return Self::one_light();
        }
        let path = herdr_config_path();
        let Ok(s) = fs::read_to_string(path) else {
            return Self::catppuccin();
        };
        let Ok(v) = s.parse::<toml::Value>() else {
            return Self::catppuccin();
        };
        Self::from_herdr_config(&v)
    }

    fn from_herdr_config(v: &toml::Value) -> Self {
        let mut theme = Self::catppuccin();
        if let Some(name) = v
            .get("theme")
            .and_then(|x| x.as_table())
            .and_then(|x| x.get("name"))
            .and_then(|x| x.as_str())
            .and_then(Self::from_name)
        {
            theme = name;
        }
        if let Some(custom) = v
            .get("theme")
            .and_then(|x| x.as_table())
            .and_then(|x| x.get("custom"))
            .and_then(|x| x.as_table())
        {
            theme.apply_custom(custom);
        }
        theme
    }

    fn from_name(name: &str) -> Option<Self> {
        match normalize_theme_name(name).as_str() {
            "catppuccin" | "catppuccinmocha" => Some(Self::catppuccin()),
            "catppuccinlatte" | "latte" | "light" => Some(Self::catppuccin_latte()),
            "terminal" => Some(Self::terminal()),
            "tokyonight" => Some(Self::tokyo_night()),
            "tokyonightday" | "tokyoday" => Some(Self::tokyo_night_day()),
            "dracula" => Some(Self::dracula()),
            "nord" => Some(Self::nord()),
            "gruvbox" | "gruvboxdark" => Some(Self::gruvbox()),
            "gruvboxlight" => Some(Self::gruvbox_light()),
            "onedark" => Some(Self::one_dark()),
            "onelight" => Some(Self::one_light()),
            "solarized" | "solarizeddark" => Some(Self::solarized()),
            "solarizedlight" => Some(Self::solarized_light()),
            "kanagawa" => Some(Self::kanagawa()),
            "kanagawalotus" | "lotus" => Some(Self::kanagawa_lotus()),
            "rosepine" => Some(Self::rose_pine()),
            "rosepinedawn" | "dawn" => Some(Self::rose_pine_dawn()),
            "vesper" => Some(Self::vesper()),
            _ => None,
        }
    }

    fn apply_custom(&mut self, custom: &toml::map::Map<String, toml::Value>) {
        for (k, v) in custom {
            if let Some(c) = v.as_str().and_then(parse_color) {
                self.set(k, c);
            }
        }
    }

    fn set(&mut self, key: &str, color: Color) {
        match key {
            "accent" => self.accent = color,
            "panel_bg" => self.panel_bg = color,
            "surface0" => self.surface0 = color,
            "surface1" => self.surface1 = color,
            "surface_dim" => self.surface_dim = color,
            "overlay0" => self.overlay0 = color,
            "overlay1" => self.overlay1 = color,
            "text" => self.text = color,
            "subtext0" => self.subtext0 = color,
            "green" => self.green = color,
            "yellow" => self.yellow = color,
            "red" => self.red = color,
            "blue" => self.blue = color,
            "teal" => self.teal = color,
            "mauve" => self.mauve = color,
            "peach" => self.peach = color,
            _ => {}
        }
    }
}

fn herdr_config_path() -> PathBuf {
    if let Ok(xdg) = env::var("XDG_CONFIG_HOME") {
        return Path::new(&xdg).join("herdr/config.toml");
    }
    home().join(".config/herdr/config.toml")
}

fn normalize_theme_name(name: &str) -> String {
    name.chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .collect::<String>()
        .to_ascii_lowercase()
}

fn rgb(r: u8, g: u8, b: u8) -> Color {
    Color::Rgb(r, g, b)
}

fn parse_color(s: &str) -> Option<Color> {
    let s = s.trim();
    match s.to_ascii_lowercase().as_str() {
        "reset" | "default" | "none" | "transparent" => return Some(Color::Reset),
        _ => {}
    }
    if let Some(rgb) = s.strip_prefix("rgb(").and_then(|x| x.strip_suffix(')')) {
        let mut parts = rgb.split(',').map(|p| p.trim().parse::<u8>().ok());
        return Some(Color::Rgb(parts.next()??, parts.next()??, parts.next()??));
    }
    if let Some(hex) = s.strip_prefix('#') {
        if hex.len() == 6 {
            return Some(rgb(
                u8::from_str_radix(&hex[0..2], 16).ok()?,
                u8::from_str_radix(&hex[2..4], 16).ok()?,
                u8::from_str_radix(&hex[4..6], 16).ok()?,
            ));
        }
    }
    match s.to_ascii_lowercase().as_str() {
        "black" => Some(Color::Black),
        "red" => Some(Color::Red),
        "green" => Some(Color::Green),
        "yellow" => Some(Color::Yellow),
        "blue" => Some(Color::Blue),
        "magenta" => Some(Color::Magenta),
        "cyan" => Some(Color::Cyan),
        "gray" | "grey" => Some(Color::Gray),
        "darkgray" | "darkgrey" => Some(Color::DarkGray),
        "lightred" => Some(Color::LightRed),
        "lightgreen" => Some(Color::LightGreen),
        "lightyellow" => Some(Color::LightYellow),
        "lightblue" => Some(Color::LightBlue),
        "lightmagenta" => Some(Color::LightMagenta),
        "lightcyan" => Some(Color::LightCyan),
        "white" => Some(Color::White),
        _ => None,
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    fn theme_value(toml_src: &str) -> toml::Value {
        toml_src.parse::<toml::Value>().expect("valid toml")
    }

    #[test]
    fn uses_herdr_default_when_theme_name_is_unset() {
        let theme = Theme::from_herdr_config(&theme_value(""));

        assert_eq!(theme.panel_bg, rgb(24, 24, 37));
        assert_eq!(theme.accent, rgb(137, 180, 250));
    }

    #[test]
    fn inherits_rose_pine_dawn_and_custom_overrides() {
        let theme = Theme::from_herdr_config(&theme_value(
            r##"
            [theme]
            name = "rose_pine_dawn"

            [theme.custom]
            accent = "#ff00ff"
            panel_bg = "reset"
            "##,
        ));

        assert_eq!(theme.text, rgb(70, 66, 97));
        assert_eq!(theme.surface0, rgb(242, 233, 225));
        assert_eq!(theme.accent, rgb(255, 0, 255));
        assert_eq!(theme.panel_bg, Color::Reset);
    }

    #[test]
    fn resolves_all_herdr_0_7_4_themes() {
        for name in [
            "catppuccin",
            "catppuccin-latte",
            "terminal",
            "tokyo-night",
            "tokyo-night-day",
            "dracula",
            "nord",
            "gruvbox",
            "gruvbox-light",
            "one-dark",
            "one-light",
            "solarized",
            "solarized-light",
            "kanagawa",
            "kanagawa-lotus",
            "rose-pine",
            "rose-pine-dawn",
            "vesper",
        ] {
            assert!(
                Theme::from_name(name).is_some(),
                "theme should resolve: {name}"
            );
        }

        let dracula = Theme::from_herdr_config(&theme_value(
            r#"
            [theme]
            name = "dracula"
            "#,
        ));
        assert_eq!(dracula.panel_bg, rgb(40, 42, 54));
        assert_eq!(dracula.accent, rgb(189, 147, 249));
    }

    #[test]
    fn parses_rgb_named_and_reset_custom_colors() {
        let theme = Theme::from_herdr_config(&theme_value(
            r##"
            [theme]
            name = "terminal"

            [theme.custom]
            accent = "rgb(1, 2, 3)"
            green = "blue"
            peach = "transparent"
            "##,
        ));

        assert_eq!(theme.accent, rgb(1, 2, 3));
        assert_eq!(theme.green, Color::Blue);
        assert_eq!(theme.peach, Color::Reset);
    }
}
