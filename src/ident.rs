pub const SORE: &str = "それ";
pub const KAISU: &str = "回数";
pub const TAISHO: &str = "対象";

pub fn fnv64(s: &str) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for b in s.as_bytes() {
        h ^= *b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

pub fn vid(prefix: &str, name: &str) -> String {
    format!("{}_{:016x}", prefix, fnv64(name))
}
