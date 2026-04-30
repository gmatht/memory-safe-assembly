use bums::x86_64::ExecutionEngineX86;
use std::io::Write;
use z3::Config;

#[test]
fn unresolved_label_records_relocation() {
    let mut tmp = std::env::temp_dir();
    tmp.push("test_reloc.s");
    let mut f = std::fs::File::create(&tmp).expect("create tmp asm");
    let content = ".quad .L_undef + 4\n";
    f.write_all(content.as_bytes()).expect("write");
    f.flush().unwrap();

    let cfg = Config::new();
    let ctx = z3::Context::new(&cfg);
    let mut engine = ExecutionEngineX86::from_asm_file(tmp.to_str().unwrap(), &ctx);
    // should have recorded a relocation for unresolved label
    let rels = engine.take_relocations();
    assert!(rels.len() >= 1);
    let expr = &rels[0].expr;
    let abstracts = expr.get_abstracts();
    assert!(abstracts.iter().any(|s| s.contains(".L_undef")));
}
