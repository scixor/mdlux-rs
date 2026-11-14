use crate::markdown::ast::Alignment;
use crate::render::inline::Span;
use crate::render::media::{MediaKind, MediaSlot};
use crate::theme::Style;
use crate::util::width::visible_width;

use super::ansi::apply_style;
use super::wrap::wrap_spans;

#[derive(Debug, Clone)]
pub struct RenderedCellMedia {
    pub source: String,
    pub alt: String,
    pub width_cells: u32,
    pub height_cells: u32,
    pub sequence: Option<String>,
}

#[derive(Debug, Clone)]
pub struct RenderedTable {
    pub headers: Vec<String>,
    pub rows: Vec<Vec<String>>,
    pub aligns: Vec<Alignment>,
    pub media: Vec<Vec<Vec<RenderedCellMedia>>>,
}

pub fn render_table(
    table: &RenderedTable,
    width: usize,
    ansi: bool,
    header_style: Style,
    border_style: Style,
) -> String {
    render_table_internal(
        table,
        width,
        ansi,
        header_style,
        border_style,
        0,
        &mut Vec::new(),
    )
}

pub fn render_table_media(
    table: &RenderedTable,
    width: usize,
    ansi: bool,
    header_style: Style,
    border_style: Style,
    line_offset: usize,
    slots: &mut Vec<MediaSlot>,
) -> String {
    render_table_internal(
        table,
        width,
        ansi,
        header_style,
        border_style,
        line_offset,
        slots,
    )
}

fn render_table_internal(
    table: &RenderedTable,
    width: usize,
    ansi: bool,
    header_style: Style,
    border_style: Style,
    line_offset: usize,
    slots: &mut Vec<MediaSlot>,
) -> String {
    if table.headers.is_empty() {
        return render_table_fallback(table);
    }
    let cols = table.headers.len();
    let has_media = table
        .media
        .iter()
        .any(|row| row.iter().any(|cell| !cell.is_empty()));

    if !has_media {
        let mut col_widths = vec![1usize; cols];
        for (i, header) in table.headers.iter().enumerate() {
            col_widths[i] = col_widths[i].max(visible_width(header));
        }
        for row in &table.rows {
            for (i, cell) in row.iter().enumerate().take(cols) {
                col_widths[i] = col_widths[i].max(visible_width(cell));
            }
        }
        if table_total_width(&col_widths) <= width {
            return render_simple_table(table, &col_widths, ansi, header_style, border_style);
        }
    }

    if available_width(width, cols) < cols {
        return render_table_fallback(table);
    }
    render_table_wrapped(
        table,
        width,
        ansi,
        header_style,
        border_style,
        line_offset,
        slots,
    )
}

fn render_simple_table(
    table: &RenderedTable,
    widths: &[usize],
    ansi: bool,
    header_style: Style,
    border_style: Style,
) -> String {
    let border = apply_style(&render_border(widths), border_style, ansi);
    let mut out = String::new();
    out.push_str(&border);
    out.push('\n');
    out.push_str(&render_row(
        &table.headers,
        widths,
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
            widths,
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

#[allow(clippy::too_many_arguments)]
fn render_table_wrapped(
    table: &RenderedTable,
    width: usize,
    ansi: bool,
    header_style: Style,
    border_style: Style,
    line_offset: usize,
    slots: &mut Vec<MediaSlot>,
) -> String {
    let cols = table.headers.len();
    let available = available_width(width, cols);
    let mut desired = vec![1usize; cols];
    for (i, header) in table.headers.iter().enumerate() {
        desired[i] = desired[i].max(visible_width(header));
    }
    for (row_idx, row) in table.rows.iter().enumerate() {
        for (i, cell) in row.iter().enumerate().take(cols) {
            let mut wanted = visible_width(cell);
            if let Some(cell_media) = table.media.get(row_idx).and_then(|row| row.get(i)) {
                wanted = wanted.max(
                    cell_media
                        .iter()
                        .map(|media| media.width_cells)
                        .max()
                        .unwrap_or(0) as usize,
                );
            }
            desired[i] = desired[i].max(wanted);
        }
    }

    let mut widths = vec![1usize; cols];
    if desired.iter().sum::<usize>() <= available {
        widths.clone_from(&desired);
    } else {
        distribute_width(&desired, &mut widths, available.saturating_sub(cols));
    }

    let border = apply_style(&render_border(&widths), border_style, ansi);
    let mut result = String::new();
    result.push_str(&border);
    result.push('\n');
    result.push_str(&render_wrapped_row(
        &table.headers,
        &widths,
        &table.aligns,
        ansi,
        header_style,
        border_style,
        None,
        line_offset,
        slots,
    ));
    result.push('\n');
    result.push_str(&border);
    result.push('\n');
    for (row_idx, row) in table.rows.iter().enumerate() {
        let line_before = result.lines().count();
        result.push_str(&render_wrapped_row(
            row,
            &widths,
            &table.aligns,
            ansi,
            Style::default(),
            border_style,
            table.media.get(row_idx).map(Vec::as_slice),
            line_offset + line_before,
            slots,
        ));
        result.push('\n');
    }
    result.push_str(&border);
    result
}

fn available_width(width: usize, cols: usize) -> usize {
    width.saturating_sub(cols * 3 + 1)
}

fn distribute_width(desired: &[usize], widths: &mut [usize], extra: usize) {
    let mut remaining = extra;
    while remaining > 0 {
        let mut chosen: Option<usize> = None;
        let mut best = 0usize;
        for (i, &wanted) in desired.iter().enumerate() {
            if widths[i] < wanted {
                let deficit = wanted - widths[i];
                if deficit > best {
                    best = deficit;
                    chosen = Some(i);
                }
            }
        }
        let Some(index) = chosen else { break };
        widths[index] += 1;
        remaining -= 1;
    }
}

fn wrap_cell_row(cells: &[String], widths: &[usize]) -> Vec<Vec<String>> {
    widths
        .iter()
        .enumerate()
        .map(|(idx, width)| {
            let wrapped = wrap_cell(cells.get(idx).map_or("", String::as_str), *width);
            if wrapped.is_empty() {
                vec![String::new()]
            } else {
                wrapped
            }
        })
        .collect()
}

#[allow(clippy::too_many_arguments)]
fn render_wrapped_row(
    cells: &[String],
    widths: &[usize],
    aligns: &[Alignment],
    ansi: bool,
    row_style: Style,
    border_style: Style,
    media: Option<&[Vec<RenderedCellMedia>]>,
    line_offset: usize,
    slots: &mut Vec<MediaSlot>,
) -> String {
    let cols = widths.len();
    let wrapped = wrap_cell_row(cells, widths);
    let media_heights: Vec<usize> = (0..cols)
        .map(|idx| {
            media
                .and_then(|rows| rows.get(idx))
                .map(|entries| {
                    entries
                        .iter()
                        .map(|entry| entry.height_cells as usize)
                        .sum()
                })
                .unwrap_or(0)
        })
        .collect();
    let max_lines = (0..cols)
        .map(|idx| wrapped[idx].len() + media_heights[idx])
        .max()
        .unwrap_or(1)
        .max(1);

    if let Some(media) = media {
        for (cell_idx, cell_media) in media.iter().enumerate().take(cols) {
            let mut media_top = 0usize;
            for entry in cell_media {
                let column = 2 + widths
                    .iter()
                    .take(cell_idx)
                    .map(|width| width + 3)
                    .sum::<usize>();
                slots.push(MediaSlot {
                    source: entry.source.clone(),
                    alt: entry.alt.clone(),
                    kind: MediaKind::Image,
                    line: (line_offset + media_top) as u32,
                    column: column as u32,
                    width_cells: entry.width_cells,
                    height_cells: entry.height_cells,
                });
                media_top += entry.height_cells as usize;
            }
        }
    }

    let mut out = String::new();
    for line_idx in 0..max_lines {
        out.push_str(&apply_style("|", border_style, ansi));
        for idx in 0..cols {
            let line = line_idx
                .checked_sub(media_heights[idx])
                .and_then(|text_idx| wrapped[idx].get(text_idx))
                .map_or("", String::as_str);
            let align = aligns.get(idx).copied().unwrap_or(Alignment::Left);
            let mut cell = String::new();
            if let Some(entries) = media.and_then(|rows| rows.get(idx)) {
                let mut media_top = 0usize;
                for entry in entries {
                    if media_top == line_idx
                        && let Some(sequence) = &entry.sequence
                    {
                        cell.push_str(sequence);
                    }
                    media_top += entry.height_cells as usize;
                }
            }
            cell.push_str(&pad_visible(line, widths[idx], align));
            out.push_str(&apply_style(&format!(" {cell} "), row_style, ansi));
            out.push_str(&apply_style("|", border_style, ansi));
        }
        if line_idx + 1 < max_lines {
            out.push('\n');
        }
    }
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

fn wrap_cell(text: &str, width: usize) -> Vec<String> {
    if width == 0 {
        return vec![text.to_string()];
    }
    let span = Span {
        text: text.to_string(),
        style: Style::default(),
    };
    wrap_spans(&[span], width)
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
            media: Vec::new(),
        };
        let out = render_table(&table, 5, false, Style::default(), Style::default());
        assert!(out.contains("Name:"));
    }
}
