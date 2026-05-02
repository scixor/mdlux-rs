mod kanagawa;
mod nord;
pub mod syntect;

use std::str::FromStr;

use ::syntect::highlighting::{Color, FontStyle, ScopeSelectors, StyleModifier, ThemeItem};

pub(super) fn scope_style(
    scope: &str,
    foreground: Option<Color>,
    background: Option<Color>,
    font_style: Option<FontStyle>,
) -> ThemeItem {
    ThemeItem {
        scope: ScopeSelectors::from_str(scope).expect("valid scope selector"),
        style: StyleModifier {
            foreground,
            background,
            font_style,
        },
    }
}

const fn rgb(r: u8, g: u8, b: u8) -> Color {
    Color { r, g, b, a: 0xFF }
}
