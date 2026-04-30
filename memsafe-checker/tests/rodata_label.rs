use std::io::Write;
use z3::Config;

#[test]
fn rodata_label_and_bytes_parsed() {
    let mut tmp = std::env::temp_dir();
    tmp.push("test_rodata.s");
    let mut f = std::fs::File::create(&tmp).expect("create tmp asm");
    let content = ".section .rodata\n.rodata_label:\n.byte 0x11, 0x22, 0x33\n";
    f.write_all(content.as_bytes()).expect("write");
    f.flush().unwrap();

    let cfg = Config::new();
    let ctx = z3::Context::new(&cfg);
    let engine =
        bums::x86_64::engine::ExecutionEngineX86::from_asm_file(tmp.to_str().unwrap(), &ctx);
    // the memory_labels should contain the rodata label
    assert!(engine.computer.memory_labels.contains_key(".rodata_label"));
    // and the backing memory region should have three bytes starting at that address
    let addr = engine
        .computer
        .memory_labels
        .get(".rodata_label")
        .cloned()
        .expect("label addr");
    if let Some(mem) = engine.computer.memory.get("memory") {
        // first byte should be 0x11
        let b0 = mem.get(addr).expect("byte0");
        assert_eq!(b0.kind, bums::common::RegisterKind::Immediate);
        assert_eq!(b0.offset as i64, 0x11);
    } else {
        panic!("memory region missing");
    }
}
