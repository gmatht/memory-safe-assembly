use std::io::Write;
use z3::Config;

#[test]
fn align_variants_behave_as_expected() {
    let mut tmp = std::env::temp_dir();
    tmp.push("test_align_variants.s");
    let mut f = std::fs::File::create(&tmp).expect("create tmp asm");
    let content = r#"
.L_a:
.byte 0x01
.align 2
.L_b:
.byte 0x02
.align 16
.L_c:
.byte 0x03
"#;
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
    let c = engine
        .computer
        .memory_labels
        .get(".L_c")
        .cloned()
        .expect("label c");

    // initial base is 4
    assert_eq!(a, 4);
    // after .byte (1) and align 2 (2 -> 1<<2 == 4), .L_b should be at address 8
    assert_eq!(b, 8);
    // after .byte at 8 -> 9, .align 16 (treated as byte count) aligns to 16
    assert_eq!(c, 16);
}
