use bums::x86_64::instruction_parser::*;
use std::path::PathBuf;

#[test]
fn parse_icx_sample() {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("../../icx_core_avx2_v3_gas.s");
    // path points to repo root; if file missing the test will skip
    if !path.exists() {
        return;
    }
    let s = std::fs::read_to_string(path).expect("read");
    let mut count = 0usize;
    for line in s.lines() {
        if let Some(ins) = parse_x86_line(line) {
            // just ensure opcode parsed
            assert!(!ins.opcode.is_empty());
            count += 1;
        }
    }
    assert!(count > 0);
}
