pub mod genrust;
pub mod runtime;

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::ast::Node;
use crate::error::{NadeError, Result};

pub fn compile(
    ast: &[Node],
    funcs: &HashMap<String, Vec<String>>,
    out: Option<&Path>,
    emit_rust: Option<&Path>,
) -> Result<PathBuf> {
    let src = genrust::generate(ast, funcs)?;
    let keep_rust = emit_rust.map(|p| p.to_path_buf());
    let rs_path = match &keep_rust {
        Some(p) => p.clone(),
        None => std::env::temp_dir().join(format!("nadash_gen_{}.rs", std::process::id())),
    };
    std::fs::write(&rs_path, &src)
        .map_err(|e| NadeError::new(format!("Rustソースの書き込みに失敗しました: {}", e), 0))?;

    let exe: PathBuf = match out {
        Some(p) => p.to_path_buf(),
        None => PathBuf::from("nadash_out"),
    };

    if let Some(parent) = exe.parent() {
        if !parent.as_os_str().is_empty() && !parent.exists() {
            std::fs::create_dir_all(parent)
                .map_err(|e| NadeError::new(format!("出力先ディレクトリ作成失敗: {}", e), 0))?;
        }
    }

    let rustc = std::env::var("RUSTC").unwrap_or_else(|_| "rustc".to_string());
    let st = Command::new(&rustc)
        .arg("-O")
        .arg("--edition")
        .arg("2021")
        .arg("-o")
        .arg(&exe)
        .arg(&rs_path)
        .status()
        .map_err(|_| {
            NadeError::new(
                format!("コンパイラ『{}』が起動できません。PATHを確認してください", rustc),
                0,
            )
        })?;

    if !st.success() {
        return Err(NadeError::new("生成コードのコンパイルに失敗しました", 0));
    }
    Ok(exe)
}
