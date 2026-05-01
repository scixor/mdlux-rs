#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Alignment {
    Left,
    Center,
    Right,
    None,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Block {
    Heading {
        level: u8,
        content: Vec<Inline>,
    },
    Paragraph(Vec<Inline>),
    CodeBlock {
        lang: Option<String>,
        code: String,
    },
    BlockQuote(Vec<Block>),
    List {
        ordered: bool,
        start: Option<u64>,
        items: Vec<Vec<Block>>,
    },
    Table {
        headers: Vec<Vec<Inline>>,
        rows: Vec<Vec<Vec<Inline>>>,
        aligns: Vec<Alignment>,
    },
    FootnoteDefinition {
        name: String,
        content: Vec<Block>,
    },
    Rule,
    Image {
        alt: String,
        path: String,
        title: Option<String>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Inline {
    Text(String),
    Emph(Vec<Inline>),
    Strong(Vec<Inline>),
    Strike(Vec<Inline>),
    Code(String),
    Link {
        text: Vec<Inline>,
        dest: String,
        title: Option<String>,
    },
    Image {
        alt: String,
        path: String,
        title: Option<String>,
    },
    FootnoteRef(String),
    SoftBreak,
    HardBreak,
}

impl Inline {
    pub fn plain_text(inlines: &[Self]) -> String {
        let mut out = String::new();
        for inline in inlines {
            match inline {
                Inline::Text(text) | Inline::Code(text) => out.push_str(text),
                Inline::Emph(inner) | Inline::Strong(inner) | Inline::Strike(inner) => {
                    out.push_str(&Self::plain_text(inner))
                }
                Inline::Link { text, .. } => out.push_str(&Self::plain_text(text)),
                Inline::Image { alt, .. } => out.push_str(alt),
                Inline::FootnoteRef(name) => {
                    out.push_str("[^");
                    out.push_str(name);
                    out.push(']');
                }
                Inline::SoftBreak | Inline::HardBreak => out.push(' '),
            }
        }
        out
    }
}
