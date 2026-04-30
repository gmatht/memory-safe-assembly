use std::io::Write;
use z3::Config;

#[test]
fn parse_label_minus_number_with_parentheses_falls_back() {
    let mut tmp = std::env::temp_dir();
    tmp.push("test_label_paren.s");
    let mut f = std::fs::File::create(&tmp).expect("create tmp asm");
    let content = ".L_a:\n.byte 0x01\n.long (.L_b) - 4\n.L_b:\n.byte 0x02\n";
    f.write_all(content.as_bytes()).expect("write");
    f.flush().unwrap();

    let cfg = Config::new();
    let ctx = z3::Context::new(&cfg);
    let engine =
        bums::x86_64::engine::ExecutionEngineX86::from_asm_file(tmp.to_str().unwrap(), &ctx);

    // ensure labels exist and memory has been emitted
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

    // basic sanity: .L_b should be after .L_a's byte
    assert!(b > a);
}
