pub mod detect;
pub mod graphics;
pub mod hyperlink;
pub mod text_size;

pub(crate) fn escape_osc(input: &str) -> String {
    input.replace(['\u{1b}', '\u{7}'], "").replace('\\', "\\\\")
}
