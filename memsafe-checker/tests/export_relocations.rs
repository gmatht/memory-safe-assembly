use bums::x86_64::ExecutionEngineX86;
use std::io::Write;
use z3::Config;

#[test]
fn export_relocations_contains_label() {
    let mut tmp = std::env::temp_dir();
    tmp.push("test_export_reloc.s");
    let mut f = std::fs::File::create(&tmp).expect("create tmp asm");
    let content = ".quad .L_my_label + 8\n";
    f.write_all(content.as_bytes()).expect("write");
    f.flush().unwrap();

    let cfg = Config::new();
    let ctx = z3::Context::new(&cfg);
    let mut engine = ExecutionEngineX86::from_asm_file(tmp.to_str().unwrap(), &ctx);
    // ensure reloc recorded
    let rels = engine.take_relocations();
    assert!(rels.len() >= 1);
    // re-run from_asm_file to populate relocations again for export
    let engine = ExecutionEngineX86::from_asm_file(tmp.to_str().unwrap(), &ctx);
    let json = engine.export_relocations_json();
    assert!(json.contains(".L_my_label"));

    // also test writing to a file
    let mut out = std::env::temp_dir();
    out.push("export_reloc_out.json");
    let path = out.to_str().expect("temp path");
    engine.export_relocations_to_file(path).expect("write file");
    let contents = std::fs::read_to_string(path).expect("read");
    assert!(contents.contains(".L_my_label"));
}
