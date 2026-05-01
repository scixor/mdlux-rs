// Kitty text-sizing protocol: https://sw.kovidgoyal.net/kitty/text-sizing-protocol/
// Escape format: ESC ] 66 ; <metadata> ; <text> BEL
// Metadata: colon-separated key=value pairs.
//   s=1..7   integer scale; each char occupies s*s cells
//   n/d      fractional scale where d > n; shrinks font to n/d of a cell height
//   v=0..2   vertical alignment: 0=top (superscript), 1=bottom (subscript), 2=center
//   w=0..7   explicit cell width; 0 = auto-calculate from Unicode properties

pub enum TextSize {
    /// Integer scale s=1..7. Each character renders in an s*s cell block.
    Scale(u8),
    /// Sub-cell fractional size n/d (d must be > n). Shrinks font without changing cell count.
    /// Used for superscripts (valign=0), subscripts (valign=1), or centered (valign=2).
    SubScale {
        numerator: u8,
        denominator: u8,
        valign: u8,
    },
}

pub fn sized_text(text: &str, size: TextSize) -> String {
    let escaped = super::escape_osc(text);
    match size {
        TextSize::Scale(s) => format!("\u{1b}]66;s={s};{escaped}\u{7}"),
        TextSize::SubScale {
            numerator: n,
            denominator: d,
            valign: v,
        } => {
            format!("\u{1b}]66;n={n}:d={d}:v={v};{escaped}\u{7}")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{TextSize, sized_text};

    #[test]
    fn uses_osc_66_with_bel_terminator() {
        let out = sized_text("Hello", TextSize::Scale(2));
        assert!(out.starts_with("\u{1b}]66;s=2;"));
        assert!(out.ends_with('\u{7}'));
    }

    #[test]
    fn subscript_includes_valign() {
        let out = sized_text(
            "x",
            TextSize::SubScale {
                numerator: 1,
                denominator: 2,
                valign: 1,
            },
        );
        assert!(out.contains("n=1:d=2:v=1"));
    }
}
