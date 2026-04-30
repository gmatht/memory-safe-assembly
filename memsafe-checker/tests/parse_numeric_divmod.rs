use std::io::Write;
use z3::Config;

#[test]
fn numeric_token_division_and_modulo() {
    let mut tmp = std::env::temp_dir();
    tmp.push("test_numeric_divmod.s");
    let mut f = std::fs::File::create(&tmp).expect("create tmp asm");
    let content = ".L_dm:\n.hword 100 / 4\n.hword 103 % 100\n";
    f.write_all(content.as_bytes()).expect("write");
    f.flush().unwrap();

    let cfg = Config::new();
    let ctx = z3::Context::new(&cfg);
    let engine =
        bums::x86_64::engine::ExecutionEngineX86::from_asm_file(tmp.to_str().unwrap(), &ctx);
    let addr = engine
        .computer
        .memory_labels
        .get(".L_dm")
        .cloned()
        .expect("label");
    if let Some(mem) = engine.computer.memory.get("memory") {
        let b0 = mem.get(addr).expect("b0").offset as u8;
        let b1 = mem.get(addr + 1).expect("b1").offset as u8;
        let val = (b1 as u16) << 8 | (b0 as u16);
        assert_eq!(val, 25);

        let b2 = mem.get(addr + 2).expect("b2").offset as u8;
        let b3 = mem.get(addr + 3).expect("b3").offset as u8;
        let val2 = (b3 as u16) << 8 | (b2 as u16);
        assert_eq!(val2, 3);
    } else {
        panic!("memory missing");
    }
}
