use std::process::ExitCode;

use nadash::error::NadeError;

const VERSION: &str = env!("CARGO_PKG_VERSION");

fn usage() {
    println!(
        "nadash v{VERSION} - なでしこ3互換 処理系 (Rust + LuaJIT)

使い方:
  nadash run <file.n3> | -e <code>      LuaJITで実行
  nadash compile <file.n3> [-o 出力] [--rust file.rs]
                                        Rustにコンパイル
  nadash lex <file.n3> | -e <code>      字句解析結果を表示
  nadash ast <file.n3> | -e <code>      構文木(AST)を表示
  nadash lua <file.n3> | -e <code>      生成コード(Lua)を表示

環境変数:
  NADASH_LUA    フォールバック用Luaコマンド(既定: luajit)
  RUSTC         コンパイルに使うrustc(既定: rustc)"
    );
}

fn read_input(args: &[String]) -> Result<String, String> {
    if args.is_empty() {
        return Err("入力がありません。ファイル名か -e <コード> を指定してください".into());
    }
    if args[0] == "-e" {
        if args.len() < 2 {
            return Err("-e の後ろにコードを指定してください".into());
        }
        return Ok(args[1].clone());
    }
    std::fs::read_to_string(&args[0])
        .map_err(|e| format!("ファイル『{}』を読めません: {}", args[0], e))
}

fn main() -> ExitCode {
    let argv: Vec<String> = std::env::args().skip(1).collect();
    if argv.is_empty() {
        usage();
        return ExitCode::SUCCESS;
    }
    let cmd = argv[0].as_str();
    let rest = &argv[1..];

    match cmd {
        "--help" | "-h" | "help" => {
            usage();
            ExitCode::SUCCESS
        }
        "--version" | "-V" | "version" => {
            println!("nadash v{VERSION}");
            ExitCode::SUCCESS
        }
        "run" => with_src(rest, |src| nadash::run_interpreter(src)),
        "lua" => with_src(rest, |src| {
            let code = nadash::generate_lua(src)?;
            println!("{code}");
            Ok(())
        }),
        "ast" => with_src(rest, |src| {
            let p = nadash::parser::parse(src)?;
            println!("{}", p.dump_text());
            Ok(())
        }),
        "lex" => with_lex(rest),
        "compile" => cmd_compile(rest),
        other => {
            eprintln!("未知のコマンド『{other}』");
            usage();
            ExitCode::FAILURE
        }
    }
}

fn with_src(args: &[String], f: impl FnOnce(&str) -> nadash::error::Result<()>) -> ExitCode {
    match read_input(args).and_then(|src| f(&src).map_err(|e| e.to_string())) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("{e}");
            ExitCode::FAILURE
        }
    }
}

fn with_lex(args: &[String]) -> ExitCode {
    let src = match read_input(args) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::FAILURE;
        }
    };
    match nadash::lexer::tokenize(&src) {
        Ok(toks) => {
            for t in toks {
                println!("{:4}  {}", t.line, t.desc());
            }
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("字句解析エラー: {e}");
            ExitCode::FAILURE
        }
    }
}

fn cmd_compile(args: &[String]) -> ExitCode {
    let mut src: Option<String> = None;
    let mut out: Option<std::path::PathBuf> = None;
    let mut rust_path: Option<std::path::PathBuf> = None;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "-o" | "--out" => {
                if i + 1 >= args.len() {
                    eprintln!("-o の後ろに出力先を指定してください");
                    return ExitCode::FAILURE;
                }
                out = Some(std::path::PathBuf::from(&args[i + 1]));
                i += 2;
            }
            "--rust" | "--emit-rust" => {
                if i + 1 >= args.len() {
                    eprintln!("--rust の後ろに出力ファイルを指定してください");
                    return ExitCode::FAILURE;
                }
                rust_path = Some(std::path::PathBuf::from(&args[i + 1]));
                i += 2;
            }
            "-e" => {
                if i + 1 >= args.len() {
                    eprintln!("-e の後ろにコードを指定してください");
                    return ExitCode::FAILURE;
                }
                src = Some(args[i + 1].clone());
                i += 2;
            }
            p => {
                match std::fs::read_to_string(p) {
                    Ok(s) => src = Some(s),
                    Err(e) => {
                        eprintln!("ファイル『{p}』を読めません: {e}");
                        return ExitCode::FAILURE;
                    }
                }
                i += 1;
            }
        }
    }
    let Some(src) = src else {
        eprintln!("入力がありません");
        return ExitCode::FAILURE;
    };
    match nadash::run_compiler(&src, out.as_deref(), rust_path.as_deref()) {
        Ok(exe) => {
            println!("コンパイル完了: {}", exe.display());
            ExitCode::SUCCESS
        }
        Err(NadeError { msg, .. }) => {
            eprintln!("{msg}");
            ExitCode::FAILURE
        }
    }
}
