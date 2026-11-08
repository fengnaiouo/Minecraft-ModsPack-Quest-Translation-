/// FNV-1a 64 bit, used to build stable string ids without extra dependencies.
pub fn stable_id(parts: &[&str]) -> String {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for part in parts {
        for byte in part.as_bytes() {
            hash ^= u64::from(*byte);
            hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
        }
        // separator so ("ab","c") and ("a","bc") differ
        hash ^= 0xff;
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("{hash:016x}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_are_stable_and_distinct() {
        let a = stable_id(&["file", "quests[0].title", "Hello"]);
        let b = stable_id(&["file", "quests[0].title", "Hello"]);
        let c = stable_id(&["file", "quests[1].title", "Hello"]);
        assert_eq!(a, b);
        assert_ne!(a, c);
    }
}
