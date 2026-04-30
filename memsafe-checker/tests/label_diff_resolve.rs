use bums::x86_64::ExecutionEngineX86;
use std::io::Write;
use z3::Config;

#[test]
fn label_diff_resolves_to_concrete() {
    let mut tmp = std::env::temp_dir();
    tmp.push("test_label_diff.s");
    let mut f = std::fs::File::create(&tmp).expect("create tmp asm");
    let content = r#".L_a:
.byte 0x01
.L_b:
.byte 0x02
.long .L_b - .L_a
"#;
    f.write_all(content.as_bytes()).expect("write");
    f.flush().unwrap();

    let cfg = Config::new();
    let ctx = z3::Context::new(&cfg);
    let engine = ExecutionEngineX86::from_asm_file(tmp.to_str().unwrap(), &ctx);

    let a = engine.computer.memory_labels.get(".L_a").cloned().unwrap();
    let b = engine.computer.memory_labels.get(".L_b").cloned().unwrap();
    // the long storing b - a should be at address after .L_b (which is b)
    let long_addr = b + 1; // .long emitted after the single byte at .L_b

    if let Some(mem) = engine.computer.memory.get("memory") {
        let b0 = mem.get(long_addr).expect("b0").offset as u8;
        let b1 = mem.get(long_addr + 1).expect("b1").offset as u8;
        let b2 = mem.get(long_addr + 2).expect("b2").offset as u8;
        let b3 = mem.get(long_addr + 3).expect("b3").offset as u8;
        let val = (b3 as u32) << 24 | (b2 as u32) << 16 | (b1 as u32) << 8 | (b0 as u32);
        assert_eq!(val, (b - a) as u32);
    } else {
        panic!("memory region missing");
    }
}
