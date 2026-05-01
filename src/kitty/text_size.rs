pub enum TextSize {
    Normal,
    Scale(u8),
    Fraction { numerator: u8, denominator: u8 },
}

pub fn sized_text(text: &str, size: TextSize) -> String {
    match size {
        TextSize::Normal => text.to_string(),
        TextSize::Scale(scale) => {
            format!("\u{1b}]66;s={};{}\u{7}", scale, super::escape_osc(text))
        }
        TextSize::Fraction {
            numerator,
            denominator,
        } => format!(
            "\u{1b}]66;n={}:d={};{}\u{7}",
            numerator,
            denominator,
            super::escape_osc(text)
        ),
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
}
