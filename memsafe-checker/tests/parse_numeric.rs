use std::io::Write;
use z3::Config;

#[test]
fn numeric_token_parentheses_and_unary() {
    let mut tmp = std::env::temp_dir();
    tmp.push("test_numeric_paren.s");
    let mut f = std::fs::File::create(&tmp).expect("create tmp asm");
    let content = ".L_x:\n.hword (16 * 2)\n.hword -320b\n";
    f.write_all(content.as_bytes()).expect("write");
    f.flush().unwrap();

    let cfg = Config::new();
    let ctx = z3::Context::new(&cfg);
    let engine =
        bums::x86_64::engine::ExecutionEngineX86::from_asm_file(tmp.to_str().unwrap(), &ctx);
    let addr = engine
        .computer
        .memory_labels
        .get(".L_x")
        .cloned()
        .expect("label");
    if let Some(mem) = engine.computer.memory.get("memory") {
        let v0 = mem.get(addr).expect("b0").offset as u8;
        let v1 = mem.get(addr + 1).expect("b1").offset as u8;
        let val = (v1 as u16) << 8 | (v0 as u16);
        assert_eq!(val, 32);
        let v2 = mem.get(addr + 2).expect("b2").offset as u8;
        let v3 = mem.get(addr + 3).expect("b3").offset as u8;
        let val2 = (v3 as u16) << 8 | (v2 as u16);
        // -320 in two's complement low 16 bits -> 0xFE40 -> decimal 65056 but we store as i64 low bytes
        assert_eq!(val2, (-(320i32) as i16) as u16);
    } else {
        panic!("memory missing");
    }
}
