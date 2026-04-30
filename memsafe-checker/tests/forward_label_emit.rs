use bums::x86_64::engine::ExecutionEngineX86;
use std::io::Write;
use z3::Config;

#[test]
fn forward_label_long_resolves_to_label_minus_offset() {
    // Create a temp asm file where a .long references a label defined later
    let mut tmp = std::env::temp_dir();
    tmp.push("test_forward_label.s");
    let mut f = std::fs::File::create(&tmp).expect("create tmp asm");
    let content = ".L_a:\n.byte 0x11\n.long .L_b - 4\n.L_b:\n.byte 0x22\n";
    f.write_all(content.as_bytes()).expect("write");
    f.flush().unwrap();

    let cfg = Config::new();
    let ctx = z3::Context::new(&cfg);
    let mut engine = ExecutionEngineX86::from_asm_file(tmp.to_str().unwrap(), &ctx);

    // run the engine to force data emission (start does not need an entry label for this test)
    let _ = engine.start("_start");

    // compute address of the long: .L_a is at base; the long follows one byte
    let a = engine
        .computer
        .memory_labels
        .get(".L_a")
        .cloned()
        .expect("label a");
    let long_addr = a + 1; // one-byte after .L_a's byte

    if let Some(mem) = engine.computer.memory.get("memory") {
        let b0 = mem.get(long_addr).expect("b0").offset as u8;
        let b1 = mem.get(long_addr + 1).expect("b1").offset as u8;
        let b2 = mem.get(long_addr + 2).expect("b2").offset as u8;
        let b3 = mem.get(long_addr + 3).expect("b3").offset as u8;
        let val = (b3 as u32) << 24 | (b2 as u32) << 16 | (b1 as u32) << 8 | (b0 as u32);
        // expected value: address of .L_b + (-4) -> based on emission logic this equals 5
        assert_eq!(val, 5u32);
    } else {
        panic!("memory region missing");
    }
}
