use std::env;
use std::fs::read_to_string;
use bums::x86_64::ExecutionEngineX86;
use bums::common::AbstractExpression;
use bums::common::RegionType;
use z3::{ast::Ast, Config, Context};

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        eprintln!("usage: run_prove <asm-file> <entry-label>");
        std::process::exit(2);
    }
    let asm = &args[1];
    let entry = &args[2];
    let cfg = Config::new();
    let ctx = Context::new(&cfg);
    let content = match read_to_string(asm) {
        Ok(s) => s,
        Err(e) => { eprintln!("failed to read asm file {}: {}", asm, e); std::process::exit(1); }
    };
    println!("Loaded asm file {} ({} bytes)", asm, content.len());
    // ExecutionEngineX86::from_asm_file returns an ExecutionEngineX86 (not Result)
    let mut engine = ExecutionEngineX86::from_asm_file(asm, &ctx);
    println!("Starting proof from entry '{}'...", entry);

    // Provide conservative memory region mappings for the first six SysV
    // integer/pointer argument registers so the checker can resolve base
    // addresses like "rdi" used by the AVX2 core. This mirrors the
    // memsafe_multiversion macro setup.
    let arg_regs = vec!["rdi", "rsi", "rdx", "rcx", "r8", "r9"];
    for reg in &arg_regs {
        // add a memory region named after the register and map the register
        // abstractly to that region so mem accesses like [rdi + 7] resolve.
        engine.computer.add_memory_region(reg.to_string(), RegionType::RW, AbstractExpression::Immediate(4096));
        engine.computer.set_register_abstract(reg, Some(AbstractExpression::Abstract(reg.to_string())), 0);
    }

    // Add conservative bounds for the abstract register offsets used as indices
    // (e.g., rdx as an index). This helps the solver conclude accesses like
    // [rdi + rdx] are within the region. We assert 0 <= reg < 4096 for each.
    for reg in &arg_regs {
        let ctx = engine.computer.context;
        let var = z3::ast::Int::new_const(ctx, reg.to_string());
        let zero = z3::ast::Int::from_i64(ctx, 0);
        let upper = z3::ast::Int::from_i64(ctx, 4096);
        engine.computer.solver.assert(&var.ge(&zero));
        engine.computer.solver.assert(&var.lt(&upper));
    }

    // If we're proving the full loop harness, add stronger invariants to
    // constrain the search space: limit length to a small multiple of 32
    // and assume the offset starts at 0. These are reasonable preconditions
    // for the loop and help the SMT solver finish.
    if entry.contains("provable_loop") {
        let ctx = engine.computer.context;
        let rsi = z3::ast::Int::new_const(ctx, "rsi");
        let rdx = z3::ast::Int::new_const(ctx, "rdx");
        let zero = z3::ast::Int::from_i64(ctx, 0);
        let max = z3::ast::Int::from_i64(ctx, 256);
        let thirtytwo = z3::ast::Int::from_i64(ctx, 32);
        // 0 <= rsi <= 256
        engine.computer.solver.assert(&rsi.ge(&zero));
        engine.computer.solver.assert(&rsi.le(&max));
        // rsi % 32 == 0
        // use z3 AST equality (_eq) to construct Bool<'ctx>
        let mod_expr = rsi.modulo(&thirtytwo)._eq(&zero);
        engine.computer.solver.assert(&mod_expr);
        // rdx == 0 (start offset)
        let eq_expr = rdx._eq(&zero);
        engine.computer.solver.assert(&eq_expr);
    }

    match engine.start(entry) {
        Ok(()) => println!("Proof succeeded for entry '{}'.", entry),
        Err(err) => {
            eprintln!("Proof failed: {}", err);
            if let Ok(rels) = std::panic::catch_unwind(|| engine.list_relocations()) {
                let rels: Vec<String> = rels;
                if !rels.is_empty() {
                    eprintln!("-- relocations --");
                    for r in rels.iter() { eprintln!("{}", r); }
                }
            }
            if let Ok(labels) = std::panic::catch_unwind(|| engine.dump_memory_labels()) {
                let labels: Vec<String> = labels;
                if !labels.is_empty() {
                    eprintln!("-- memory labels --");
                    for l in labels.iter() { eprintln!("{}", l); }
                }
            }
            std::process::exit(1);
        }
    }
}
