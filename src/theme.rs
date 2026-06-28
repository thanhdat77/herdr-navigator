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
            accent: rgb(137, 180, 250),
            panel_bg: rgb(24, 24, 37),
            surface0: rgb(49, 50, 68),
            surface1: rgb(69, 71, 90),
            surface_dim: rgb(30, 30, 46),
            overlay0: rgb(108, 112, 134),
            overlay1: rgb(127, 132, 156),
            text: rgb(205, 214, 244),
            subtext0: rgb(166, 173, 200),
            green: rgb(166, 227, 161),
            yellow: rgb(249, 226, 175),
            red: rgb(243, 139, 168),
            blue: rgb(137, 180, 250),
            teal: rgb(148, 226, 213),
            mauve: rgb(203, 166, 247),
            peach: rgb(250, 179, 135),
        }
    }

    fn one_light() -> Self {
        Self {
            accent: rgb(97, 175, 239),
            panel_bg: rgb(250, 250, 250),
            surface0: rgb(232, 232, 232),
            surface1: rgb(240, 240, 240),
            surface_dim: rgb(244, 244, 244),
            overlay0: rgb(160, 161, 167),
            overlay1: rgb(128, 129, 135),
            text: rgb(56, 58, 66),
            subtext0: rgb(105, 108, 119),
            green: rgb(80, 161, 79),
            yellow: rgb(193, 132, 1),
            red: rgb(228, 86, 73),
            blue: rgb(1, 132, 188),
            teal: rgb(9, 151, 152),
            mauve: rgb(166, 38, 164),
            peach: rgb(152, 104, 1),
        }
    }

    fn rose_pine() -> Self {
        Self {
            accent: rgb(196, 167, 231),
            panel_bg: rgb(25, 23, 36),
            surface0: rgb(31, 29, 46),
            surface1: rgb(38, 35, 58),
            surface_dim: rgb(31, 29, 46),
            overlay0: rgb(110, 106, 134),
            overlay1: rgb(144, 140, 170),
            text: rgb(224, 222, 244),
            subtext0: rgb(144, 140, 170),
            green: rgb(67, 153, 145),
            yellow: rgb(246, 193, 119),
            red: rgb(235, 111, 146),
            blue: rgb(144, 122, 169),
            teal: rgb(86, 148, 159),
            mauve: rgb(196, 167, 231),
            peach: rgb(246, 193, 119),
        }
    }

    fn rose_pine_dawn() -> Self {
        Self {
            accent: rgb(144, 122, 169),
            panel_bg: rgb(250, 244, 237),
            surface0: rgb(223, 218, 217),
            surface1: rgb(242, 233, 225),
            surface_dim: rgb(244, 237, 232),
            overlay0: rgb(152, 147, 165),
            overlay1: rgb(121, 117, 147),
            text: rgb(87, 82, 121),
            subtext0: rgb(121, 117, 147),
            green: rgb(40, 105, 131),
            yellow: rgb(234, 157, 52),
            red: rgb(180, 99, 122),
            blue: rgb(86, 148, 159),
            teal: rgb(86, 148, 159),
            mauve: rgb(144, 122, 169),
            peach: rgb(234, 157, 52),
        }
    }

    fn terminal() -> Self {
        Self {
            accent: ansi(12),
            panel_bg: Color::Reset,
            surface0: ansi(8),
            surface1: ansi(0),
            surface_dim: ansi(0),
            overlay0: ansi(8),
            overlay1: ansi(7),
            text: ansi(7),
            subtext0: ansi(8),
            green: ansi(10),
            yellow: ansi(11),
            red: ansi(9),
            blue: ansi(12),
            teal: ansi(14),
            mauve: ansi(13),
            peach: ansi(208),
        }
    }

    pub(crate) fn load(inherit: bool) -> Self {
        if !inherit {
            return Self::one_light();
        }
        let path = herdr_config_path();
        let Ok(s) = fs::read_to_string(path) else {
            return Self::one_light();
        };
        let Ok(v) = s.parse::<toml::Value>() else {
            return Self::one_light();
        };
        Self::from_herdr_config(&v)
    }

    fn from_herdr_config(v: &toml::Value) -> Self {
        let mut theme = Self::one_light();
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
            "terminal" => Some(Self::terminal()),
            "onelight" => Some(Self::one_light()),
            "catppuccin" => Some(Self::catppuccin()),
            "rosepine" => Some(Self::rose_pine()),
            "rosepinedawn" => Some(Self::rose_pine_dawn()),
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

fn ansi(i: u8) -> Color {
    Color::Indexed(i)
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

        assert_eq!(theme.text, rgb(87, 82, 121));
        assert_eq!(theme.surface0, rgb(223, 218, 217));
        assert_eq!(theme.accent, rgb(255, 0, 255));
        assert_eq!(theme.panel_bg, Color::Reset);
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
