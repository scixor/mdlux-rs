pub mod builtin;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Color {
    Ansi(u8),
    Rgb(u8, u8, u8),
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Style {
    pub fg: Option<Color>,
    pub bg: Option<Color>,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub dim: bool,
}

#[derive(Debug, Clone)]
pub struct Theme {
    pub name: &'static str,
    pub heading1: Style,
    pub heading2: Style,
    pub heading3: Style,
    pub heading4: Style,
    pub text: Style,
    pub emphasis: Style,
    pub strong: Style,
    pub inline_code: Style,
    pub code: Style,
    pub quote_marker: Style,
    pub link_label: Style,
    pub link_url: Style,
    pub rule: Style,
    pub table_header: Style,
    pub table_border: Style,
    pub list_marker: Style,
}
