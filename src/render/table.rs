use crate::markdown::ast::Alignment;
use crate::render::inline::Span;
use crate::theme::Style;
use crate::util::width::visible_width;

use super::wrap::wrap_spans;

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
    let cols = table.headers.len();
    let overhead = cols * 3 + 1;
    let available = width.saturating_sub(overhead);
    if total <= width {
        // existing render path
    } else if available < cols.max(1) {
        return render_table_fallback(table);
    } else {
        return render_table_wrapped(table, width, ansi, header_style, border_style);
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
            let label = if header.trim().is_empty() {
                format!("column {}", i + 1)
            } else {
                header.clone()
            };
            let value = row.get(i).map(|s| s.as_str()).unwrap_or("");
            out.push_str(&format!("{label}: {value}\n"));
        }
        out.push('\n');
    }
    if table.rows.is_empty() {
        for (i, header) in table.headers.iter().enumerate() {
            let label = if header.trim().is_empty() {
                format!("column {}", i + 1)
            } else {
                header.clone()
            };
            out.push_str(&format!("{label}: \n"));
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

fn render_table_wrapped(
    table: &RenderedTable,
    width: usize,
    ansi: bool,
    header_style: Style,
    border_style: Style,
) -> String {
    let cols = table.headers.len();
    if cols == 0 {
        return render_table_fallback(table);
    }
    let overhead = cols * 3 + 1;
    let available = width.saturating_sub(overhead);
    if available < cols {
        return render_table_fallback(table);
    }

    let mut desired = vec![1usize; cols];
    for i in 0..cols {
        let mut w = visible_width(&table.headers[i]);
        for row in &table.rows {
            if let Some(cell) = row.get(i) {
                w = w.max(visible_width(cell));
            }
        }
        desired[i] = w.max(1);
    }

    let mut widths = vec![1usize; cols];
    let total_desired: usize = desired.iter().sum();
    if total_desired <= available {
        widths = desired.clone();
    } else {
        let mut remaining = available.saturating_sub(cols);
        while remaining > 0 {
            let mut chosen: Option<usize> = None;
            let mut best = 0usize;
            for i in 0..cols {
                if widths[i] < desired[i] {
                    let deficit = desired[i] - widths[i];
                    if deficit > best {
                        best = deficit;
                        chosen = Some(i);
                    }
                }
            }
            let Some(i) = chosen else {
                break;
            };
            widths[i] += 1;
            remaining -= 1;
        }
    }

    let border = apply_style(&render_border(&widths), border_style, ansi);
    let mut out = String::new();
    out.push_str(&border);
    out.push('\n');
    out.push_str(&render_wrapped_row(
        &table.headers,
        &widths,
        &table.aligns,
        ansi,
        header_style,
        border_style,
    ));
    out.push('\n');
    out.push_str(&border);
    out.push('\n');
    for row in &table.rows {
        out.push_str(&render_wrapped_row(
            row,
            &widths,
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

fn render_wrapped_row(
    cells: &[String],
    widths: &[usize],
    aligns: &[Alignment],
    ansi: bool,
    row_style: Style,
    border_style: Style,
) -> String {
    let cols = widths.len();
    let mut wrapped: Vec<Vec<String>> = Vec::with_capacity(cols);
    let mut max_lines = 1usize;
    for i in 0..cols {
        let cell = cells.get(i).map(|s| s.as_str()).unwrap_or("");
        let mut lines = wrap_cell(cell, widths[i]);
        if lines.is_empty() {
            lines.push(String::new());
        }
        max_lines = max_lines.max(lines.len());
        wrapped.push(lines);
    }

    let mut out = String::new();
    for line_idx in 0..max_lines {
        out.push_str(&apply_style("|", border_style, ansi));
        for i in 0..cols {
            let line = wrapped[i].get(line_idx).map(|s| s.as_str()).unwrap_or("");
            let align = aligns.get(i).copied().unwrap_or(Alignment::Left);
            let padded = pad_visible(line, widths[i], align);
            let styled = apply_style(&format!(" {padded} "), row_style, ansi);
            out.push_str(&styled);
            out.push_str(&apply_style("|", border_style, ansi));
        }
        if line_idx + 1 < max_lines {
            out.push('\n');
        }
    }
    out
}

fn wrap_cell(text: &str, width: usize) -> Vec<String> {
    if width == 0 {
        return vec![text.to_string()];
    }
    let span = Span {
        text: text.to_string(),
        style: Style::default(),
    };
    let lines = wrap_spans(&[span], width);
    lines
        .into_iter()
        .map(|spans| spans.into_iter().map(|s| s.text).collect())
        .collect()
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
        let out = render_table(&table, 5, false, Style::default(), Style::default());
        assert!(out.contains("Name:"));
    }
}
