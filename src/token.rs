#[derive(Debug, Clone, PartialEq)]
pub enum TokKind {
    Num(f64),
    Str(String),
    Word(String),
    Josi(String),
    Op(String),
    Eol,
    Eof,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub kind: TokKind,
    pub line: usize,
}

impl Token {
    pub fn new(kind: TokKind, line: usize) -> Self {
        Token { kind, line }
    }

    pub fn word(&self) -> Option<&str> {
        match &self.kind {
            TokKind::Word(w) => Some(w),
            _ => None,
        }
    }

    pub fn josi(&self) -> Option<&str> {
        match &self.kind {
            TokKind::Josi(j) => Some(j),
            _ => None,
        }
    }

    pub fn is_op(&self, s: &str) -> bool {
        matches!(&self.kind, TokKind::Op(o) if o == s)
    }

    pub fn is_word(&self, s: &str) -> bool {
        self.word() == Some(s)
    }

    pub fn desc(&self) -> String {
        match &self.kind {
            TokKind::Num(n) => format!("数値({})", n),
            TokKind::Str(s) => format!("文字列「{}」", s),
            TokKind::Word(w) => format!("語「{}」", w),
            TokKind::Josi(j) => format!("助詞「{}」", j),
            TokKind::Op(o) => format!("記号「{}」", o),
            TokKind::Eol => "改行".into(),
            TokKind::Eof => "終端".into(),
        }
    }
}
