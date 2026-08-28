use std::collections::HashMap;

use crate::ast::{dump, Node};
use crate::builtins;
use crate::error::{NadeError, Result};
use crate::lexer;
use crate::token::{TokKind, Token};

const TARAREBA: &[&str] = &["ならば", "なら", "たら", "れば"];
const NO_PAREN_CALL: &[&str] = &[
    "そして", "または", "違えば", "ここまで", "ここから", "もし", "抜ける",
    "続ける", "戻る", "代入", "反復",
];
const TARAREBA_NEG: &[&str] = &["なければ", "でなければ"];

pub struct Parser {
    toks: Vec<Token>,
    pos: usize,
    pub funcs: HashMap<String, Vec<String>>,
}

pub struct Parsed {
    ast: Vec<Node>,
    funcs: HashMap<String, Vec<String>>,
    dump: String,
}

impl Parsed {
    pub fn ast(&self) -> &[Node] {
        &self.ast
    }
    pub fn funcs(&self) -> &HashMap<String, Vec<String>> {
        &self.funcs
    }
    pub fn dump_text(&self) -> String {
        self.dump.clone()
    }
}

pub fn parse(src: &str) -> Result<Parsed> {
    let toks = lexer::tokenize(src)?;
    let mut p = Parser { toks, pos: 0, funcs: HashMap::new() };
    let ast = p.parse_program()?;
    let d = dump(&ast, 0);
    Ok(Parsed { ast, funcs: p.funcs, dump: d })
}

pub fn parse_expr_snippet(src: &str) -> Option<Node> {
    let toks = lexer::tokenize(src).ok()?;
    let mut p = Parser { toks, pos: 0, funcs: HashMap::new() };
    p.skip_seps();
    if p.eof() {
        return None;
    }
    let e = p.parse_sentence().ok()?;
    p.skip_seps();
    if !p.eof() {
        return None;
    }
    Some(e)
}

fn expand_string(s: &str, line: usize) -> Result<Node> {
    if !s.contains('{') {
        return Ok(Node::Str(s.to_string()));
    }
    let chars: Vec<char> = s.chars().collect();
    let mut parts: Vec<Node> = Vec::new();
    let mut lit = String::new();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c == '{' {
            let mut depth = 1;
            let mut j = i + 1;
            while j < chars.len() && depth > 0 {
                if chars[j] == '{' {
                    depth += 1;
                } else if chars[j] == '}' {
                    depth -= 1;
                    if depth == 0 {
                        break;
                    }
                }
                j += 1;
            }
            if depth != 0 {
                lit.push(c);
                i += 1;
                continue;
            }
            let inner: String = chars[i + 1..j].iter().collect();
            let parsed = parse_expr_snippet(&inner);
            match parsed {
                Some(node) => {
                    if !lit.is_empty() {
                        parts.push(Node::Str(std::mem::take(&mut lit)));
                    }
                    parts.push(node);
                    i = j + 1;
                }
                None => {
                    lit.push(c);
                    i += 1;
                }
            }
        } else {
            lit.push(c);
            i += 1;
        }
    }
    if !lit.is_empty() {
        parts.push(Node::Str(lit));
    }
    if parts.is_empty() {
        return Ok(Node::Str(String::new()));
    }
    let mut it = parts.into_iter();
    let first = it.next().unwrap();
    let mut acc = first;
    for p in it {
        acc = Node::Bin("..".into(), Box::new(acc), Box::new(p));
    }
    let _ = line;
    Ok(acc)
}

impl Parser {
    fn peek(&self, k: usize) -> &Token {
        let i = (self.pos + k).min(self.toks.len() - 1);
        &self.toks[i]
    }

    fn cur_line(&self) -> usize {
        self.peek(0).line
    }

    fn next(&mut self) -> Token {
        let t = self.peek(0).clone();
        if self.pos < self.toks.len() - 1 {
            self.pos += 1;
        }
        t
    }

    fn eof(&self) -> bool {
        matches!(self.peek(0).kind, TokKind::Eof)
    }

    fn skip_seps(&mut self) {
        while matches!(self.peek(0).kind, TokKind::Eol) || self.peek(0).is_op(",") {
            self.next();
        }
    }

    fn skip_newlines(&mut self) {
        while matches!(self.peek(0).kind, TokKind::Eol) {
            self.next();
        }
    }

    fn starts_expr(&self) -> bool {
        match &self.peek(0).kind {
            TokKind::Num(_) | TokKind::Str(_) => true,
            TokKind::Word(w) => {
                !matches!(w.as_str(), "違えば" | "ここまで" | "ここから")
                    && TARAREBA.contains(&w.as_str()) == false
                    && TARAREBA_NEG.contains(&w.as_str()) == false
            }
            TokKind::Op(o) => matches!(o.as_str(), "(" | "[" | "{" | "-" | "!"),
            _ => false,
        }
    }

    pub fn parse_program(&mut self) -> Result<Vec<Node>> {
        self.prepass_funcs();
        let mut out = Vec::new();
        loop {
            self.skip_seps();
            if self.eof() {
                break;
            }
            out.push(self.parse_statement()?);
        }
        Ok(out)
    }

    fn prepass_funcs(&mut self) {
        let toks = self.toks.clone();
        let n = toks.len();
        for i in 0..n {
            if let TokKind::Word(w) = &toks[i].kind {
                if w == "●" {
                    let mut j = i + 1;
                    if j < n && toks[j].is_op("(") {
                        let mut depth = 0;
                        while j < n {
                            if toks[j].is_op("(") {
                                depth += 1;
                            } else if toks[j].is_op(")") {
                                depth -= 1;
                                if depth == 0 {
                                    j += 1;
                                    break;
                                }
                            }
                            j += 1;
                        }
                    }
                    if j < n {
                        if let TokKind::Word(nm) = &toks[j].kind {
                            if !nm.starts_with('●') && nm != "●" {
                                if toks.get(j + 1).and_then(|t| t.josi()) == Some("とは") {
                                    self.funcs.entry(nm.clone()).or_default();
                                }
                            }
                        }
                    }
                } else if w.starts_with('●') && w.len() > 3 {
                    if toks.get(i + 1).and_then(|t| t.josi()) == Some("とは") {
                        let name = w.trim_start_matches('●').to_string();
                        self.funcs.entry(name).or_default();
                    }
                }
            }
        }
    }

    fn parse_body(&mut self, what: &str, line: usize) -> Result<(Vec<Node>, bool)> {
        while self.peek(0).is_op(",") {
            self.next();
        }
        let mut multiline = false;
        if self.peek(0).is_word("ここから") {
            self.next();
            multiline = true;
        }
        if matches!(self.peek(0).kind, TokKind::Eol) {
            multiline = true;
        }
        if !multiline {
            if self.eof() {
                return Err(NadeError::new(
                    format!("『{}』構文の後に文がありません", what),
                    line,
                ));
            }
            return Ok((vec![self.parse_statement()?], false));
        }
        self.skip_seps();
        let mut body = Vec::new();
        loop {
            self.skip_seps();
            if self.eof() {
                return Err(NadeError::new(
                    format!("『{}』構文には『ここまで』が必要です", what),
                    line,
                ));
            }
            if self.peek(0).is_word("ここまで") {
                self.next();
                break;
            }
            if self.peek(0).is_word("違えば") {
                break;
            }
            body.push(self.parse_statement()?);
        }
        Ok((body, true))
    }

    pub fn parse_statement(&mut self) -> Result<Node> {
        let start = self.pos;
        let line = self.cur_line();
        let n = self.parse_statement_inner()?;
        if self.pos == start {
            return Err(NadeError::new(
                format!("解析できない文です: {}", self.peek(0).desc()),
                line,
            ));
        }
        Ok(n)
    }

    fn parse_statement_inner(&mut self) -> Result<Node> {
        let line = self.cur_line();

        if let TokKind::Word(w) = &self.peek(0).kind {
            match w.as_str() {
                "もし" => return self.parse_if(),
                "抜ける" => {
                    self.next();
                    return Ok(Node::Break);
                }
                "続ける" => {
                    self.next();
                    return Ok(Node::Continue);
                }
                "戻る" => {
                    self.next();
                    self.skip_seps();
                    if self.starts_expr() {
                        let e = self.parse_expr()?;
                        return Ok(Node::Return(Some(Box::new(e))));
                    }
                    return Ok(Node::Return(None));
                }
                _ => {}
            }
            if w.starts_with('●') {
                return self.parse_def_func();
            }
            if w == "変数" || w == "定数" {
                let is_const = w == "定数";
                self.next();
                if self.peek(0).josi() == Some("の") {
                    self.next();
                }
                if let TokKind::Word(name) = &self.peek(0).kind {
                    let name = name.clone();
                    self.next();
                    self.skip_seps();
                    if self.peek(0).is_op("=") {
                        self.next();
                        let v = self.parse_assign_rhs(line)?;
                        return Ok(assign_node(var_target(&name)?, v, is_const));
                    }
                    return Ok(assign_node(var_target(&name)?, Node::Null, is_const));
                }
                return Err(NadeError::new("『変数』の後には変数名が必要です", line));
            }
        }

        if self.starts_assign_target() {
            if let Some(n) = self.try_parse_assign()? {
                return Ok(n);
            }
        }

        let n = self.parse_sentence()?;
        Ok(wrap_stmt(n))
    }

    fn starts_assign_target(&self) -> bool {
        matches!(self.peek(0).kind, TokKind::Word(_))
    }

    fn try_parse_assign(&mut self) -> Result<Option<Node>> {
        let line = self.cur_line();
        let save = self.pos;
        let lhs = match self.parse_postfix() {
            Ok(n) => n,
            Err(_) => {
                self.pos = save;
                return Ok(None);
            }
        };
        if !matches!(lhs, Node::Var(_) | Node::Index(..)) {
            self.pos = save;
            return Ok(None);
        }
        if self.peek(0).is_op("=") {
            self.next();
            let rhs = self.parse_assign_rhs(line)?;
            return Ok(Some(Node::Assign { target: Box::new(lhs), value: Box::new(rhs) }));
        }
        self.pos = save;
        Ok(None)
    }

    fn parse_assign_rhs(&mut self, line: usize) -> Result<Node> {
        let e = self.parse_expr()?;
        if self.peek(0).josi().is_some() {
            return self.parse_chain(e, line);
        }
        Ok(e)
    }

    fn parse_if(&mut self) -> Result<Node> {
        let line = self.cur_line();
        self.next();
        self.skip_seps();
        let mut cond = self.parse_expr()?;
        let jt = self.peek(0).clone();
        let neg;
        match jt.josi() {
            Some(j) if TARAREBA.contains(&j) => {
                neg = false;
                self.next();
            }
            Some(j) if TARAREBA_NEG.contains(&j) => {
                neg = true;
                self.next();
            }
            _ => {
                return Err(NadeError::new(
                    "『もし』文の条件の後には『ならば』『たら』『れば』などが必要です",
                    line,
                ))
            }
        }
        if neg {
            cond = Node::Un("!".into(), Box::new(cond));
        }
        let (then_body, then_multi) = self.parse_body("もし", line)?;
        self.skip_seps();
        let mut else_body = Vec::new();
        let mut else_multi = false;
        if self.peek(0).is_word("違えば") {
            self.next();
            let r = self.parse_body("違えば", line)?;
            else_body = r.0;
            else_multi = r.1;
        }
        let _ = (then_multi, else_multi);
        Ok(Node::If {
            cond: Box::new(cond),
            then_body,
            else_body,
        })
    }

    fn parse_def_func(&mut self) -> Result<Node> {
        let line = self.cur_line();
        let mut params: Vec<String> = Vec::new();
        let mut name: Option<String> = None;
        if let TokKind::Word(w) = &self.peek(0).kind {
            if w.starts_with('●') {
                let n = w.trim_start_matches('●').to_string();
                self.next();
                if !n.is_empty() {
                    name = Some(n);
                }
            } else {
                return Err(NadeError::new("関数定義は『●』で始めます", line));
            }
        } else {
            return Err(NadeError::new("関数定義は『●』で始めます", line));
        }
        if self.peek(0).is_op("(") {
            params = self.parse_paren_params()?;
        }
        if name.is_none() {
            if let TokKind::Word(w) = &self.peek(0).kind {
                let w = w.clone();
                self.next();
                name = Some(w);
            } else {
                return Err(NadeError::new("関数名が必要です", line));
            }
        }
        if self.peek(0).is_op("(") && params.is_empty() {
            params = self.parse_paren_params()?;
        }
        match self.peek(0).josi() {
            Some("とは") => {
                self.next();
            }
            _ => {
                return Err(NadeError::new(
                    format!("関数『{}』の定義には『とは』が必要です", name.clone().unwrap_or_default()),
                    line,
                ))
            }
        }
        let name = name.unwrap();
        let (body, _) = self.parse_body(&format!("関数{}", name), line)?;
        self.funcs.insert(name.clone(), params.clone());
        Ok(Node::FuncDef { name, params, body })
    }

    fn try_parse_paren_call(&mut self) -> Result<Option<Node>> {
        let save = self.pos;
        let line = self.cur_line();
        self.next();
        if self.peek(0).is_op(")") {
            self.pos = save;
            return Ok(None);
        }
        let mut args: Vec<(String, Node)> = Vec::new();
        let mut curv: Option<Node> = None;
        let mut command: Option<String> = None;
        let ok = (|| -> Result<bool> {
            let first = self.parse_expr()?;
            curv = Some(first);
            loop {
                if self.peek(0).is_op(")") || self.eof() {
                    break;
                }
                if let TokKind::Word(w) = &self.peek(0).kind {
                    if builtins::lookup(w).is_some() || self.funcs.contains_key(w) {
                        let w = w.clone();
                        self.next();
                        command = Some(w);
                        break;
                    }
                    return Ok(false);
                }
                let Some(j) = self.peek(0).josi().map(|s| s.to_string()) else {
                    return Ok(false);
                };
                self.next();
                let v = curv.take().ok_or_else(|| NadeError::new("括弧内の解析に失敗しました", line))?;
                args.push((j, v));
                if self.peek(0).is_op(")") {
                    break;
                }
                let known_word = match &self.peek(0).kind {
                    TokKind::Word(w) => {
                        builtins::lookup(w).is_some() || self.funcs.contains_key(w)
                    }
                    _ => false,
                };
                if known_word {
                    continue;
                }
                if !self.starts_expr() {
                    return Ok(false);
                }
                let nv = self.parse_expr()?;
                curv = Some(nv);
            }
            if !self.peek(0).is_op(")") {
                return Ok(false);
            }
            Ok(true)
        })()
        .unwrap_or(false);
        if !ok {
            self.pos = save;
            return Ok(None);
        }
        self.next();
        if let Some(w) = command {
            if let Some(v) = curv.take() {
                args.push((String::new(), v));
            }
            return Ok(Some(Node::Call { name: w, args }));
        }
        if let TokKind::Word(w) = &self.peek(0).kind {
            if !NO_PAREN_CALL.contains(&w.as_str()) {
                let w = w.clone();
                self.next();
                if let Some(v) = curv.take() {
                    args.push((String::new(), v));
                }
                return Ok(Some(Node::Call { name: w, args }));
            }
        }
        self.pos = save;
        let _ = line;
        Ok(None)
    }

    fn parse_paren_params(&mut self) -> Result<Vec<String>> {
        let line = self.cur_line();
        self.next();
        let mut ps = Vec::new();
        loop {
            self.skip_seps();
            if self.peek(0).is_op(")") {
                self.next();
                break;
            }
            if self.eof() {
                return Err(NadeError::new("関数の引数定義で『)』がありません", line));
            }
            if let TokKind::Word(w) = &self.peek(0).kind {
                let w = w.clone();
                self.next();
                ps.push(w);
                if matches!(self.peek(0).josi(), Some("と") | Some("の")) {
                    self.next();
                } else if self.peek(0).is_op(",") {
                    self.next();
                }
            } else {
                return Err(NadeError::new("関数の引数名として解釈できません", line));
            }
        }
        Ok(ps)
    }

    fn parse_sentence(&mut self) -> Result<Node> {
        let line = self.cur_line();
        let e1 = self.parse_expr()?;

        if self.peek(0).josi() == Some("は") {
            self.next();
            let v = self.parse_assign_rhs(line)?;
            if !matches!(e1, Node::Var(_) | Node::Index(..)) {
                return Err(NadeError::new("『は』の左側には変数を指定してください", line));
            }
            return Ok(Node::Assign { target: Box::new(e1), value: Box::new(v) });
        }

        if self.peek(0).is_word("回") {
            self.next();
            if self.peek(0).is_word("繰り返す") || self.peek(0).is_word("繰返") {
                self.next();
            }
            let (body, _multi) = self.parse_body("回", line)?;
            return Ok(Node::RepeatTimes { count: Box::new(e1), body });
        }

        if self.peek(0).josi() == Some("の") && self.peek(1).is_word("間") {
            self.next();
            self.next();
            if self.peek(0).is_word("繰り返す") || self.peek(0).is_word("繰返") {
                self.next();
            }
            let (body, _multi) = self.parse_body("間", line)?;
            return Ok(Node::While { cond: Box::new(e1), body });
        }

        if self.peek(0).josi() == Some("から") {
            let from = e1;
            self.next();
            let to = self.parse_expr()?;
            if self.peek(0).josi() != Some("まで") {
                return Err(NadeError::new(
                    "『AからBまで繰り返す』の形で指定してください",
                    line,
                ));
            }
            self.next();
            if self.peek(0).is_word("繰り返す") || self.peek(0).is_word("繰返") {
                self.next();
                let (body, _multi) = self.parse_body("繰り返す", line)?;
                return Ok(Node::RepeatRange {
                    from: Box::new(from),
                    to: Box::new(to),
                    body,
                });
            }
            return Err(NadeError::new("『まで』の後は『繰り返す』が必要です", line));
        }

        if self.peek(0).josi() == Some("を") && self.peek(1).is_word("反復") {
            self.next();
            self.next();
            let (body, _multi) = self.parse_body("反復", line)?;
            return Ok(Node::ForEach { arr: Box::new(e1), body });
        }

        let mut args: Vec<(String, Node)> = Vec::new();
        let mut curv: Option<Node> = Some(e1);
        loop {
            let Some(j) = self.peek(0).josi().map(|s| s.to_string()) else {
                break;
            };
            let Some(v) = curv.take() else {
                break;
            };
            self.next();
            args.push((j, v));
            let known_word = match &self.peek(0).kind {
                TokKind::Word(w) => {
                    builtins::lookup(w).is_some()
                        || self.funcs.contains_key(w)
                        || w == "代入"
                }
                _ => false,
            };
            if !known_word {
                if !self.starts_expr() {
                    if self.peek(0).josi().is_some() {
                        return Err(NadeError::new(
                            format!("助詞『{}』の前に値がありません", self.peek(0).josi().unwrap()),
                            line,
                        ));
                    }
                    break;
                }
                let nv = self.parse_expr()?;
                curv = Some(nv);
            } else {
                break;
            }
        }

        if let TokKind::Word(w) = &self.peek(0).kind {
            let w = w.clone();
            self.next();
            if w == "代入" {
                let target = find_arg(&args, &["に", "へ"])
                    .ok_or_else(|| NadeError::new("『代入』には『～に～を代入』の形が必要です", line))?;
                let value = find_arg(&args, &["を"])
                    .ok_or_else(|| NadeError::new("『代入』には『～に～を代入』の形が必要です", line))?;
                if !matches!(target, Node::Var(_) | Node::Index(..)) {
                    return Err(NadeError::new("『代入』の代入先が不正です", line));
                }
                return Ok(Node::Assign {
                    target: Box::new(target.clone()),
                    value: Box::new(value.clone()),
                });
            }
            let node = Node::Call { name: w, args };
            if self.peek(0).josi().is_some() {
                return self.parse_chain(node, line);
            }
            return Ok(node);
        }

        if let Some(f) = curv {
            if args.is_empty() {
                return Ok(f);
            }
        }
        Err(NadeError::new(
            format!("命令名が見つかりません: {}", self.peek(0).desc()),
            line,
        ))
    }

    fn parse_chain(&mut self, first: Node, line: usize) -> Result<Node> {
        let mut args: Vec<(String, Node)> = Vec::new();
        let mut curv: Option<Node> = Some(first);
        loop {
            let Some(j) = self.peek(0).josi().map(|s| s.to_string()) else {
                break;
            };
            let Some(v) = curv.take() else {
                break;
            };
            self.next();
            args.push((j, v));
            let known_word = match &self.peek(0).kind {
                TokKind::Word(w) => {
                    builtins::lookup(w).is_some()
                        || self.funcs.contains_key(w)
                        || w == "代入"
                }
                _ => false,
            };
            if !known_word {
                if !self.starts_expr() {
                    if self.peek(0).josi().is_some() {
                        return Err(NadeError::new(
                            format!("助詞『{}』の前に値がありません", self.peek(0).josi().unwrap()),
                            line,
                        ));
                    }
                    break;
                }
                let nv = self.parse_expr()?;
                curv = Some(nv);
            } else {
                break;
            }
        }
        if let TokKind::Word(w) = &self.peek(0).kind {
            let w = w.clone();
            self.next();
            let node = Node::Call { name: w, args };
            if self.peek(0).josi().is_some() {
                return self.parse_chain(node, line);
            }
            return Ok(node);
        }
        Err(NadeError::new(
            format!("命令名が見つかりません: {}", self.peek(0).desc()),
            line,
        ))
    }

    pub fn parse_expr(&mut self) -> Result<Node> {
        self.parse_or()
    }

    fn parse_or(&mut self) -> Result<Node> {
        let mut l = self.parse_and()?;
        while self.peek(0).is_word("または") || self.peek(0).is_op("||") {
            self.next();
            let r = self.parse_and()?;
            l = Node::Bin("||".into(), Box::new(l), Box::new(r));
        }
        Ok(l)
    }

    fn parse_and(&mut self) -> Result<Node> {
        let mut l = self.parse_concat()?;
        while self.peek(0).is_word("そして") || self.peek(0).is_op("&&") {
            self.next();
            let r = self.parse_concat()?;
            l = Node::Bin("&&".into(), Box::new(l), Box::new(r));
        }
        Ok(l)
    }

    fn parse_concat(&mut self) -> Result<Node> {
        let mut l = self.parse_cmp()?;
        while self.peek(0).is_op("&") {
            self.next();
            let r = self.parse_cmp()?;
            l = Node::Bin("..".into(), Box::new(l), Box::new(r));
        }
        Ok(l)
    }

    fn parse_cmp(&mut self) -> Result<Node> {
        let mut l = self.parse_add()?;
        loop {
            let op = match &self.peek(0).kind {
                TokKind::Op(o) if matches!(o.as_str(), "==" | "=" | "!=" | "<" | ">" | "<=" | ">=") => o.clone(),
                _ => break,
            };
            let op = if op == "=" { "==".to_string() } else { op };
            self.next();
            let r = self.parse_add()?;
            l = Node::Bin(op, Box::new(l), Box::new(r));
        }
        Ok(l)
    }

    fn parse_add(&mut self) -> Result<Node> {
        let mut l = self.parse_mul()?;
        loop {
            let op = match &self.peek(0).kind {
                TokKind::Op(o) if o == "+" || o == "-" => o.clone(),
                _ => break,
            };
            self.next();
            let r = self.parse_mul()?;
            l = Node::Bin(op, Box::new(l), Box::new(r));
        }
        Ok(l)
    }

    fn parse_mul(&mut self) -> Result<Node> {
        let mut l = self.parse_unary()?;
        loop {
            let op = match &self.peek(0).kind {
                TokKind::Op(o) if matches!(o.as_str(), "*" | "/" | "%" | "^") => o.clone(),
                _ => break,
            };
            self.next();
            let r = self.parse_unary()?;
            l = Node::Bin(op, Box::new(l), Box::new(r));
        }
        Ok(l)
    }

    fn parse_unary(&mut self) -> Result<Node> {
        if self.peek(0).is_op("-") {
            self.next();
            let e = self.parse_unary()?;
            return Ok(Node::Un("-".into(), Box::new(e)));
        }
        if self.peek(0).is_op("!") {
            self.next();
            let e = self.parse_unary()?;
            return Ok(Node::Un("!".into(), Box::new(e)));
        }
        self.parse_pow()
    }

    fn parse_pow(&mut self) -> Result<Node> {
        let base = self.parse_postfix()?;
        if self.peek(0).is_op("^") {
            self.next();
            let exp = self.parse_unary()?;
            return Ok(Node::Bin("^".into(), Box::new(base), Box::new(exp)));
        }
        Ok(base)
    }

    fn parse_postfix(&mut self) -> Result<Node> {
        let mut n = self.parse_primary()?;
        while self.peek(0).is_op("[") {
            self.next();
            let idx = self.parse_expr()?;
            if !self.peek(0).is_op("]") {
                return Err(NadeError::new("『]』が必要です", self.cur_line()));
            }
            self.next();
            n = Node::Index(Box::new(n), Box::new(idx));
        }
        Ok(n)
    }

    fn parse_primary(&mut self) -> Result<Node> {
        let line = self.cur_line();
        let t = self.peek(0).clone();
        match t.kind {
            TokKind::Num(v) => {
                self.next();
                Ok(Node::Num(v))
            }
            TokKind::Str(s) => {
                self.next();
                expand_string(&s, line)
            }
            TokKind::Op(ref o) if o == "(" => {
                if let Some(n) = self.try_parse_paren_call()? {
                    return Ok(n);
                }
                self.next();
                let e = self.parse_expr()?;
                if !self.peek(0).is_op(")") {
                    return Err(NadeError::new("『)』が必要です", line));
                }
                self.next();
                Ok(e)
            }
            TokKind::Op(ref o) if o == "[" => {
                self.next();
                let mut items = Vec::new();
                self.skip_newlines();
                if !self.peek(0).is_op("]") {
                    loop {
                        items.push(self.parse_expr()?);
                        self.skip_newlines();
                        if self.peek(0).is_op(",") {
                            self.next();
                            self.skip_seps();
                            continue;
                        }
                        break;
                    }
                }
                if !self.peek(0).is_op("]") {
                    return Err(NadeError::new("『]』が必要です", line));
                }
                self.next();
                Ok(Node::Arr(items))
            }
            TokKind::Op(ref o) if o == "{" => {
                self.next();
                let mut items: Vec<(String, Node)> = Vec::new();
                self.skip_newlines();
                if !self.peek(0).is_op("}") {
                    loop {
                        let key = match &self.peek(0).kind {
                            TokKind::Str(s) => {
                                let s = s.clone();
                                self.next();
                                s
                            }
                            TokKind::Word(w) => {
                                let w = w.clone();
                                self.next();
                                w
                            }
                            TokKind::Num(v) => {
                                let s = fmt_num(*v);
                                self.next();
                                s
                            }
                            _ => {
                                return Err(NadeError::new(
                                    "オブジェクトのキーには文字列か語を指定してください",
                                    line,
                                ))
                            }
                        };
                        if !self.peek(0).is_op(":") {
                            return Err(NadeError::new("オブジェクトは『キー:値』の形で書きます", line));
                        }
                        self.next();
                        let v = self.parse_expr()?;
                        items.push((key, v));
                        self.skip_newlines();
                        if self.peek(0).is_op(",") {
                            self.next();
                            self.skip_seps();
                            continue;
                        }
                        break;
                    }
                }
                if !self.peek(0).is_op("}") {
                    return Err(NadeError::new("『}』が必要です", line));
                }
                self.next();
                Ok(Node::Obj(items))
            }
            TokKind::Word(ref w) => {
                let w = w.clone();
                match w.as_str() {
                    "はい" | "真" => {
                        self.next();
                        return Ok(Node::Bool(true));
                    }
                    "いいえ" | "偽" => {
                        self.next();
                        return Ok(Node::Bool(false));
                    }
                    "無" | "ヌル" => {
                        self.next();
                        return Ok(Node::Null);
                    }
                    _ => {}
                }
                if self.peek(1).is_op("(") {
                    self.next();
                    self.next();
                    let mut args = Vec::new();
                    self.skip_seps();
                    if !self.peek(0).is_op(")") {
                        loop {
                            let e = self.parse_expr()?;
                            args.push((String::new(), e));
                            self.skip_seps();
                            if self.peek(0).is_op(",") {
                                self.next();
                                self.skip_seps();
                                continue;
                            }
                            break;
                        }
                    }
                    if !self.peek(0).is_op(")") {
                        return Err(NadeError::new("『)』が必要です", line));
                    }
                    self.next();
                    return Ok(Node::Call { name: w, args });
                }
                self.next();
                Ok(Node::Var(w))
            }
            _ => Err(NadeError::new(format!("式として解釈できません: {}", t.desc()), line)),
        }
    }
}

fn wrap_stmt(n: Node) -> Node {
    match n {
        Node::Call { .. }
        | Node::Assign { .. }
        | Node::If { .. }
        | Node::RepeatTimes { .. }
        | Node::RepeatRange { .. }
        | Node::ForEach { .. }
        | Node::While { .. }
        | Node::FuncDef { .. }
        | Node::Return(_)
        | Node::Break
        | Node::Continue => n,
        other => Node::Expr(Box::new(other)),
    }
}

fn var_target(name: &str) -> Result<Node> {
    Ok(Node::Var(name.to_string()))
}

fn assign_node(target: Node, value: Node, _is_const: bool) -> Node {
    Node::Assign { target: Box::new(target), value: Box::new(value) }
}

fn find_arg<'a>(args: &'a [(String, Node)], variants: &[&str]) -> Option<&'a Node> {
    for (j, v) in args {
        if variants.contains(&j.as_str()) {
            return Some(v);
        }
    }
    None
}

pub fn fmt_num(v: f64) -> String {
    if v.fract() == 0.0 && v.abs() < 1e15 {
        format!("{}", v as i64)
    } else {
        format!("{}", v)
    }
}
