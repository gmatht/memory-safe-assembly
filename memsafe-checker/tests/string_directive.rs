use std::io::Write;
use z3::Config;

#[test]
fn string_directive_handles_commas_and_escapes() {
    let mut tmp = std::env::temp_dir();
    tmp.push("test_string.s");
    let mut f = std::fs::File::create(&tmp).expect("create tmp asm");
    let content = r#".L_s:
.asciz "Hello, world\"!"
"#;
    f.write_all(content.as_bytes()).expect("write");
    f.flush().unwrap();

    let cfg = Config::new();
    let ctx = z3::Context::new(&cfg);
    let engine =
        bums::x86_64::engine::ExecutionEngineX86::from_asm_file(tmp.to_str().unwrap(), &ctx);
    // label exists
    let addr = engine
        .computer
        .memory_labels
        .get(".L_s")
        .cloned()
        .expect("label");
    if let Some(mem) = engine.computer.memory.get("memory") {
        // read bytes until zero terminator
        let mut read = Vec::new();
        let mut a = addr;
        loop {
            if let Some(rv) = mem.get(a) {
                if rv.kind == bums::common::RegisterKind::Immediate {
                    let b = rv.offset as u8;
                    if b == 0 {
                        break;
                    }
                    read.push(b);
                } else {
                    break;
                }
            } else {
                break;
            }
            a += 1;
        }
        let s = String::from_utf8(read).expect("utf8");
        assert_eq!(s, "Hello, world\"!");
    } else {
        panic!("memory region missing");
    }
}
