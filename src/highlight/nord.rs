use std::str::FromStr;
use syntect::highlighting::{
    Color, FontStyle, ScopeSelectors, StyleModifier, Theme, ThemeItem, ThemeSettings,
};

pub(super) fn theme() -> Theme {
    Theme {
        name: Some("Nord Custom".to_string()),
        author: Some("mdlux".to_string()),
        settings: ThemeSettings {
            foreground: Some(rgb(216, 222, 233)),
            background: Some(rgb(46, 52, 64)),
            caret: Some(rgb(229, 233, 240)),
            line_highlight: Some(rgb(59, 66, 82)),
            selection: Some(rgb(76, 86, 106)),
            selection_foreground: Some(rgb(236, 239, 244)),
            gutter_foreground: Some(rgb(136, 192, 208)),
            ..ThemeSettings::default()
        },
        scopes: vec![
            scope_style(
                "comment",
                Some(rgb(129, 161, 193)),
                None,
                Some(FontStyle::ITALIC),
            ),
            scope_style("string", Some(rgb(163, 190, 140)), None, None),
            scope_style(
                "constant, constant.numeric",
                Some(rgb(180, 142, 173)),
                None,
                None,
            ),
            scope_style("keyword", Some(rgb(129, 161, 193)), None, None),
            scope_style(
                "keyword.control, storage.type",
                Some(rgb(129, 161, 193)),
                None,
                Some(FontStyle::BOLD),
            ),
            scope_style("entity.name.function", Some(rgb(136, 192, 208)), None, None),
            scope_style(
                "variable.function, variable.function.shell",
                Some(rgb(136, 192, 208)),
                None,
                Some(FontStyle::BOLD),
            ),
            scope_style(
                "entity.name.command.shell, meta.statement.command.name.basic.shell, meta.statement.command.name.quoted.shell",
                Some(rgb(143, 188, 187)),
                None,
                Some(FontStyle::BOLD),
            ),
            scope_style(
                "meta.function-call.shell, meta.function-call.bash, entity.name.function.shell, entity.name.function.bash, support.function.builtin.shell, support.function.builtin.bash, support.function.builtin.zsh",
                Some(rgb(136, 192, 208)),
                None,
                None,
            ),
            scope_style(
                "support.function, support.function.builtin",
                Some(rgb(143, 188, 187)),
                None,
                None,
            ),
            scope_style("variable", Some(rgb(216, 222, 233)), None, None),
            scope_style("variable.language", Some(rgb(208, 135, 112)), None, None),
            scope_style(
                "punctuation, operator",
                Some(rgb(229, 233, 240)),
                None,
                None,
            ),
            scope_style(
                "entity.name.type, support.type",
                Some(rgb(143, 188, 187)),
                None,
                None,
            ),
            scope_style(
                "invalid",
                Some(rgb(191, 97, 106)),
                Some(rgb(67, 76, 94)),
                None,
            ),
            scope_style("markup.bold", None, None, Some(FontStyle::BOLD)),
            scope_style("markup.italic", None, None, Some(FontStyle::ITALIC)),
            scope_style(
                "markup.heading",
                Some(rgb(136, 192, 208)),
                None,
                Some(FontStyle::BOLD),
            ),
            scope_style("markup.link", Some(rgb(129, 161, 193)), None, None),
            scope_style(
                "markup.inserted",
                Some(rgb(163, 190, 140)),
                Some(rgb(59, 66, 82)),
                None,
            ),
            scope_style(
                "markup.deleted",
                Some(rgb(191, 97, 106)),
                Some(rgb(67, 76, 94)),
                None,
            ),
        ],
    }
}

fn scope_style(
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
