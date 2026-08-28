use std::collections::HashMap;

use crate::ast::Node;
use crate::builtins;
use crate::callargs::{resolve_builtin, resolve_positional, resolve_user};
use crate::error::{NadeError, Result};
use crate::ident::{vid, KAISU, SORE, TAISHO};

pub struct LuaGen {
    out: String,
    ind: usize,
}

pub const PRELUDE_LUA: &str = r#"
local function nd_type(x)
  local t = type(x)
  if t == "nil" then return "無効" end
  if t == "boolean" then return "真偽値" end
  if t == "number" then return "数値" end
  if t == "string" then return "文字列" end
  if t == "table" then
    if rawget(x, 0) == "arr" then return "配列" end
    return "オブジェクト"
  end
  return t
end

local function nd_utf8len(s)
  local n = 0
  for _ in s:gmatch("[%z\1-\127\194-\244][\128-\191]*") do n = n + 1 end
  return n
end

local function nd_utf8sub(s, a, b)
  local chars = {}
  for ch in s:gmatch("[%z\1-\127\194-\244][\128-\191]*") do chars[#chars + 1] = ch end
  local parts = {}
  for i = a, b do parts[#parts + 1] = chars[i] or "" end
  return table.concat(parts)
end

local function nd_tostr(x)
  local t = type(x)
  if t == "nil" then return "無効" end
  if t == "boolean" then return x and "はい" or "いいえ" end
  if t == "number" then
    if x == math.floor(x) and math.abs(x) < 2^53 then return string.format("%d", x) end
    return tostring(x)
  end
  if t == "string" then return x end
  if type(x) == "table" and rawget(x, 0) == "arr" then
    local ps = {}
    for i = 1, #x do ps[i] = nd_tostr(x[i]) end
    return "[" .. table.concat(ps, ",") .. "]"
  end
  if type(x) == "table" then
    local ps = {}
    for k, v in pairs(x) do ps[#ps + 1] = tostring(k) .. ":" .. nd_tostr(v) end
    return "{" .. table.concat(ps, ",") .. "}"
  end
  return tostring(x)
end

local function nd_tonum_soft(x)
  if type(x) == "number" then return x end
  if type(x) == "string" then return tonumber(x) end
  if type(x) == "boolean" then return x and 1 or 0 end
  return nil
end

local function nd_num(x)
  local n = nd_tonum_soft(x)
  if n == nil then error("数値が必要ですが「" .. nd_tostr(x) .. "」が指定されました") end
  return n
end

local function nd_truthy(x) return not (x == nil or x == false) end

local function nd_add(a, b)
  if type(a) == "table" or type(b) == "table" then error("テーブル型は足し算できません") end
  if type(a) == "boolean" or type(b) == "boolean" then error("真偽値は足し算できません") end
  local na, nb = nd_tonum_soft(a), nd_tonum_soft(b)
  if na ~= nil and nb ~= nil then return na + nb end
  return nd_tostr(a) .. nd_tostr(b)
end

local function nd_sub(a, b) return nd_num(a) - nd_num(b) end
local function nd_mul(a, b) return nd_num(a) * nd_num(b) end

local function nd_div(a, b)
  local bb = nd_num(b)
  if bb == 0 then error("ゼロ除算エラー") end
  return nd_num(a) / bb
end

local function nd_mod(a, b)
  local bb = nd_num(b)
  if bb == 0 then error("ゼロ除算エラー") end
  return nd_num(a) % bb
end

local function nd_pow(a, b) return nd_num(a) ^ nd_num(b) end

local function nd_eq(a, b)
  if type(a) == "table" or type(b) == "table" then return a == b end
  if a == nil or b == nil then return (a == nil) and (b == nil) end
  local na, nb = nd_tonum_soft(a), nd_tonum_soft(b)
  if na ~= nil and nb ~= nil then return na == nb end
  return nd_tostr(a) == nd_tostr(b)
end

local function nd_cmp_lt(a, b)
  if type(a) == "string" and type(b) == "string" then return a < b end
  return nd_num(a) < nd_num(b)
end

local function nd_le(a, b)
  if nd_eq(a, b) then return true end
  return nd_cmp_lt(a, b)
end

local function nd_idx(t, i)
  if type(t) == "string" then
    local n = math.floor(nd_num(i))
    return nd_utf8sub(t, n, n)
  end
  if type(t) ~= "table" then error("添字アクセスできるのは配列・オブジェクト・文字列だけです") end
  if type(i) == "number" then return t[math.floor(i) + 1] end
  return t[i]
end

local function nd_setidx(t, i, v)
  if type(t) ~= "table" then error("代入できるのは配列・オブジェクトだけです") end
  if type(i) == "number" then t[math.floor(i) + 1] = v else t[i] = v end
end

local function nd_len(x)
  if type(x) == "string" then return nd_utf8len(x) end
  if type(x) == "table" and rawget(x, 0) == "arr" then return #x end
  if type(x) == "table" then
    local n = 0
    for _ in pairs(x) do n = n + 1 end
    return n
  end
  error("『文字数』には文字列か配列を指定してください")
end

local function nd_arr(items)
  local t = { [0] = "arr" }
  if items then for i, v in ipairs(items) do t[i] = v end end
  return t
end

local function nd_push(t, v)
  if type(t) ~= "table" or rawget(t, 0) ~= "arr" then error("『配列追加』には配列を指定してください") end
  t[#t + 1] = v
  return t
end

local function nd_sort(t)
  if type(t) ~= "table" or rawget(t, 0) ~= "arr" then error("『配列ソート』には配列を指定してください") end
  table.sort(t)
  return t
end

local function nd_reverse(t)
  if type(t) ~= "table" or rawget(t, 0) ~= "arr" then error("『配列逆順』には配列を指定してください") end
  local o = {}
  o[0] = "arr"
  for i = 1, #t do o[i] = t[#t - i + 1] end
  return o
end

local function nd_trim(s)
  return (tostring(s):gsub("^%s+", ""):gsub("%s+$", ""))
end

local function nd_patesc(s)
  return (tostring(s):gsub("[%^%$%(%)%%%.%[%]%*%+%-%?]", "%%%1"))
end

local function nd_print(x) io.write(nd_tostr(x), "\n") end

local function nd_rand(a, b)
  local lo, hi = math.floor(nd_num(a)), math.floor(nd_num(b))
  if hi < lo then lo, hi = hi, lo end
  return math.random(lo, hi)
end

local function nd_round(x) return math.floor(nd_num(x) + 0.5) end
local function nd_ceil(x) return math.ceil(nd_num(x)) end
local function nd_floor(x) return math.floor(nd_num(x)) end
local function nd_abs(x) return math.abs(nd_num(x)) end
local function nd_sign(x)
  local n = nd_num(x)
  if n > 0 then return 1 elseif n < 0 then return -1 end
  return 0
end

local function nd_replace(s, a, b)
  local rep = tostring(b):gsub("(%%)", "%%%%")
  return (tostring(s):gsub(nd_patesc(a), rep))
end

local function nd_split(s, d)
  local out = nd_arr({})
  local ss = tostring(s)
  if tostring(d) == "" then
    for ch in ss:gmatch("[%z\1-\127\194-\244][\128-\191]*") do out[#out + 1] = ch end
    return out
  end
  local pat = "(" .. nd_patesc(d) .. ")"
  local pos = 1
  while true do
    local a, b = ss:find(pat, pos)
    if not a then break end
    out[#out + 1] = ss:sub(pos, a - 1)
    pos = b + 1
  end
  out[#out + 1] = ss:sub(pos)
  return out
end

local function nd_join(arr, sep)
  if type(arr) ~= "table" then error("『配列接続』には配列を指定してください") end
  local ps = {}
  for i = 1, #arr do ps[i] = nd_tostr(arr[i]) end
  return table.concat(ps, nd_tostr(sep))
end

local function nd_sleep(sec)
  local t0 = os.clock()
  while os.clock() - t0 < nd_num(sec) do end
end

local function nd_jsonenc(x)
  local function enc(v)
    local t = type(v)
    if v == nil then return "null" end
    if t == "boolean" then return tostring(v) end
    if t == "number" then
      if v == math.floor(v) and math.abs(v) < 2^53 then return string.format("%d", v) end
      return tostring(v)
    end
    if t == "string" then
      return '"' .. v:gsub('[%c"\\]', function(c)
        if c == '"' then return '\\"' elseif c == "\\" then return "\\\\" end
        if c == "\n" then return "\\n" elseif c == "\t" then return "\\t" end
        return string.format("\\u%04x", string.byte(c))
      end) .. '"'
    end
    if t == "table" then
      if rawget(v, 0) == "arr" then
        local ps = {}
        for i = 1, #v do ps[i] = enc(v[i]) end
        return "[" .. table.concat(ps, ",") .. "]"
      end
      local keys = {}
      for k in pairs(v) do keys[#keys + 1] = k end
      table.sort(keys, function(a, b) return tostring(a) < tostring(b) end)
      local ps = {}
      for _, k in ipairs(keys) do ps[#ps + 1] = enc(tostring(k)) .. ":" .. enc(v[k]) end
      return "{" .. table.concat(ps, ",") .. "}"
    end
    error("JSON化できない型です")
  end
  return enc(x)
end

local function nd_utf8char(code)
  if code < 0x80 then return string.char(code) end
  if code < 0x800 then
    return string.char(0xC0 + math.floor(code / 0x40), 0x80 + code % 0x40)
  end
  if code < 0x10000 then
    return string.char(0xE0 + math.floor(code / 0x1000), 0x80 + math.floor(code / 0x40) % 0x40, 0x80 + code % 0x40)
  end
  return string.char(0xF0 + math.floor(code / 0x40000), 0x80 + math.floor(code / 0x1000) % 0x40,
    0x80 + math.floor(code / 0x40) % 0x40, 0x80 + code % 0x40)
end

local function nd_jsondec(s)
  if type(s) == "table" then return s end
  local str = tostring(s)
  local pos = 1
  local function skipws()
    while pos <= #str do
      local c = str:sub(pos, pos)
      if c == " " or c == "\t" or c == "\n" or c == "\r" then pos = pos + 1 else break end
    end
  end
  local parse_value
  local function parse_string()
    pos = pos + 1
    local buf = {}
    while pos <= #str do
      local c = str:sub(pos, pos)
      if c == '"' then pos = pos + 1 return table.concat(buf) end
      if c == "\\" then
        local e = str:sub(pos + 1, pos + 1)
        if e == "n" then buf[#buf + 1] = "\n" pos = pos + 2
        elseif e == "t" then buf[#buf + 1] = "\t" pos = pos + 2
        elseif e == "r" then buf[#buf + 1] = "\r" pos = pos + 2
        elseif e == "u" then
          local code = tonumber(str:sub(pos + 2, pos + 5), 16)
          buf[#buf + 1] = nd_utf8char(code)
          pos = pos + 6
        else buf[#buf + 1] = e pos = pos + 2 end
      else
        buf[#buf + 1] = c
        pos = pos + 1
      end
    end
    error("JSON解析エラー: 文字列が閉じていません")
  end
  local function parse_number()
    local a, b = str:find("^-?%d+%.?%d*[eE]?[-+]?%d*", pos)
    if not a then error("JSON解析エラー: 不正な数値です") end
    local n = tonumber(str:sub(a, b))
    pos = b + 1
    return n
  end
  parse_value = function()
    skipws()
    local c = str:sub(pos, pos)
    if c == "{" then
      pos = pos + 1
      local obj = {}
      skipws()
      if str:sub(pos, pos) == "}" then pos = pos + 1 return obj end
      while true do
        skipws()
        local k = parse_string()
        skipws()
        if str:sub(pos, pos) ~= ":" then error("JSON解析エラー: ':'が必要") end
        pos = pos + 1
        obj[k] = parse_value()
        skipws()
        local d = str:sub(pos, pos)
        if d == "," then pos = pos + 1
        elseif d == "}" then pos = pos + 1 return obj
        else error("JSON解析エラー") end
      end
    elseif c == "[" then
      pos = pos + 1
      local arr = nd_arr({})
      skipws()
      if str:sub(pos, pos) == "]" then pos = pos + 1 return arr end
      while true do
        arr[#arr + 1] = parse_value()
        skipws()
        local d = str:sub(pos, pos)
        if d == "," then pos = pos + 1
        elseif d == "]" then pos = pos + 1 return arr
        else error("JSON解析エラー") end
      end
    elseif c == '"' then return parse_string()
    elseif str:sub(pos, pos + 3) == "true" then pos = pos + 4 return true
    elseif str:sub(pos, pos + 4) == "false" then pos = pos + 5 return false
    elseif str:sub(pos, pos + 3) == "null" then pos = pos + 4 return nil
    else return parse_number() end
  end
  local ok, v = pcall(function() return parse_value() end)
  if not ok then error(tostring(v)) end
  return v
end
"#;

impl LuaGen {
    pub fn new() -> Self {
        LuaGen { out: String::new(), ind: 0 }
    }

    fn line(&mut self, s: &str) {
        for _ in 0..self.ind {
            self.out.push_str("  ");
        }
        self.out.push_str(s);
        self.out.push('\n');
    }

    fn open(&mut self, s: &str) {
        self.line(s);
        self.ind += 1;
    }

    fn close(&mut self, s: &str) {
        self.ind -= 1;
        self.line(s);
    }

    fn mid(&mut self, s: &str) {
        self.ind -= 1;
        self.line(s);
        self.ind += 1;
    }
}

pub fn lua_escape(s: &str) -> String {
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
    vid("V", name)
}

pub fn generate(ast: &[Node], funcs: &HashMap<String, Vec<String>>) -> Result<String> {
    let mut g = LuaGen::new();
    g.out.push_str(PRELUDE_LUA);
    g.out.push('\n');
    g.line("math.randomseed(math.floor((os.time() * 1000) + (os.clock() * 1e6)))");
    for n in ast.iter().filter(|n| matches!(n, Node::FuncDef { .. })) {
        gen_funcdef(&mut g, n, funcs)?;
    }
    for n in ast.iter().filter(|n| !matches!(n, Node::FuncDef { .. })) {
        gen_stmt(&mut g, n, funcs)?;
    }
    Ok(g.out)
}

fn gen_funcdef(g: &mut LuaGen, node: &Node, funcs: &HashMap<String, Vec<String>>) -> Result<()> {
    let Node::FuncDef { name, params, body } = node else { unreachable!() };
    let args: Vec<String> = params.iter().map(|p| var(p)).collect();
    g.open(&format!("function {}({})", vid("UF", name), args.join(", ")));
    gen_block(g, body, funcs)?;
    g.line(&format!("return {}", var(SORE)));
    g.close("end");
    Ok(())
}

fn gen_block(g: &mut LuaGen, stmts: &[Node], funcs: &HashMap<String, Vec<String>>) -> Result<()> {
    for s in stmts {
        gen_stmt(g, s, funcs)?;
    }
    Ok(())
}

fn loop_open(g: &mut LuaGen) {
    g.line("__br = false");
    g.open("repeat");
    g.line("if __br then break end");
}

fn loop_close(g: &mut LuaGen) {
    g.close("until true");
    g.line("if __br then break end");
}

fn gen_stmt(g: &mut LuaGen, node: &Node, funcs: &HashMap<String, Vec<String>>) -> Result<()> {
    match node {
        Node::Expr(e) => {
            let v = gen_expr(g, e, funcs)?;
            g.line(&format!("{} = {}", var(SORE), v));
        }
        Node::Assign { target, value } => match target.as_ref() {
            Node::Var(name) => {
                let v = gen_expr(g, value, funcs)?;
                g.line(&format!("{} = {}", var(name), v));
            }
            Node::Index(base, idx) => {
                let bv = gen_expr(g, base, funcs)?;
                let iv = gen_expr(g, idx, funcs)?;
                let vv = gen_expr(g, value, funcs)?;
                g.line(&format!("nd_setidx({}, {}, {})", bv, iv, vv));
            }
            other => {
                return Err(NadeError::new(
                    format!("代入先として不正です: {:?}", other),
                    0,
                ))
            }
        },
        Node::If { cond, then_body, else_body } => {
            let c = gen_expr(g, cond, funcs)?;
            g.open(&format!("if {} then", c));
            gen_block(g, then_body, funcs)?;
            if !else_body.is_empty() {
                g.mid("else");
                gen_block(g, else_body, funcs)?;
            }
            g.close("end");
        }
        Node::RepeatTimes { count, body } => {
            let c = gen_expr(g, count, funcs)?;
            g.line(&format!("for __i = 1, nd_num({}) do", c));
            g.ind += 1;
            g.line(&format!("{} = __i", var(KAISU)));
            loop_open(g);
            gen_block(g, body, funcs)?;
            loop_close(g);
            g.ind -= 1;
            g.line("end");
        }
        Node::RepeatRange { from, to, body } => {
            let f = gen_expr(g, from, funcs)?;
            let t = gen_expr(g, to, funcs)?;
            g.line(&format!("do local __f, __t = nd_num({}), nd_num({})", f, t));
            g.line("__f, __t = math.floor(__f), math.floor(__t)");
            g.open("if __f <= __t then");
            emit_range_loop(g, funcs, body, "__i = __f, __t")?;
            g.mid("else");
            emit_range_loop(g, funcs, body, "__i = __f, __t, -1")?;
            g.close("end");
            g.line("end");
        }
        Node::ForEach { arr, body } => {
            let a = gen_expr(g, arr, funcs)?;
            g.line(&format!("do local __arr_snapshot = {}", a));
            g.open("for __k = 1, #__arr_snapshot do");
            g.line(&format!("{} = __arr_snapshot[__k]", var(TAISHO)));
            loop_open(g);
            gen_block(g, body, funcs)?;
            loop_close(g);
            g.close("end");
            g.line("end");
        }
        Node::While { cond, body } => {
            let c = gen_expr(g, cond, funcs)?;
            g.open(&format!("while {} do", c));
            loop_open(g);
            gen_block(g, body, funcs)?;
            loop_close(g);
            g.close("end");
        }
        Node::Return(e) => match e {
            Some(expr) => {
                let v = gen_expr(g, expr, funcs)?;
                g.line(&format!("{} = {}", var(SORE), v));
                g.line(&format!("return {}", var(SORE)));
            }
            None => g.line(&format!("return {}", var(SORE))),
        },
        Node::Break => g.line("__br = true; break"),
        Node::Continue => g.line("break"),
        Node::FuncDef { name, params, body } => gen_funcdef_nested(g, name, params, body, funcs)?,
        Node::Null | Node::Num(_) | Node::Str(_) | Node::Bool(_) | Node::Var(_)
        | Node::Arr(_) | Node::Obj(_) | Node::Bin(..) | Node::Un(..) | Node::Index(..)
        | Node::Call { .. } => {
            let v = gen_expr(g, node, funcs)?;
            g.line(&format!("{} = {}", var(SORE), v));
        }
    }
    Ok(())
}

fn gen_funcdef_nested(
    g: &mut LuaGen,
    name: &str,
    params: &[String],
    body: &[Node],
    funcs: &HashMap<String, Vec<String>>,
) -> Result<()> {
    let args: Vec<String> = params.iter().map(|p| var(p)).collect();
    g.open(&format!("function {}({})", vid("UF", name), args.join(", ")));
    gen_block(g, body, funcs)?;
    g.line(&format!("return {}", var(SORE)));
    g.close("end");
    Ok(())
}

fn emit_range_loop(
    g: &mut LuaGen,
    funcs: &HashMap<String, Vec<String>>,
    body: &[Node],
    header: &str,
) -> Result<()> {
    g.open(&format!("for {} do", header));
    g.line(&format!("{} = __i", var(TAISHO)));
    loop_open(g);
    gen_block(g, body, funcs)?;
    loop_close(g);
    g.close("end");
    Ok(())
}

fn gen_call(
    g: &mut LuaGen,
    name: &str,
    args: &[(String, Node)],
    funcs: &HashMap<String, Vec<String>>,
) -> Result<String> {
    if let Some(params) = funcs.get(name) {
        let vals = resolve_user(name, params, args)?;
        let mut out = Vec::new();
        for v in &vals {
            out.push(gen_expr(g, v, funcs)?);
        }
        return Ok(format!("{}({})", vid("UF", name), out.join(", ")));
    }
    if let Some(b) = builtins::lookup(name) {
        let vals = if let Some(p) = resolve_positional(name, b.arity(), args) {
            p
        } else {
            resolve_builtin(b.name, b.params, args)?
        };
        let mut out = Vec::new();
        for v in &vals {
            out.push(gen_expr(g, v, funcs)?);
        }
        return map_builtin(name, &out);
    }
    Err(NadeError::new(format!("未知の命令または関数『{}』", name), 0))
}

fn map_builtin(name: &str, args: &[String]) -> Result<String> {
    let a = |i: usize| -> String { args.get(i).cloned().unwrap_or_else(|| "nil".into()) };
    let call = |fname: &str| format!("nd_{}({})", fname, args.join(", "));
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
        "平方根" => format!("math.sqrt(nd_num({}))", a(0)),
        "サイン" => format!("math.sin(nd_num({}))", a(0)),
        "コサイン" => format!("math.cos(nd_num({}))", a(0)),
        "タンジェント" => format!("math.tan(nd_num({}))", a(0)),
        "文字数" | "配列要素数" => call("len"),
        "切り取る" => {
            format!(
                "nd_utf8sub(nd_tostr({}), math.floor(nd_num({})), math.floor(nd_num({})))",
                a(0), a(1), a(2)
            )
        }
        "置換" => call("replace"),
        "区切る" => call("split"),
        "大文字変換" => format!("tostring({}):upper()", a(0)),
        "小文字変換" => format!("tostring({}):lower()", a(0)),
        "トリム" => call("trim"),
        "配列追加" => call("push"),
        "配列ソート" => call("sort"),
        "配列逆順" => call("reverse"),
        "配列接続" => call("join"),
        "配列取得" => call("idx"),
        "JSONエンコード" => call("jsonenc"),
        "JSONデコード" => call("jsondec"),
        "型取得" => call("type"),
        "待つ" => call("sleep"),
        _ => return Err(NadeError::new(format!("命令『{}』は未実装です", name), 0)),
    })
}

fn gen_expr(
    g: &mut LuaGen,
    node: &Node,
    funcs: &HashMap<String, Vec<String>>,
) -> Result<String> {
    Ok(match node {
        Node::Num(v) => {
            if v.fract() == 0.0 && v.abs() < 9e15 {
                format!("{}", *v as i64)
            } else {
                format!("{}", v)
            }
        }
        Node::Str(s) => format!("\"{}\"", lua_escape(s)),
        Node::Bool(b) => format!("{}", b),
        Node::Null => "nil".into(),
        Node::Var(name) => {
            if funcs.contains_key(name) && !builtins::lookup(name).is_some() {
                format!("{}()", vid("UF", name))
            } else {
                var(name)
            }
        }
        Node::Arr(items) => {
            let mut parts = Vec::new();
            for it in items {
                parts.push(gen_expr(g, it, funcs)?);
            }
            format!("nd_arr({{{}}})", parts.join(","))
        }
        Node::Obj(items) => {
            let mut parts = Vec::new();
            for (k, v) in items {
                parts.push(format!("[\"{}\"]={}", lua_escape(k), gen_expr(g, v, funcs)?));
            }
            format!("{{{}}}", parts.join(","))
        }
        Node::Bin(op, l, r) => {
            let lv = gen_expr(g, l, funcs)?;
            let rv = gen_expr(g, r, funcs)?;
            match op.as_str() {
                "+" => format!("nd_add({}, {})", lv, rv),
                "-" => format!("nd_sub({}, {})", lv, rv),
                "*" => format!("nd_mul({}, {})", lv, rv),
                "/" => format!("nd_div({}, {})", lv, rv),
                "%" => format!("nd_mod({}, {})", lv, rv),
                "^" => format!("nd_pow({}, {})", lv, rv),
                ".." => format!("nd_tostr({}) .. nd_tostr({})", lv, rv),
                "==" => format!("nd_eq({}, {})", lv, rv),
                "!=" => format!("not nd_eq({}, {})", lv, rv),
                "<" => format!("nd_cmp_lt({}, {})", lv, rv),
                ">" => format!("nd_cmp_lt({}, {})", rv, lv),
                "<=" => format!("nd_le({}, {})", lv, rv),
                ">=" => format!("nd_le({}, {})", rv, lv),
                "&&" => format!("{} and {}", lv, rv),
                "||" => format!("{} or {}", lv, rv),
                other => return Err(NadeError::new(format!("未知の演算子『{}』", other), 0)),
            }
        }
        Node::Un(op, e) => {
            let ev = gen_expr(g, e, funcs)?;
            match op.as_str() {
                "-" => format!("(-nd_num({}))", ev),
                "!" => format!("(not {})", ev),
                other => return Err(NadeError::new(format!("未知の単項演算子『{}』", other), 0)),
            }
        }
        Node::Index(base, idx) => {
            format!("nd_idx({}, {})", gen_expr(g, base, funcs)?, gen_expr(g, idx, funcs)?)
        }
        Node::Call { name, args } => gen_call(g, name, args, funcs)?,
        Node::Expr(inner) => gen_expr(g, inner, funcs)?,
        other => {
            return Err(NadeError::new(
                format!("式として扱えないノード: {:?}", other),
                0,
            ))
        }
    })
}
