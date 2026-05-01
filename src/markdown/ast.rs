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
