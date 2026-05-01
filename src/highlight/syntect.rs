use syntect::easy::HighlightLines;
use syntect::highlighting::ThemeSet;
use syntect::parsing::{SyntaxReference, SyntaxSet};
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

        if is_markdown_lang(lang) {
            return self.highlight_markdown_with_embedded_fences(code, theme);
        }

        let syntax = choose_syntax(&self.syntax_set, lang);
        highlight_with_syntax(code, &self.syntax_set, syntax, theme)
    }

    fn highlight_markdown_with_embedded_fences(
        &self,
        code: &str,
        theme: &syntect::highlighting::Theme,
    ) -> String {
        let markdown_syntax = choose_syntax(&self.syntax_set, Some("markdown"));
        let mut markdown_highlighter = HighlightLines::new(markdown_syntax, theme);

        let mut out = String::new();
        let mut fence: Option<FenceState<'_>> = None;

        for line in LinesWithEndings::from(code) {
            let trimmed = line.trim_start();

            if let Some(active) = fence.as_mut() {
                if is_fence_terminator(trimmed, active.marker, active.min_len) {
                    out.push_str(&highlight_single_line(
                        &mut markdown_highlighter,
                        &self.syntax_set,
                        line,
                    ));
                    fence = None;
                    continue;
                }

                if let Some(inner_highlighter) = active.inner.as_mut() {
                    out.push_str(&highlight_single_line(
                        inner_highlighter,
                        &self.syntax_set,
                        line,
                    ));
                } else {
                    out.push_str(line);
                }
                continue;
            }

            if let Some((marker, len, inner_lang)) = parse_fence_start(trimmed) {
                out.push_str(&highlight_single_line(
                    &mut markdown_highlighter,
                    &self.syntax_set,
                    line,
                ));

                let inner = inner_lang.map(|l| {
                    let syntax = choose_syntax(&self.syntax_set, Some(l));
                    HighlightLines::new(syntax, theme)
                });

                fence = Some(FenceState {
                    marker,
                    min_len: len,
                    inner,
                });
                continue;
            }

            out.push_str(&highlight_single_line(
                &mut markdown_highlighter,
                &self.syntax_set,
                line,
            ));
        }

        out
    }
}

struct FenceState<'a> {
    marker: char,
    min_len: usize,
    inner: Option<HighlightLines<'a>>,
}

fn highlight_with_syntax(
    code: &str,
    syntax_set: &SyntaxSet,
    syntax: &SyntaxReference,
    theme: &syntect::highlighting::Theme,
) -> String {
    let mut highlighter = HighlightLines::new(syntax, theme);
    let mut out = String::new();
    for line in LinesWithEndings::from(code) {
        out.push_str(&highlight_single_line(&mut highlighter, syntax_set, line));
    }
    out
}

fn highlight_single_line(
    highlighter: &mut HighlightLines<'_>,
    syntax_set: &SyntaxSet,
    line: &str,
) -> String {
    if let Ok(ranges) = highlighter.highlight_line(line, syntax_set) {
        let mut out = as_24_bit_terminal_escaped(&ranges, false);
        out.push_str("\u{1b}[0m");
        out
    } else {
        line.to_string()
    }
}

fn is_markdown_lang(lang: Option<&str>) -> bool {
    let Some(lang) = lang else {
        return false;
    };
    matches!(normalize_lang(lang).as_str(), "md" | "markdown")
}

fn parse_fence_start(line: &str) -> Option<(char, usize, Option<&str>)> {
    let marker = line.chars().next()?;
    if marker != '`' && marker != '~' {
        return None;
    }

    let count = line.chars().take_while(|c| *c == marker).count();
    if count < 3 {
        return None;
    }

    let rest = line[count..].trim();
    if rest.is_empty() {
        return Some((marker, count, None));
    }

    let token = rest
        .split_whitespace()
        .next()
        .unwrap_or_default()
        .trim_matches('{')
        .trim_matches('}')
        .trim_start_matches('.');

    if token.is_empty() {
        Some((marker, count, None))
    } else {
        Some((marker, count, Some(token)))
    }
}

fn is_fence_terminator(line: &str, marker: char, min_len: usize) -> bool {
    if !line.starts_with(marker) {
        return false;
    }

    let count = line.chars().take_while(|c| *c == marker).count();
    count >= min_len && line[count..].trim().is_empty()
}

fn choose_syntax<'a>(ss: &'a SyntaxSet, lang: Option<&str>) -> &'a SyntaxReference {
    let Some(raw_lang) = lang.map(str::trim).filter(|l| !l.is_empty()) else {
        return ss.find_syntax_plain_text();
    };

    let normalized = normalize_lang(raw_lang);

    if let Some(s) = lookup_syntax(ss, raw_lang) {
        return s;
    }
    if normalized != raw_lang
        && let Some(s) = lookup_syntax(ss, &normalized)
    {
        return s;
    }

    for alias in aliases(&normalized) {
        if let Some(s) = lookup_syntax(ss, alias) {
            return s;
        }
    }

    ss.find_syntax_plain_text()
}

fn lookup_syntax<'a>(ss: &'a SyntaxSet, candidate: &str) -> Option<&'a SyntaxReference> {
    ss.find_syntax_by_token(candidate)
        .or_else(|| ss.find_syntax_by_extension(candidate))
        .or_else(|| ss.find_syntax_by_name(candidate))
}

fn normalize_lang(lang: &str) -> String {
    lang.trim_start_matches('.').to_ascii_lowercase()
}

fn aliases(lang: &str) -> &'static [&'static str] {
    match lang {
        "typescript" => &[
            "typescript",
            "TypeScript",
            "javascript",
            "JavaScript",
            "js",
            "rust",
        ],
        "ts" => &[
            "typescript",
            "TypeScript",
            "javascript",
            "JavaScript",
            "js",
            "rust",
        ],
        "tsx" => &[
            "tsx",
            "TypeScriptReact",
            "TypeScript React",
            "javascript",
            "JavaScript",
            "js",
            "rust",
        ],
        "js" => &["javascript", "JavaScript"],
        "jsx" => &["jsx", "JavaScript (Babel)"],
        "rs" => &["rust", "Rust"],
        "sh" | "shell" | "zsh" => &["bash", "Bourne Again Shell (bash)", "Shell-Unix-Generic"],
        "yml" => &["yaml", "YAML"],
        "zig" => &["zig", "Zig", "rust", "c", "cpp", "go"],
        _ => &[],
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
    use super::{Highlighter, choose_syntax, rendered, rendered_bash, resolved_theme_name};
    use std::collections::BTreeSet;
    use syntect::parsing::SyntaxSet;

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
    fn recognizes_typescript_and_zig_syntax_tokens() {
        let ss = SyntaxSet::load_defaults_newlines();
        let ts = &choose_syntax(&ss, Some("ts")).name;
        let typescript = &choose_syntax(&ss, Some("typescript")).name;
        let zig = &choose_syntax(&ss, Some("zig")).name;
        let plain = &ss.find_syntax_plain_text().name;
        assert_ne!(ts, plain, "ts should not resolve to plain text");
        assert_ne!(
            typescript, plain,
            "typescript should not resolve to plain text"
        );
        assert_ne!(zig, plain, "zig should not resolve to plain text");
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
    fn markdown_fence_highlights_inner_block_language() {
        let h = Highlighter::new("kanagawa");
        let code = "```ts\nconst x: number = 1\n```\n";

        let md = h.highlight_code(code, Some("md"));
        let plain = h.highlight_code(code, Some("txt"));

        assert_ne!(
            md, plain,
            "inner fenced language should affect highlighting"
        );
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
