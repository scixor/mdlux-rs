// OSC 8 hyperlink protocol: ESC ] 8 ; params ; uri ST  label  ESC ] 8 ;; ST
// params field is typically empty; uri is the link target.
// https://gist.github.com/egmontkob/eb114294efbcd5adb1944c9f3cb5feda
pub fn osc8_link(label: &str, url: &str) -> String {
    let safe_label = super::escape_osc(label);
    let safe_url = super::escape_osc(url);
    format!(
        "\u{1b}]8;;{}\u{1b}\\{}\u{1b}]8;;\u{1b}\\",
        safe_url, safe_label
    )
}

#[cfg(test)]
mod tests {
    use super::osc8_link;

    #[test]
    fn wraps_link_with_osc8() {
        let out = osc8_link("docs", "https://example.com");
        assert!(out.starts_with("\u{1b}]8;;https://example.com"));
        assert!(out.contains("docs"));
        assert!(out.ends_with("\u{1b}]8;;\u{1b}\\"));
    }
}
