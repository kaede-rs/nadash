pub mod ast;
pub mod builtins;
pub mod callargs;
pub mod compile;
pub mod error;
pub mod ident;
pub mod interp;
pub mod lexer;
pub mod parser;
pub mod token;

use std::collections::HashMap;

use crate::ast::Node;
use crate::error::Result;

pub struct Program {
    pub ast: Vec<Node>,
    pub funcs: HashMap<String, Vec<String>>,
}

pub fn compile_source(src: &str) -> Result<Program> {
    let p = parser::parse(src)?;
    Ok(Program { ast: p.ast().to_vec(), funcs: p.funcs().clone() })
}

pub fn run_interpreter(src: &str) -> Result<()> {
    let prog = compile_source(src)?;
    let lua = interp::genlua::generate(&prog.ast, &prog.funcs)?;
    interp::run_lua(&lua)
}

pub fn generate_lua(src: &str) -> Result<String> {
    let prog = compile_source(src)?;
    interp::genlua::generate(&prog.ast, &prog.funcs)
}

pub fn run_compiler(
    src: &str,
    out: Option<&std::path::Path>,
    emit_rust: Option<&std::path::Path>,
) -> Result<std::path::PathBuf> {
    let prog = compile_source(src)?;
    compile::compile(&prog.ast, &prog.funcs, out, emit_rust)
}
