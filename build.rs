use std::path::Path;
use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo::rustc-check-cfg=cfg(has_luajit)");
    if probe_luajit() {
        println!("cargo:rustc-cfg=has_luajit");
    }
}

fn probe_luajit() -> bool {
    let mut dirs: Vec<String> = Vec::new();
    if let Ok(out) = Command::new("pkg-config").args(["--variable=libdir", "luajit"]).output() {
        if out.status.success() {
            let d = String::from_utf8_lossy(&out.stdout).trim().to_string();
            if !d.is_empty() {
                dirs.push(d);
            }
        }
    }
    dirs.push("/usr/lib/x86_64-linux-gnu".to_string());
    dirs.push("/usr/lib".to_string());
    dirs.push("/usr/local/lib".to_string());
    for dir in &dirs {
        let so = Path::new(dir).join("libluajit-5.1.so");
        let a = Path::new(dir).join("libluajit-5.1.a");
        if so.exists() || a.exists() {
            println!("cargo:rustc-link-search=native={}", dir);
            println!("cargo:rustc-link-lib=dylib=luajit-5.1");
            return true;
        }
    }
    false
}
