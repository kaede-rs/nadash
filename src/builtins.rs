pub struct Builtin {
    pub name: &'static str,
    pub params: &'static [(&'static str, &'static str)],
}

impl Builtin {
    pub fn arity(&self) -> usize {
        self.params.len()
    }
}

pub static BUILTINS: &[Builtin] = &[
    Builtin { name: "表示", params: &[("と|を", "v")] },
    Builtin { name: "言う", params: &[("と|を", "v")] },
    Builtin { name: "足す", params: &[("と", "a"), ("を", "b")] },
    Builtin { name: "引く", params: &[("から", "a"), ("を", "b")] },
    Builtin { name: "掛ける", params: &[("と", "a"), ("を", "b")] },
    Builtin { name: "割る", params: &[("を", "a"), ("で", "b")] },
    Builtin { name: "余り", params: &[("を", "a"), ("で", "b")] },
    Builtin { name: "足し算", params: &[("と", "a"), ("を", "b")] },
    Builtin { name: "引き算", params: &[("から", "a"), ("を", "b")] },
    Builtin { name: "掛け算", params: &[("と", "a"), ("を", "b")] },
    Builtin { name: "割り算", params: &[("を", "a"), ("で", "b")] },
    Builtin { name: "乱数", params: &[("から", "a"), ("まで", "b")] },
    Builtin { name: "四捨五入", params: &[("を", "v")] },
    Builtin { name: "切り上げ", params: &[("を", "v")] },
    Builtin { name: "切り下げ", params: &[("を", "v")] },
    Builtin { name: "絶対値", params: &[("の", "v")] },
    Builtin { name: "符号", params: &[("の", "v")] },
    Builtin { name: "平方根", params: &[("の", "v")] },
    Builtin { name: "サイン", params: &[("の", "v")] },
    Builtin { name: "コサイン", params: &[("の", "v")] },
    Builtin { name: "タンジェント", params: &[("の", "v")] },
    Builtin { name: "文字数", params: &[("の", "s")] },
    Builtin { name: "切り取る", params: &[("の", "s"), ("から", "a"), ("まで", "b")] },
    Builtin { name: "置換", params: &[("の", "s"), ("から", "a"), ("へ|に", "b")] },
    Builtin { name: "区切る", params: &[("を", "s"), ("で", "d")] },
    Builtin { name: "大文字変換", params: &[("を", "s")] },
    Builtin { name: "小文字変換", params: &[("を", "s")] },
    Builtin { name: "トリム", params: &[("を", "s")] },
    Builtin { name: "配列追加", params: &[("へ|に", "arr"), ("を", "v")] },
    Builtin { name: "配列要素数", params: &[("の", "arr")] },
    Builtin { name: "配列ソート", params: &[("の", "arr")] },
    Builtin { name: "配列逆順", params: &[("の", "arr")] },
    Builtin { name: "配列接続", params: &[("の", "arr"), ("を", "sep")] },
    Builtin { name: "配列取得", params: &[("の", "arr"), ("から|に|を", "i")] },
    Builtin { name: "JSONエンコード", params: &[("を|の", "v")] },
    Builtin { name: "JSONデコード", params: &[("を|の", "s")] },
    Builtin { name: "型取得", params: &[("の", "v")] },
    Builtin { name: "待つ", params: &[("を|の", "sec")] },
];

pub fn lookup(name: &str) -> Option<&'static Builtin> {
    BUILTINS.iter().find(|b| b.name == name)
}
