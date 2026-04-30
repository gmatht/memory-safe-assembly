use std::env;
use std::path::PathBuf;

use z3::{Config, Context};

fn main() {
    env_logger::init();

    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: verify_asm <asm-file> [--run <label>]");
        std::process::exit(2);
    }

    let asm_path = PathBuf::from(&args[1]);
    if !asm_path.exists() {
        eprintln!("File not found: {}", asm_path.display());
        std::process::exit(3);
    }

    // Initialize Z3 context
    let mut cfg = Config::new();
    // set a reasonable timeout for solver queries to avoid very long runs
    cfg.set_timeout_msec(500);
    let ctx = Context::new(&cfg);

    // Use the x86 engine to parse the given file
    let mut engine = bums::x86_64::engine::ExecutionEngineX86::from_asm_file(
        asm_path.to_str().unwrap(),
        &ctx,
    );

    println!("Parsed {} instructions", engine.program.len());
    println!("Labels ({}):", engine.label_map.len());
    for (k, v) in engine.label_map.iter() {
        println!("  {} -> {}", k, v);
    }

    println!("Memory labels:");
    for s in engine.dump_memory_labels() {
        println!("  {}", s);
    }

    println!("Relocations (as JSON):");
    println!("{}", engine.export_relocations_json());

    // Optionally run symbolic execution from a label
    if args.len() >= 4 && args[2] == "--run" {
        let label = &args[3];
        println!("Running symbolic execution from label '{}'...", label);
        match engine.start(label) {
            Ok(_) => println!("Execution finished successfully"),
            Err(e) => eprintln!("Execution error: {}", e),
        }
    }
}
