pub mod detect;
pub mod graphics;
pub mod hyperlink;
pub mod text_size;

// Strips ESC (0x1b) and BEL (0x07) which would prematurely terminate OSC sequences,
// and escapes backslashes. Required before embedding text in any OSC payload.
pub(crate) fn escape_osc(input: &str) -> String {
    input.replace(['\u{1b}', '\u{7}'], "").replace('\\', "\\\\")
}
