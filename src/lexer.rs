use crate::error::{NadeError, Result};
use crate::token::{TokKind, Token};

pub const JOSI_LIST: &[&str] = &[
    "について", "くらい", "なのか", "までを", "までの", "による", "として",
    "とは", "から", "まで", "だけ", "より", "ほど", "など",
    "いて", "えて", "きて", "けて", "して", "って", "にて", "みて",
    "めて", "ねて", "では", "には", "んで", "ずつ",
    "ならば", "なら", "たら", "れば", "なければ", "でなければ",
    "は", "を", "に", "へ", "で", "と", "が", "の",
];

const RESERVED: &[&str] = &[
    "ここまで", "ここから", "違えば", "もし", "繰り返す", "繰返", "反復",
    "抜ける", "続ける", "戻る", "代入", "変数", "定数", "それ", "そう",
    "回数", "対象", "対象キー", "関数", "実行", "そして", "または",
    "はい", "いいえ", "真", "偽", "無", "ヌル",
];

fn completes_reserved(buf: &[char], next: &[char]) -> bool {
    RESERVED.iter().any(|r| {
        let rc: Vec<char> = r.chars().collect();
        rc.len() > buf.len() && rc.starts_with(buf) && next.starts_with(&rc[buf.len()..])
    })
}

const MULTI_OPS: &[&str] = &["<=", ">=", "==", "!=", "<>", "=<", "=>", "&&", "||"];
const SINGLE_OPS: &str = "+-*/%^&|!<>=()[]{}.:;?~@";

fn is_reserved(s: &str) -> bool {
    RESERVED.contains(&s)
}

fn match_tail_josi(chars: &[char]) -> Option<&'static str> {
    let mut best: Option<&'static str> = None;
    for &j in JOSI_LIST {
        let jl = j.chars().count();
        if chars.len() >= jl {
            let tail: String = chars[chars.len() - jl..].iter().collect();
            if tail == j && best.map_or(true, |b| b.chars().count() < jl) {
                best = Some(j);
            }
        }
    }
    best
}

fn is_josi_ext(chars: &[char]) -> bool {
    let s: String = chars.iter().collect();
    JOSI_LIST.contains(&s.as_str())
}

fn is_space(c: char) -> bool {
    c == ' ' || c == '\t' || c == '\u{3000}' || c == '\r'
}

fn is_delim(c: char) -> bool {
    is_space(c) || c == '\n' || c == '。' || c == '、' || c == ',' || c == '#'
        || "「『」』\"'".contains(c)
}

fn is_op_char(c: char) -> bool {
    SINGLE_OPS.contains(c)
}

fn normalize(src: &str) -> String {
    let mut out = String::with_capacity(src.len());
    for c in src.chars() {
        match c {
            '０'..='９' => out.push(((c as u32) - '０' as u32 + '0' as u32) as u8 as char),
            '＋' => out.push('+'),
            '－' => out.push('-'),
            '＊' | '×' => out.push('*'),
            '／' | '÷' => out.push('/'),
            '＝' => out.push('='),
            '＜' => out.push('<'),
            '＞' => out.push('>'),
            '≦' => out.push_str("<="),
            '≧' => out.push_str(">="),
            '≠' => out.push_str("!="),
            '％' => out.push('%'),
            '＆' => out.push('&'),
            '｜' => out.push('|'),
            '，' => out.push(','),
            '（' => out.push('('),
            '）' => out.push(')'),
            '［' => out.push('['),
            '］' => out.push(']'),
            '｛' => out.push('{'),
            '｝' => out.push('}'),
            _ => out.push(c),
        }
    }
    out
}

pub struct Lexer {
    pub tokens: Vec<Token>,
    line: usize,
}

impl Lexer {
    pub fn tokenize(src: &str) -> Result<Vec<Token>> {
        let norm = normalize(src);
        let chars: Vec<char> = norm.chars().collect();
        let mut lx = Lexer { tokens: Vec::new(), line: 1 };
        let mut buf: Vec<char> = Vec::new();
        let mut i = 0usize;
        let n = chars.len();

        macro_rules! flush_boundary {
            () => { lx.flush_word(&mut buf)?; };
        }

        while i < n {
            let c = chars[i];
            if c == '\n' {
                flush_boundary!();
                lx.tokens.push(Token::new(TokKind::Eol, lx.line));
                lx.line += 1;
                i += 1;
                continue;
            }
            if is_space(c) {
                flush_boundary!();
                i += 1;
                continue;
            }
            if c == '。' {
                flush_boundary!();
                lx.tokens.push(Token::new(TokKind::Eol, lx.line));
                i += 1;
                continue;
            }
            if c == '、' || c == ',' {
                flush_boundary!();
                lx.tokens.push(Token::new(TokKind::Op(",".into()), lx.line));
                i += 1;
                continue;
            }
            if c == '#' || (c == '/' && chars.get(i + 1) == Some(&'/')) {
                flush_boundary!();
                while i < n && chars[i] != '\n' {
                    i += 1;
                }
                continue;
            }
            if "「『\"'".contains(c) {
                flush_boundary!();
                i = lx.read_string(&chars, i)?;
                continue;
            }
            if c.is_ascii_digit() || (c == '.' && chars.get(i + 1).map_or(false, |d| d.is_ascii_digit())) {
                flush_boundary!();
                i = lx.read_number(&chars, i)?;
                continue;
            }
            if is_op_char(c) {
                flush_boundary!();
                i = lx.read_op(&chars, i)?;
                continue;
            }
            buf.push(c);
            i += 1;
            let cur: String = buf.iter().collect();
            if is_reserved(&cur) {
                let nc = chars.get(i).copied();
                if nc == Some('は') && chars.get(i + 1) == Some(&'い') {
                    // 『それはいいえ』のような曖昧さを避ける
                    continue;
                }
                let extended = nc.map_or(false, |x| {
                    let mut s = cur.clone();
                    s.push(x);
                    is_reserved(&s)
                });
                let ordinary =
                    nc.map_or(false, |x| !is_delim(x) && !is_op_char(x) && !x.is_ascii_digit());
                if !extended && ordinary {
                    lx.tokens.push(Token::new(TokKind::Word(cur), lx.line));
                    buf.clear();
                    continue;
                }
            }
            let delayed = completes_reserved(&buf, &chars[i..]);
            if !delayed {
            if let Some(j) = match_tail_josi(&buf) {
                let nc = chars.get(i).copied();
                let ordinary = nc.map_or(false, |x| !is_delim(x) && !is_op_char(x) && !x.is_ascii_digit());
                let extended: Option<String> = nc.map(|x| {
                    let mut s: String = j.to_string();
                    s.push(x);
                    s
                });
                let extendable = extended.map_or(false, |s| is_josi_ext(&s.chars().collect::<Vec<char>>()));
                if ordinary && !extendable {
                    let jl = j.chars().count();
                    let plen = buf.len() - jl;
                    if plen > 0 {
                        let w: String = buf[..plen].iter().collect();
                        lx.tokens.push(Token::new(TokKind::Word(w), lx.line));
                    }
                    lx.tokens.push(Token::new(TokKind::Josi(j.to_string()), lx.line));
                    buf.clear();
                }
            }
            }
        }
        flush_boundary!();
        lx.tokens.push(Token::new(TokKind::Eof, lx.line));
        Ok(lx.tokens)
    }

    fn flush_word(&mut self, buf: &mut Vec<char>) -> Result<()> {
        if buf.is_empty() {
            return Ok(());
        }
        let mut josies: Vec<&'static str> = Vec::new();
        while !is_reserved(&buf.iter().collect::<String>()) {
            match match_tail_josi(buf) {
                Some(j) => {
                    let jl = j.chars().count();
                    josies.push(j);
                    buf.truncate(buf.len() - jl);
                }
                None => break,
            }
        }
        if !buf.is_empty() {
            let w: String = buf.iter().collect();
            self.tokens.push(Token::new(TokKind::Word(w), self.line));
        }
        for j in josies.into_iter().rev() {
            self.tokens.push(Token::new(TokKind::Josi(j.to_string()), self.line));
        }
        buf.clear();
        Ok(())
    }

    fn read_string(&mut self, chars: &[char], start: usize) -> Result<usize> {
        let open = chars[start];
        let close = match open {
            '「' => '」',
            '『' => '』',
            '"' => '"',
            '\'' => '\'',
            _ => unreachable!(),
        };
        let mut i = start + 1;
        let mut s = String::new();
        while i < chars.len() {
            let c = chars[i];
            if c == '\\' && (open == '"' || open == '\'') && i + 1 < chars.len() {
                let e = chars[i + 1];
                s.push(match e {
                    'n' => '\n',
                    't' => '\t',
                    other => other,
                });
                i += 2;
                continue;
            }
            if c == close {
                self.tokens.push(Token::new(TokKind::Str(s), self.line));
                return Ok(i + 1);
            }
            if c == '\n' {
                self.line += 1;
            }
            s.push(c);
            i += 1;
        }
        Err(NadeError::new("文字列の終わりが見つかりません", self.line))
    }

    fn read_number(&mut self, chars: &[char], start: usize) -> Result<usize> {
        let mut i = start;
        let mut s = String::new();
        while i < chars.len() && chars[i].is_ascii_digit() {
            s.push(chars[i]);
            i += 1;
        }
        if i + 1 < chars.len() && chars[i] == '.' && chars[i + 1].is_ascii_digit() {
            s.push('.');
            i += 1;
            while i < chars.len() && chars[i].is_ascii_digit() {
                s.push(chars[i]);
                i += 1;
            }
        }
        let v: f64 = s.parse().map_err(|_| NadeError::new(format!("数値を解釈できません: {}", s), self.line))?;
        self.tokens.push(Token::new(TokKind::Num(v), self.line));
        Ok(i)
    }

    fn read_op(&mut self, chars: &[char], start: usize) -> Result<usize> {
        let rest: String = chars[start..(start + 3).min(chars.len())].iter().collect();
        for m in MULTI_OPS {
            if rest.starts_with(m) {
                let o = match *m {
                    "<>" => "!=",
                    "=<" => "<=",
                    "=>" => ">=",
                    other => other,
                };
                self.tokens.push(Token::new(TokKind::Op(o.to_string()), self.line));
                return Ok(start + m.chars().count());
            }
        }
        let c = chars[start];
        self.tokens.push(Token::new(TokKind::Op(c.to_string()), self.line));
        Ok(start + 1)
    }
}

pub fn tokenize(src: &str) -> Result<Vec<Token>> {
    Lexer::tokenize(src)
}
