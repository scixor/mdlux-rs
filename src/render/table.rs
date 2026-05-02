use crate::markdown::ast::Alignment;
use crate::theme::Style;
use crate::util::width::visible_width;

use super::ansi::apply_style;

pub struct RenderedTable {
    pub headers: Vec<String>,
    pub rows: Vec<Vec<String>>,
    pub aligns: Vec<Alignment>,
}

pub fn render_table(
    table: &RenderedTable,
    width: usize,
    ansi: bool,
    header_style: Style,
    border_style: Style,
) -> String {
    debug_assert!(!table.headers.is_empty(), "table must have headers");
    let cols = table.headers.len();
    let mut col_widths = vec![1usize; cols];

    for (i, header) in table.headers.iter().enumerate() {
        col_widths[i] = col_widths[i].max(visible_width(header));
    }
    for row in &table.rows {
        for (i, cell) in row.iter().enumerate().take(cols) {
            col_widths[i] = col_widths[i].max(visible_width(cell));
        }
    }

    let total = table_total_width(&col_widths);
    if total > width {
        return render_table_fallback(table);
    }

    let border = apply_style(&render_border(&col_widths), border_style, ansi);
    let mut out = String::new();
    out.push_str(&border);
    out.push('\n');
    out.push_str(&render_row(
        &table.headers,
        &col_widths,
        &table.aligns,
        ansi,
        header_style,
        border_style,
    ));
    out.push('\n');
    out.push_str(&border);
    out.push('\n');
    for row in &table.rows {
        out.push_str(&render_row(
            row,
            &col_widths,
            &table.aligns,
            ansi,
            Style::default(),
            border_style,
        ));
        out.push('\n');
    }
    out.push_str(&border);
    out
}

fn render_table_fallback(table: &RenderedTable) -> String {
    let mut out = String::new();
    for row in &table.rows {
        for (i, header) in table.headers.iter().enumerate() {
            let value = row.get(i).map(|s| s.as_str()).unwrap_or("");
            out.push_str(&format!("{header}: {value}\n"));
        }
        out.push('\n');
    }
    if table.rows.is_empty() {
        for header in &table.headers {
            out.push_str(&format!("{header}: \n"));
        }
    }
    out.trim_end().to_string()
}

fn render_row(
    row: &[String],
    widths: &[usize],
    aligns: &[Alignment],
    ansi: bool,
    row_style: Style,
    border_style: Style,
) -> String {
    let mut out = String::new();
    out.push_str(&apply_style("|", border_style, ansi));
    for (idx, width) in widths.iter().enumerate() {
        let cell = row.get(idx).map(|s| s.as_str()).unwrap_or("");
        let align = aligns.get(idx).copied().unwrap_or(Alignment::Left);
        let padded = pad_visible(cell, *width, align);
        let styled_cell = apply_style(&format!(" {padded} "), row_style, ansi);
        out.push_str(&styled_cell);
        out.push_str(&apply_style("|", border_style, ansi));
    }
    out
}

fn render_border(widths: &[usize]) -> String {
    let mut out = String::new();
    out.push('+');
    for width in widths {
        out.push_str(&"-".repeat(width + 2));
        out.push('+');
    }
    out
}

fn table_total_width(widths: &[usize]) -> usize {
    widths.iter().sum::<usize>() + widths.len() * 3 + 1
}

pub fn pad_visible(s: &str, width: usize, align: Alignment) -> String {
    let w = visible_width(s);
    if w >= width {
        return s.to_string();
    }
    let diff = width - w;
    match align {
        Alignment::Right => format!("{}{}", " ".repeat(diff), s),
        Alignment::Center => {
            let left = diff / 2;
            let right = diff - left;
            format!("{}{}{}", " ".repeat(left), s, " ".repeat(right))
        }
        Alignment::Left | Alignment::None => format!("{}{}", s, " ".repeat(diff)),
    }
}

#[cfg(test)]
mod tests {
    use crate::markdown::ast::Alignment;
    use crate::theme::Style;

    use super::{RenderedTable, pad_visible, render_table};

    #[test]
    fn pads_right() {
        let s = pad_visible("7", 3, Alignment::Right);
        assert_eq!(s, "  7");
    }

    #[test]
    fn falls_back_when_narrow() {
        let table = RenderedTable {
            headers: vec!["Name".to_string(), "Value".to_string()],
            rows: vec![vec!["Alpha".to_string(), "123".to_string()]],
            aligns: vec![Alignment::Left, Alignment::Right],
        };
        let out = render_table(&table, 10, false, Style::default(), Style::default());
        assert!(out.contains("Name:"));
    }
}
