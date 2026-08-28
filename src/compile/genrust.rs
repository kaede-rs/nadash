use std::collections::HashMap;

use crate::ast::Node;
use crate::builtins;
use crate::callargs::{resolve_builtin, resolve_positional, resolve_user};
use crate::compile::runtime::{RUNTIME_RS};
use crate::error::{NadeError, Result};
use crate::ident::{vid, KAISU, TAISHO};

pub fn rust_escape(s: &str) -> String {
    let mut o = String::new();
    for c in s.chars() {
        match c {
            '\\' => o.push_str("\\\\"),
            '"' => o.push_str("\\\""),
            '\n' => o.push_str("\\n"),
            '\t' => o.push_str("\\t"),
            '\r' => o.push_str("\\r"),
            _ => o.push(c),
        }
    }
    o
}

fn var(name: &str) -> String {
    format!("gget(\"{}\")", rust_escape(name))
}

fn num_lit(v: f64) -> String {
    if v.fract() == 0.0 && v.abs() < 9e15 {
        format!("Value::Num({}.0)", v as i64)
    } else if v.is_finite() {
        format!("Value::Num({})", v)
    } else if v > 0.0 {
        "Value::Num(f64::INFINITY)".to_string()
    } else if v < 0.0 {
        "Value::Num(f64::NEG_INFINITY)".to_string()
    } else {
        "Value::Num(f64::NAN)".to_string()
    }
}

pub fn generate(ast: &[Node], funcs: &HashMap<String, Vec<String>>) -> Result<String> {
    let mut out = String::new();
    out.push_str(RUNTIME_RS);
    for n in ast.iter().filter(|n| matches!(n, Node::FuncDef { .. })) {
        gen_funcdef(&mut out, n, funcs)?;
    }
    out.push_str("\nfn run() {\n");
    for n in ast.iter().filter(|n| !matches!(n, Node::FuncDef { .. })) {
        gen_stmt(&mut out, n, funcs)?;
    }
    out.push_str("}\n");
    Ok(out)
}

fn gen_stmt(out: &mut String, node: &Node, funcs: &HashMap<String, Vec<String>>) -> Result<()> {
    match node {
        Node::Expr(e) => {
            out.push_str(&format!("  sore_set({});\n", gen_expr(e, funcs)?));
        }
        Node::Assign { target, value } => match target.as_ref() {
            Node::Var(name) => {
                out.push_str(&format!(
                    "  gset(\"{}\", {});\n",
                    rust_escape(name),
                    gen_expr(value, funcs)?
                ));
            }
            Node::Index(base, idx) => {
                out.push_str(&format!(
                    "  rt_setidx({}, {}, {});\n",
                    gen_expr(base, funcs)?,
                    gen_expr(idx, funcs)?,
                    gen_expr(value, funcs)?
                ));
            }
            other => {
                return Err(NadeError::new(format!("代入先として不正です: {:?}", other), 0))
            }
        },
        Node::If { cond, then_body, else_body } => {
            out.push_str(&format!("  if rt_truthy({}) {{\n", gen_expr(cond, funcs)?));
            for s in then_body {
                gen_stmt(out, s, funcs)?;
            }
            if !else_body.is_empty() {
                out.push_str("  } else {\n");
                for s in else_body {
                    gen_stmt(out, s, funcs)?;
                }
            }
            out.push_str("  }\n");
        }
        Node::RepeatTimes { count, body } => {
            out.push_str(&format!("  {{\n    let __n = rt_int({});\n", gen_expr(count, funcs)?));
            out.push_str("    if __n >= 1 {\n");
            out.push_str("      for __i in 1..=__n {\n");
            out.push_str(&format!(
                "        gset(\"{}\", Value::Num(__i as f64));\n",
                rust_escape(KAISU)
            ));
            for s in body {
                gen_stmt_indent(out, s, funcs, 4)?;
            }
            out.push_str("      }\n    }\n  }\n");
        }
        Node::RepeatRange { from, to, body } => {
            out.push_str(&format!(
                "  {{\n    let __f = rt_int({});\n    let __t = rt_int({});\n",
                gen_expr(from, funcs)?,
                gen_expr(to, funcs)?
            ));
            out.push_str("    if __f <= __t {\n      for __i in __f..=__t {\n");
            out.push_str(&format!(
                "        gset(\"{}\", Value::Num(__i as f64));\n",
                rust_escape(TAISHO)
            ));
            for s in body {
                gen_stmt_indent(out, s, funcs, 4)?;
            }
            out.push_str("      }\n    } else {\n      for __i in (__t..=__f).rev() {\n");
            out.push_str(&format!(
                "        gset(\"{}\", Value::Num(__i as f64));\n",
                rust_escape(TAISHO)
            ));
            for s in body {
                gen_stmt_indent(out, s, funcs, 4)?;
            }
            out.push_str("      }\n    }\n  }\n");
        }
        Node::ForEach { arr: a, body } => {
            out.push_str(&format!(
                "  {{\n    let __items = rt_snapshot({});\n    for __it in __items {{\n",
                gen_expr(a, funcs)?
            ));
            out.push_str(&format!(
                "      gset(\"{}\", __it);\n",
                rust_escape(TAISHO)
            ));
            for s in body {
                gen_stmt_indent(out, s, funcs, 3)?;
            }
            out.push_str("    }\n  }\n");
        }
        Node::While { cond, body } => {
            out.push_str(&format!("  while rt_truthy({}) {{\n", gen_expr(cond, funcs)?));
            for s in body {
                gen_stmt_indent(out, s, funcs, 2)?;
            }
            out.push_str("  }\n");
        }
        Node::Return(e) => match e {
            Some(expr) => {
                out.push_str(&format!(
                    "  {{ let __r = {}; sore_set(__r.clone()); break 'blk __r; }}\n",
                    gen_expr(expr, funcs)?
                ));
            }
            None => out.push_str("  break 'blk sore_get();\n"),
        },
        Node::Break => out.push_str("  break;\n"),
        Node::Continue => out.push_str("  continue;\n"),
        Node::FuncDef { .. } => {
            return Err(NadeError::new("関数定義はプログラム先頭でのみ可能です", 0))
        }
        other_lit @ (Node::Null | Node::Num(_) | Node::Str(_) | Node::Bool(_)) => {
            out.push_str(&format!("  sore_set({});\n", gen_expr(other_lit, funcs)?));
        }
        _ => {
            out.push_str(&format!("  sore_set({});\n", gen_expr(node, funcs)?));
        }
    }
    Ok(())
}

fn gen_stmt_indent(
    out: &mut String,
    node: &Node,
    funcs: &HashMap<String, Vec<String>>,
    depth: usize,
) -> Result<()> {
    let mut tmp = String::new();
    gen_stmt(&mut tmp, node, funcs)?;
    let pad = "    ".repeat(depth);
    for line in tmp.lines() {
        if line.is_empty() {
            continue;
        }
        out.push_str(&pad);
        out.push_str(line);
        out.push('\n');
    }
    Ok(())
}

fn gen_funcdef(
    out: &mut String,
    node: &Node,
    funcs: &HashMap<String, Vec<String>>,
) -> Result<()> {
    let Node::FuncDef { name, params, body } = node else { unreachable!() };
    let fid = vid("uf", name);
    out.push_str(&format!("\nfn {}(args: &[Value]) -> Value {{\n", fid));
    out.push_str("  let __sv: Vec<(&str, Value)> = vec![\n");
    for p in params {
        out.push_str(&format!("    (\"{}\", gget(\"{}\")),\n", rust_escape(p), rust_escape(p)));
    }
    out.push_str("  ];\n");
    for (i, p) in params.iter().enumerate() {
        out.push_str(&format!(
            "  gset(\"{}\", args.get({}).cloned().unwrap_or(Value::Null));\n",
            rust_escape(p),
            i
        ));
    }
    out.push_str("  let __ret: Value = 'blk: {\n");
    for s in body {
        gen_stmt_indent(out, s, funcs, 3)?;
    }
    out.push_str("    sore_get()\n  };\n");
    out.push_str("  for (__k, __v) in __sv {\n    gset(__k, __v);\n  }\n");
    out.push_str("  __ret\n}\n");
    Ok(())
}

fn gen_call(
    name: &str,
    args: &[(String, Node)],
    funcs: &HashMap<String, Vec<String>>,
) -> Result<String> {
    if let Some(params) = funcs.get(name) {
        let vals = resolve_user(name, params, args)?;
        let mut parts = Vec::new();
        for v in &vals {
            parts.push(gen_expr(v, funcs)?);
        }
        return Ok(format!("{}(&[{}])", vid("uf", name), parts.join(", ")));
    }
    if let Some(b) = builtins::lookup(name) {
        let vals = if let Some(p) = resolve_positional(name, b.arity(), args) {
            p
        } else {
            resolve_builtin(b.name, b.params, args)?
        };
        let mut parts = Vec::new();
        for v in &vals {
            parts.push(gen_expr(v, funcs)?);
        }
        return map_builtin(name, &parts);
    }
    Err(NadeError::new(format!("未知の命令または関数『{}』", name), 0))
}

fn map_builtin(name: &str, args: &[String]) -> Result<String> {
    let call = |fname: &str| format!("rt_{}({})", fname, args.join(", "));
    Ok(match name {
        "表示" | "言う" => call("print"),
        "足す" | "足し算" => call("add"),
        "引く" | "引き算" => call("sub"),
        "掛ける" | "掛け算" => call("mul"),
        "割る" | "割り算" => call("div"),
        "余り" => call("mod"),
        "乱数" => call("rand"),
        "四捨五入" => call("round"),
        "切り上げ" => call("ceil"),
        "切り下げ" => call("floor"),
        "絶対値" => call("abs"),
        "符号" => call("sign"),
        "平方根" => call("sqrt"),
        "サイン" => call("sin"),
        "コサイン" => call("cos"),
        "タンジェント" => call("tan"),
        "文字数" | "配列要素数" => call("len"),
        "切り取る" => call("mid"),
        "置換" => call("replace"),
        "区切る" => call("split"),
        "大文字変換" => call("upper"),
        "小文字変換" => call("lower"),
        "トリム" => call("trim"),
        "配列追加" => call("push"),
        "配列ソート" => call("sort"),
        "配列逆順" => call("reverse"),
        "配列接続" => call("join"),
        "配列取得" => call("idx"),
        "JSONエンコード" => call("jsonenc"),
        "JSONデコード" => call("jsondec"),
        "型取得" => call("typename"),
        "待つ" => call("sleep"),
        _ => return Err(NadeError::new(format!("命令『{}』は未実装です", name), 0)),
    })
}

fn gen_expr(node: &Node, funcs: &HashMap<String, Vec<String>>) -> Result<String> {
    Ok(match node {
        Node::Num(v) => num_lit(*v),
        Node::Str(s) => format!("Value::Str(\"{}\".to_string())", rust_escape(s)),
        Node::Bool(b) => format!("Value::Bool({})", b),
        Node::Null => "Value::Null".to_string(),
        Node::Var(name) => {
            if funcs.contains_key(name) && builtins::lookup(name).is_none() {
                format!("{}(&[])", vid("uf", name))
            } else {
                var(name)
            }
        }
        Node::Arr(items) => {
            let mut parts = Vec::new();
            for it in items {
                parts.push(gen_expr(it, funcs)?);
            }
            format!("arr(vec![{}])", parts.join(", "))
        }
        Node::Obj(items) => {
            let mut parts = Vec::new();
            for (k, v) in items {
                parts.push(format!(
                    "(\"{}\", {})",
                    rust_escape(k),
                    gen_expr(v, funcs)?
                ));
            }
            format!("obj(vec![{}])", parts.join(", "))
        }
        Node::Bin(op, l, r) => {
            let lv = gen_expr(l, funcs)?;
            let rv = gen_expr(r, funcs)?;
            match op.as_str() {
                "+" => format!("rt_add({}, {})", lv, rv),
                "-" => format!("rt_sub({}, {})", lv, rv),
                "*" => format!("rt_mul({}, {})", lv, rv),
                "/" => format!("rt_div({}, {})", lv, rv),
                "%" => format!("rt_mod({}, {})", lv, rv),
                "^" => format!("rt_pow({}, {})", lv, rv),
                ".." => format!("Value::Str(format!(\"{{}}{{}}\", rt_tostr({}), rt_tostr({})))", lv, rv),
                "==" => format!("Value::Bool(rt_eq({}, {}))", lv, rv),
                "!=" => format!("Value::Bool(!rt_eq({}, {}))", lv, rv),
                "<" => format!("Value::Bool(rt_lt({}, {}))", lv, rv),
                ">" => format!("Value::Bool(rt_lt({}, {}))", rv, lv),
                "<=" => format!("Value::Bool(rt_le({}, {}))", lv, rv),
                ">=" => format!("Value::Bool(rt_le({}, {}))", rv, lv),
                "&&" => format!(
                    "{{ let __l = {}; if __l.truthy() {{ Value::Bool({}.truthy()) }} else {{ Value::Bool(false) }} }}",
                    lv, rv
                ),
                "||" => format!(
                    "{{ let __l = {}; if __l.truthy() {{ Value::Bool(true) }} else {{ Value::Bool({}.truthy()) }} }}",
                    lv, rv
                ),
                other => return Err(NadeError::new(format!("未知の演算子『{}』", other), 0)),
            }
        }
        Node::Un(op, e) => {
            let ev = gen_expr(e, funcs)?;
            match op.as_str() {
                "-" => format!("Value::Num(-{}.num())", ev),
                "!" => format!("Value::Bool(!{}.truthy())", ev),
                other => return Err(NadeError::new(format!("未知の単項演算子『{}』", other), 0)),
            }
        }
        Node::Index(base, idx) => {
            format!("rt_idx({}, {})", gen_expr(base, funcs)?, gen_expr(idx, funcs)?)
        }
        Node::Call { name, args } => gen_call(name, args, funcs)?,
        Node::Expr(inner) => gen_expr(inner, funcs)?,
        other => {
            return Err(NadeError::new(
                format!("式として扱えないノード: {:?}", other),
                0,
            ))
        }
    })
}
