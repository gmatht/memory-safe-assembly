use bums::x86_64::ExecutionEngineX86;
use std::io::Write;
use z3::Config;

#[test]
fn export_linker_relocs_file_has_symbol() {
    let mut tmp = std::env::temp_dir();
    tmp.push("test_export_linker_relocs.s");
    let mut f = std::fs::File::create(&tmp).expect("create tmp asm");
    let content = ".quad .L_symbol + 4\n";
    f.write_all(content.as_bytes()).expect("write");
    f.flush().unwrap();

    let cfg = Config::new();
    let ctx = z3::Context::new(&cfg);
    let engine = ExecutionEngineX86::from_asm_file(tmp.to_str().unwrap(), &ctx);
    let mut out = std::env::temp_dir();
    out.push("linker_relocs.json");
    let path = out.to_str().expect("path");
    engine.export_linker_relocs_to_file(path).expect("write");
    let s = std::fs::read_to_string(path).expect("read");
    assert!(s.contains(".L_symbol"));
    assert!(s.contains("SYM_ADD") || s.contains("EXPR") || s.contains("DIFF"));
}
