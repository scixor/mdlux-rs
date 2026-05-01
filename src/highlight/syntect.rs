use syntect::easy::HighlightLines;
use syntect::highlighting::ThemeSet;
use syntect::parsing::SyntaxSet;
use syntect::util::{LinesWithEndings, as_24_bit_terminal_escaped};

const KANAGAWA_CUSTOM_NAME: &str = "kanagawa-custom";
const NORD_CUSTOM_NAME: &str = "nord-custom";

pub struct Highlighter {
    syntax_set: SyntaxSet,
    theme_set: ThemeSet,
    theme_name: String,
}

impl Highlighter {
    pub fn new(theme_name: &str) -> Self {
        let theme_set = build_theme_set();
        let theme_name = choose_theme_name(theme_name, &theme_set);
        Self {
            syntax_set: SyntaxSet::load_defaults_newlines(),
            theme_set,
            theme_name,
        }
    }

    pub fn highlight_code(&self, code: &str, lang: Option<&str>) -> String {
        let theme = self
            .theme_set
            .themes
            .get(&self.theme_name)
            .or_else(|| self.theme_set.themes.get("base16-ocean.dark"));
        let Some(theme) = theme else {
            return code.to_string();
        };

        let syntax = lang
            .and_then(|l| self.syntax_set.find_syntax_by_token(l))
            .unwrap_or_else(|| self.syntax_set.find_syntax_plain_text());

        let mut h = HighlightLines::new(syntax, theme);
        let mut out = String::new();
        for line in LinesWithEndings::from(code) {
            if let Ok(ranges) = h.highlight_line(line, &self.syntax_set) {
                out.push_str(&as_24_bit_terminal_escaped(&ranges, false));
                out.push_str("\u{1b}[0m");
            } else {
                out.push_str(line);
            }
        }
        out
    }
}

fn choose_theme_name(theme: &str, ts: &ThemeSet) -> String {
    let candidates: &[&str] = match theme {
        "ansi" => &[
            "base16-eighties.dark",
            "base16-ocean.dark",
            "InspiredGitHub",
        ],
        "dark" => &[
            "base16-ocean.dark",
            "base16-eighties.dark",
            "InspiredGitHub",
        ],
        "light" => &["InspiredGitHub", "base16-ocean.light", "Solarized (light)"],
        "nord" => &[
            NORD_CUSTOM_NAME,
            "Nord",
            "base16-ocean.dark",
            "Solarized (dark)",
        ],
        "gruvbox" => &["Gruvbox dark", "base16-mocha.dark", "base16-eighties.dark"],
        "kanagawa" => &[
            KANAGAWA_CUSTOM_NAME,
            "Solarized (dark)",
            "base16-ocean.dark",
            "base16-eighties.dark",
        ],
        _ => &["base16-ocean.dark", "InspiredGitHub"],
    };

    for name in candidates {
        if ts.themes.contains_key(*name) {
            return (*name).to_string();
        }
    }
    ts.themes
        .keys()
        .next()
        .cloned()
        .unwrap_or_else(|| "base16-ocean.dark".to_string())
}

fn build_theme_set() -> ThemeSet {
    let mut theme_set = ThemeSet::load_defaults();
    theme_set
        .themes
        .insert(KANAGAWA_CUSTOM_NAME.to_string(), super::kanagawa::theme());
    theme_set
        .themes
        .insert(NORD_CUSTOM_NAME.to_string(), super::nord::theme());
    theme_set
}

#[cfg(test)]
fn resolved_theme_name(theme: &str) -> String {
    let ts = build_theme_set();
    choose_theme_name(theme, &ts)
}

#[cfg(test)]
fn rendered(theme: &str) -> String {
    let h = Highlighter::new(theme);
    h.highlight_code("fn main() {}\n", Some("rust"))
}

#[cfg(test)]
fn rendered_bash(theme: &str) -> String {
    let code = "if [ -n \"$HOME\" ]; then\n  echo \"hi\" | sed 's/h/H/' > out.txt\nfi\n";
    let h = Highlighter::new(theme);
    h.highlight_code(code, Some("bash"))
}

#[cfg(test)]
mod tests {
    use super::{Highlighter, rendered, rendered_bash, resolved_theme_name};
    use std::collections::BTreeSet;

    #[test]
    fn highlights_or_falls_back() {
        let h = Highlighter::new("ansi");
        let out = h.highlight_code("fn main() {}\n", Some("rust"));
        assert!(!out.is_empty());
        assert!(out.contains("\u{1b}[0m"));
    }

    #[test]
    fn picks_distinct_theme_for_nord_vs_ansi() {
        let ansi = resolved_theme_name("ansi");
        let nord = resolved_theme_name("nord");
        assert_ne!(ansi, nord);
    }

    #[test]
    fn picks_custom_nord_theme() {
        let nord = resolved_theme_name("nord");
        assert_eq!(nord, "nord-custom");
    }

    #[test]
    fn output_differs_for_light_vs_dark_family() {
        let light = rendered("light");
        let dark = rendered("dark");
        assert_ne!(light, dark);
    }

    #[test]
    fn picks_custom_kanagawa_theme() {
        let kanagawa = resolved_theme_name("kanagawa");
        assert_eq!(kanagawa, "kanagawa-custom");
    }

    #[test]
    fn output_differs_for_kanagawa_vs_ansi() {
        let kanagawa = rendered("kanagawa");
        let ansi = rendered("ansi");
        assert_ne!(kanagawa, ansi);
    }

    #[test]
    fn kanagawa_bash_uses_multiple_truecolor_groups() {
        let out = rendered_bash("kanagawa");
        let mut colors = BTreeSet::new();
        for part in out.split("\u{1b}[") {
            if let Some(code) = part.strip_prefix("38;2;")
                && let Some((rgb, _rest)) = code.split_once('m')
            {
                colors.insert(rgb.to_string());
            }
        }
        assert!(
            colors.len() >= 4,
            "expected more color variety, got {colors:?}"
        );
    }
}
