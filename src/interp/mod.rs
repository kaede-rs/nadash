
use crate::error::{NadeError, Result};

#[cfg(has_luajit)]
mod ffi {
    use std::ffi::{c_char, c_int, c_void, CString};
    use std::sync::atomic::{AtomicBool, Ordering};

    use crate::error::{NadeError, Result};

    #[link(name = "luajit-5.1")]
    extern "C" {
        fn luaL_newstate() -> *mut c_void;
        fn luaL_openlibs(L: *mut c_void);
        fn luaL_loadstring(L: *mut c_void, s: *const c_char) -> c_int;
        fn lua_pcall(L: *mut c_void, nargs: c_int, nresults: c_int, errfunc: c_int) -> c_int;
        fn lua_tolstring(L: *mut c_void, index: c_int, len: *mut usize) -> *const c_char;
        fn lua_close(L: *mut c_void);
    }

    static BROKEN: AtomicBool = AtomicBool::new(false);

    #[allow(non_snake_case)]
    unsafe fn pop_err(L: *mut c_void) -> String {
        let mut len: usize = 0;
        let p = lua_tolstring(L, -1, &mut len);
        if p.is_null() {
            return "不明なLuaエラー".to_string();
        }
        let bytes = std::slice::from_raw_parts(p as *const u8, len);
        String::from_utf8_lossy(bytes).into_owned()
    }

    pub fn available() -> bool {
        !BROKEN.load(Ordering::Relaxed)
    }

    pub fn run(code: &str) -> Result<()> {
        if BROKEN.load(Ordering::Relaxed) {
            return Err(NadeError::new("LuaJITが利用できません", 0));
        }
        let cs = match CString::new(code) {
            Ok(c) => c,
            Err(_) => return Err(NadeError::new("内部エラー: LuaコードにNULが含まれています", 0)),
        };
        unsafe {
            #[allow(non_snake_case)]
            let L = luaL_newstate();
            if L.is_null() {
                BROKEN.store(true, Ordering::Relaxed);
                return Err(NadeError::new("Luaステートの作成に失敗しました", 0));
            }
            luaL_openlibs(L);
            let mut status = luaL_loadstring(L, cs.as_ptr());
            if status == 0 {
                status = lua_pcall(L, 0, 0, 0);
            }
            if status != 0 {
                let msg = pop_err(L);
                lua_close(L);
                return Err(NadeError::new(format!("実行時エラー:\n{}", msg), 0));
            }
            lua_close(L);
        }
        Ok(())
    }
}

pub mod genlua;

pub fn run_lua(code: &str) -> Result<()> {
    #[cfg(has_luajit)]
    {
        if ffi::available() {
            return ffi::run(code);
        }
    }
    run_lua_subprocess(code)
}

fn run_lua_subprocess(code: &str) -> Result<()> {
    let exe = std::env::var("NADASH_LUA").unwrap_or_else(|_| "luajit".to_string());
    let tmp = std::env::temp_dir().join(format!("nadash_{}.lua", std::process::id()));
    std::fs::write(&tmp, code)
        .map_err(|e| NadeError::new(format!("一時ファイル書き込み失敗: {}", e), 0))?;
    let st = std::process::Command::new(&exe)
        .arg(&tmp)
        .status()
        .map_err(|_| {
            NadeError::new(
                format!(
                    "Lua実行環境『{}』が見つかりません。NADASH_LUA環境変数で指定してください",
                    exe
                ),
                0,
            )
        })?;
    let _ = std::fs::remove_file(&tmp);
    if st.success() {
        Ok(())
    } else {
        Err(NadeError::new(
            "実行時エラー",
            st.code().unwrap_or(1).max(0) as usize,
        ))
    }
}
