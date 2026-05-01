use super::{Color, Style, Theme};

pub fn default_theme_name() -> &'static str {
    "ansi"
}

pub fn list_theme_names() -> &'static [&'static str] {
    &["ansi", "dark", "light", "nord", "gruvbox"]
}

pub fn find_theme(name: &str) -> Option<&'static Theme> {
    match name {
        "ansi" => Some(&ANSI),
        "dark" => Some(&DARK),
        "light" => Some(&LIGHT),
        "nord" => Some(&NORD),
        "gruvbox" => Some(&GRUVBOX),
        _ => None,
    }
}

const fn s(fg: Option<Color>, bold: bool, italic: bool, underline: bool, dim: bool) -> Style {
    Style {
        fg,
        bg: None,
        bold,
        italic,
        underline,
        dim,
    }
}

static ANSI: Theme = Theme {
    name: "ansi",
    heading1: s(Some(Color::Ansi(33)), true, false, true, false),
    heading2: s(Some(Color::Ansi(39)), true, false, false, false),
    heading3: s(Some(Color::Ansi(45)), true, false, false, false),
    heading4: s(Some(Color::Ansi(37)), true, false, false, true),
    text: s(None, false, false, false, false),
    emphasis: s(None, false, true, false, false),
    strong: s(None, true, false, false, false),
    inline_code: s(Some(Color::Ansi(36)), false, false, false, false),
    code: s(Some(Color::Ansi(250)), false, false, false, false),
    quote_marker: s(Some(Color::Ansi(244)), false, false, false, false),
    link_label: s(Some(Color::Ansi(39)), false, false, true, false),
    link_url: s(Some(Color::Ansi(244)), false, false, false, false),
    rule: s(Some(Color::Ansi(244)), false, false, false, false),
    table_header: s(Some(Color::Ansi(33)), true, false, false, false),
    table_border: s(Some(Color::Ansi(244)), false, false, false, false),
    list_marker: s(Some(Color::Ansi(39)), true, false, false, false),
};

static DARK: Theme = Theme {
    name: "dark",
    heading1: s(Some(Color::Rgb(224, 108, 117)), true, false, true, false),
    heading2: s(Some(Color::Rgb(97, 175, 239)), true, false, false, false),
    heading3: s(Some(Color::Rgb(198, 120, 221)), true, false, false, false),
    heading4: s(Some(Color::Rgb(171, 178, 191)), true, false, false, true),
    text: s(Some(Color::Rgb(171, 178, 191)), false, false, false, false),
    emphasis: s(None, false, true, false, false),
    strong: s(None, true, false, false, false),
    inline_code: s(Some(Color::Rgb(86, 182, 194)), false, false, false, false),
    code: s(Some(Color::Rgb(171, 178, 191)), false, false, false, false),
    quote_marker: s(Some(Color::Rgb(92, 99, 112)), false, false, false, false),
    link_label: s(Some(Color::Rgb(97, 175, 239)), false, false, true, false),
    link_url: s(Some(Color::Rgb(92, 99, 112)), false, false, false, false),
    rule: s(Some(Color::Rgb(92, 99, 112)), false, false, false, false),
    table_header: s(Some(Color::Rgb(224, 108, 117)), true, false, false, false),
    table_border: s(Some(Color::Rgb(92, 99, 112)), false, false, false, false),
    list_marker: s(Some(Color::Rgb(97, 175, 239)), true, false, false, false),
};

static LIGHT: Theme = Theme {
    name: "light",
    heading1: s(Some(Color::Rgb(196, 26, 22)), true, false, true, false),
    heading2: s(Some(Color::Rgb(0, 64, 175)), true, false, false, false),
    heading3: s(Some(Color::Rgb(111, 66, 193)), true, false, false, false),
    heading4: s(Some(Color::Rgb(60, 60, 60)), true, false, false, true),
    text: s(Some(Color::Rgb(30, 30, 30)), false, false, false, false),
    emphasis: s(None, false, true, false, false),
    strong: s(None, true, false, false, false),
    inline_code: s(Some(Color::Rgb(10, 132, 255)), false, false, false, false),
    code: s(Some(Color::Rgb(35, 35, 35)), false, false, false, false),
    quote_marker: s(Some(Color::Rgb(140, 140, 140)), false, false, false, false),
    link_label: s(Some(Color::Rgb(0, 64, 175)), false, false, true, false),
    link_url: s(Some(Color::Rgb(100, 100, 100)), false, false, false, false),
    rule: s(Some(Color::Rgb(160, 160, 160)), false, false, false, false),
    table_header: s(Some(Color::Rgb(196, 26, 22)), true, false, false, false),
    table_border: s(Some(Color::Rgb(160, 160, 160)), false, false, false, false),
    list_marker: s(Some(Color::Rgb(0, 64, 175)), true, false, false, false),
};

static NORD: Theme = Theme {
    name: "nord",
    heading1: s(Some(Color::Rgb(191, 97, 106)), true, false, true, false),
    heading2: s(Some(Color::Rgb(129, 161, 193)), true, false, false, false),
    heading3: s(Some(Color::Rgb(180, 142, 173)), true, false, false, false),
    heading4: s(Some(Color::Rgb(216, 222, 233)), true, false, false, true),
    text: s(Some(Color::Rgb(216, 222, 233)), false, false, false, false),
    emphasis: s(None, false, true, false, false),
    strong: s(None, true, false, false, false),
    inline_code: s(Some(Color::Rgb(136, 192, 208)), false, false, false, false),
    code: s(Some(Color::Rgb(216, 222, 233)), false, false, false, false),
    quote_marker: s(Some(Color::Rgb(76, 86, 106)), false, false, false, false),
    link_label: s(Some(Color::Rgb(129, 161, 193)), false, false, true, false),
    link_url: s(Some(Color::Rgb(94, 129, 172)), false, false, false, false),
    rule: s(Some(Color::Rgb(76, 86, 106)), false, false, false, false),
    table_header: s(Some(Color::Rgb(191, 97, 106)), true, false, false, false),
    table_border: s(Some(Color::Rgb(76, 86, 106)), false, false, false, false),
    list_marker: s(Some(Color::Rgb(129, 161, 193)), true, false, false, false),
};

static GRUVBOX: Theme = Theme {
    name: "gruvbox",
    heading1: s(Some(Color::Rgb(251, 73, 52)), true, false, true, false),
    heading2: s(Some(Color::Rgb(131, 165, 152)), true, false, false, false),
    heading3: s(Some(Color::Rgb(211, 134, 155)), true, false, false, false),
    heading4: s(Some(Color::Rgb(235, 219, 178)), true, false, false, true),
    text: s(Some(Color::Rgb(235, 219, 178)), false, false, false, false),
    emphasis: s(None, false, true, false, false),
    strong: s(None, true, false, false, false),
    inline_code: s(Some(Color::Rgb(142, 192, 124)), false, false, false, false),
    code: s(Some(Color::Rgb(235, 219, 178)), false, false, false, false),
    quote_marker: s(Some(Color::Rgb(146, 131, 116)), false, false, false, false),
    link_label: s(Some(Color::Rgb(131, 165, 152)), false, false, true, false),
    link_url: s(Some(Color::Rgb(168, 153, 132)), false, false, false, false),
    rule: s(Some(Color::Rgb(146, 131, 116)), false, false, false, false),
    table_header: s(Some(Color::Rgb(251, 73, 52)), true, false, false, false),
    table_border: s(Some(Color::Rgb(146, 131, 116)), false, false, false, false),
    list_marker: s(Some(Color::Rgb(131, 165, 152)), true, false, false, false),
};
