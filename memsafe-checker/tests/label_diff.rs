use std::io::Write;
use z3::Config;

#[test]
fn label_minus_label_emits_difference() {
    let mut tmp = std::env::temp_dir();
    tmp.push("test_label_diff.s");
    let mut f = std::fs::File::create(&tmp).expect("create tmp asm");
    // layout: .L_a at base, then a .long (.L_b - .L_a) which should compute to offset
    let content = ".L_a:\n.byte 0x00\n.L_b:\n.byte 0x00\n.long .L_b - .L_a\n";
    f.write_all(content.as_bytes()).expect("write");
    f.flush().unwrap();

    let cfg = Config::new();
    let ctx = z3::Context::new(&cfg);
    let engine =
        bums::x86_64::engine::ExecutionEngineX86::from_asm_file(tmp.to_str().unwrap(), &ctx);

    let a = engine
        .computer
        .memory_labels
        .get(".L_a")
        .cloned()
        .expect("label a");
    let b = engine
        .computer
        .memory_labels
        .get(".L_b")
        .cloned()
        .expect("label b");

    let long_addr = b + 1; // long emitted after .L_b's byte
                           // sanity: .L_b should be after .L_a
    assert!(b > a);
    if let Some(mem) = engine.computer.memory.get("memory") {
        let b0 = mem.get(long_addr).expect("b0").offset as u8;
        let b1 = mem.get(long_addr + 1).expect("b1").offset as u8;
        let b2 = mem.get(long_addr + 2).expect("b2").offset as u8;
        let b3 = mem.get(long_addr + 3).expect("b3").offset as u8;
        let val = (b3 as u32) << 24 | (b2 as u32) << 16 | (b1 as u32) << 8 | (b0 as u32);
        // expected difference: .L_b - .L_a = (a+1) - a = 1 (since each label had a byte)
        assert_eq!(val, 1u32);
    } else {
        panic!("memory region missing");
    }
}
