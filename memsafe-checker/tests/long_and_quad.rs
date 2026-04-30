use std::io::Write;
use z3::Config;

#[test]
fn long_and_quad_store_little_endian_bytes() {
    let mut tmp = std::env::temp_dir();
    tmp.push("test_long_quad.s");
    let mut f = std::fs::File::create(&tmp).expect("create tmp asm");
    let content = r#".L_w:
.long 0x11223344
.quad 0x0102030405060708
"#;
    f.write_all(content.as_bytes()).expect("write");
    f.flush().unwrap();

    let cfg = Config::new();
    let ctx = z3::Context::new(&cfg);
    let engine =
        bums::x86_64::engine::ExecutionEngineX86::from_asm_file(tmp.to_str().unwrap(), &ctx);
    let addr = engine
        .computer
        .memory_labels
        .get(".L_w")
        .cloned()
        .expect("label");
    if let Some(mem) = engine.computer.memory.get("memory") {
        // check first four bytes are 44 33 22 11 (little-endian)
        let b0 = mem.get(addr).expect("b0");
        let b1 = mem.get(addr + 1).expect("b1");
        let b2 = mem.get(addr + 2).expect("b2");
        let b3 = mem.get(addr + 3).expect("b3");
        assert_eq!(b0.offset as u8, 0x44);
        assert_eq!(b1.offset as u8, 0x33);
        assert_eq!(b2.offset as u8, 0x22);
        assert_eq!(b3.offset as u8, 0x11);

        // next eight bytes are for the quad
        let qstart = addr + 4;
        let qb: Vec<u8> = (0..8)
            .map(|i| mem.get(qstart + i).expect("qb").offset as u8)
            .collect();
        assert_eq!(qb, vec![0x08, 0x07, 0x06, 0x05, 0x04, 0x03, 0x02, 0x01]);
    } else {
        panic!("memory region missing");
    }
}
