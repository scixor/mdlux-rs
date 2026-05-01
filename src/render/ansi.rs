use crate::theme::{Color, Style};

pub fn apply_style(input: &str, style: Style, ansi: bool) -> String {
    if !ansi {
        return input.to_string();
    }
    let mut parts: Vec<String> = Vec::new();
    if style.bold {
        parts.push("1".to_string());
    }
    if style.dim {
        parts.push("2".to_string());
    }
    if style.italic {
        parts.push("3".to_string());
    }
    if style.underline {
        parts.push("4".to_string());
    }
    if let Some(fg) = style.fg {
        match fg {
            Color::Ansi(v) => parts.push(format!("38;5;{v}")),
            Color::Rgb(r, g, b) => parts.push(format!("38;2;{r};{g};{b}")),
        }
    }
    if let Some(bg) = style.bg {
        match bg {
            Color::Ansi(v) => parts.push(format!("48;5;{v}")),
            Color::Rgb(r, g, b) => parts.push(format!("48;2;{r};{g};{b}")),
        }
    }
    if parts.is_empty() {
        return input.to_string();
    }
    format!("\u{1b}[{}m{}\u{1b}[0m", parts.join(";"), input)
}
