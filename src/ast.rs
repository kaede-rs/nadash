#[derive(Debug, Clone, PartialEq)]
pub enum Node {
    Num(f64),
    Str(String),
    Bool(bool),
    Null,
    Var(String),
    Arr(Vec<Node>),
    Obj(Vec<(String, Node)>),
    Bin(String, Box<Node>, Box<Node>),
    Un(String, Box<Node>),
    Index(Box<Node>, Box<Node>),
    Call {
        name: String,
        args: Vec<(String, Node)>,
    },
    Assign {
        target: Box<Node>,
        value: Box<Node>,
    },
    If {
        cond: Box<Node>,
        then_body: Vec<Node>,
        else_body: Vec<Node>,
    },
    RepeatTimes {
        count: Box<Node>,
        body: Vec<Node>,
    },
    RepeatRange {
        from: Box<Node>,
        to: Box<Node>,
        body: Vec<Node>,
    },
    ForEach {
        arr: Box<Node>,
        body: Vec<Node>,
    },
    While {
        cond: Box<Node>,
        body: Vec<Node>,
    },
    FuncDef {
        name: String,
        params: Vec<String>,
        body: Vec<Node>,
    },
    Return(Option<Box<Node>>),
    Break,
    Continue,
    Expr(Box<Node>),
}

impl Node {
    pub fn is_funcdef(&self) -> bool {
        matches!(self, Node::FuncDef { .. })
    }
}

pub type Ast = Vec<Node>;

pub fn dump(nodes: &[Node], indent: usize) -> String {
    let mut out = String::new();
    for n in nodes {
        dump_node(n, indent, &mut out);
    }
    out
}

fn pad(indent: usize) -> String {
    "  ".repeat(indent)
}

fn dump_node(n: &Node, ind: usize, out: &mut String) {
    let p = pad(ind);
    match n {
        Node::Num(v) => out.push_str(&format!("{}数値 {}\n", p, v)),
        Node::Str(s) => out.push_str(&format!("{}文字列 「{}」\n", p, s)),
        Node::Bool(b) => out.push_str(&format!("{}真偽 {}\n", p, b)),
        Node::Null => out.push_str(&format!("{}無効\n", p)),
        Node::Var(s) => out.push_str(&format!("{}変数 {}\n", p, s)),
        Node::Arr(items) => {
            out.push_str(&format!("{}配列\n", p));
            for it in items {
                dump_node(it, ind + 1, out);
            }
        }
        Node::Obj(items) => {
            out.push_str(&format!("{}オブジェクト\n", p));
            for (k, v) in items {
                out.push_str(&format!("{}キー「{}」:\n", p, k));
                dump_node(v, ind + 1, out);
            }
        }
        Node::Bin(op, a, b) => {
            out.push_str(&format!("{}二項 「{}」\n", p, op));
            dump_node(a, ind + 1, out);
            dump_node(b, ind + 1, out);
        }
        Node::Un(op, a) => {
            out.push_str(&format!("{}単項 「{}」\n", p, op));
            dump_node(a, ind + 1, out);
        }
        Node::Index(base, idx) => {
            out.push_str(&format!("{}添字\n", p));
            dump_node(base, ind + 1, out);
            dump_node(idx, ind + 1, out);
        }
        Node::Call { name, args } => {
            out.push_str(&format!("{}命令 「{}」\n", p, name));
            for (josi, v) in args {
                out.push_str(&format!("{}引数[{}]\n", p, josi));
                dump_node(v, ind + 1, out);
            }
        }
        Node::Assign { target, value } => {
            out.push_str(&format!("{}代入\n", p));
            dump_node(target, ind + 1, out);
            dump_node(value, ind + 1, out);
        }
        Node::If { cond, then_body, else_body } => {
            out.push_str(&format!("{}もし\n", p));
            dump_node(cond, ind + 1, out);
            out.push_str(&format!("{}ならば\n", p));
            for s in then_body {
                dump_node(s, ind + 1, out);
            }
            if !else_body.is_empty() {
                out.push_str(&format!("{}違えば\n", p));
                for s in else_body {
                    dump_node(s, ind + 1, out);
                }
            }
        }
        Node::RepeatTimes { count, body } => {
            out.push_str(&format!("{}回繰り返し\n", p));
            dump_node(count, ind + 1, out);
            for s in body {
                dump_node(s, ind + 1, out);
            }
        }
        Node::RepeatRange { from, to, body } => {
            out.push_str(&format!("{}範囲繰り返し\n", p));
            dump_node(from, ind + 1, out);
            dump_node(to, ind + 1, out);
            for s in body {
                dump_node(s, ind + 1, out);
            }
        }
        Node::ForEach { arr, body } => {
            out.push_str(&format!("{}反復\n", p));
            dump_node(arr, ind + 1, out);
            for s in body {
                dump_node(s, ind + 1, out);
            }
        }
        Node::While { cond, body } => {
            out.push_str(&format!("{}間繰り返し\n", p));
            dump_node(cond, ind + 1, out);
            for s in body {
                dump_node(s, ind + 1, out);
            }
        }
        Node::FuncDef { name, params, body } => {
            out.push_str(&format!("{}関数定義 「{}」({})\n", p, name, params.join(", ")));
            for s in body {
                dump_node(s, ind + 1, out);
            }
        }
        Node::Return(e) => {
            out.push_str(&format!("{}戻る\n", p));
            if let Some(e) = e {
                dump_node(e, ind + 1, out);
            }
        }
        Node::Break => out.push_str(&format!("{}抜ける\n", p)),
        Node::Continue => out.push_str(&format!("{}続ける\n", p)),
        Node::Expr(e) => {
            out.push_str(&format!("{}式\n", p));
            dump_node(e, ind + 1, out);
        }
    }
}
