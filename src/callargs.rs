use crate::ast::Node;
use crate::error::{NadeError, Result};

pub fn resolve_builtin(
    name: &str,
    params: &[(&'static str, &'static str)],
    args: &[(String, Node)],
) -> Result<Vec<Node>> {
    let mut used = vec![false; args.len()];
    let mut slots: Vec<Option<Node>> = vec![None; params.len()];
    for (pi, (variants, _pname)) in params.iter().enumerate() {
        let vs: Vec<&str> = variants.split('|').collect();
        for (i, (j, _)) in args.iter().enumerate() {
            if !used[i] && vs.contains(&j.as_str()) {
                used[i] = true;
                slots[pi] = Some(args[i].1.clone());
                break;
            }
        }
    }
    let unfilled: Vec<usize> = (0..params.len()).filter(|i| slots[*i].is_none()).collect();
    let unused: Vec<usize> = (0..args.len()).filter(|i| !used[*i]).collect();
    if unfilled.len() == 1 && unused.len() == 1 {
        slots[unfilled[0]] = Some(args[unused[0]].1.clone());
        used[unused[0]] = true;
    } else if !unfilled.is_empty() {
        let (variants, _) = params[unfilled[0]];
        return Err(NadeError::new(
            format!("命令『{}』には『{}』付きの引数が必要です", name, variants),
            0,
        ));
    } else if let Some(i) = unused.first() {
        return Err(NadeError::new(
            format!(
                "命令『{}』に使われない助詞『{}』の引数があります",
                name, args[*i].0
            ),
            0,
        ));
    }
    Ok(slots.into_iter().map(|s| s.unwrap_or(Node::Null)).collect())
}

pub fn resolve_positional(
    _name: &str,
    arity: usize,
    args: &[(String, Node)],
) -> Option<Vec<Node>> {
    if args.len() != arity {
        return None;
    }
    if args.iter().any(|(j, _)| !j.is_empty()) {
        return None;
    }
    Some(args.iter().map(|(_, n)| n.clone()).collect())
}

pub fn resolve_user(name: &str, params: &[String], args: &[(String, Node)]) -> Result<Vec<Node>> {
    if let Some(vals) = resolve_positional(name, params.len(), args) {
        return Ok(vals);
    }
    if args.len() != params.len() {
        return Err(NadeError::new(
            format!(
                "関数『{}』の引数は{}個ですが、{}個指定されています",
                name,
                params.len(),
                args.len()
            ),
            0,
        ));
    }
    Ok(args.iter().map(|(_, n)| n.clone()).collect())
}
