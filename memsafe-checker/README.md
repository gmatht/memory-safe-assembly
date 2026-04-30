## Symbolic Execution of Assembly for Checking Memory Safety

Multiple test examples of how to use this library without a wrapping macro can be found in [tests](tests/examples.rs).

Note: the computer model is a wip and cannot currently handle the entire Aarch64 ISA.

#### Usage 
1. Configure and initialize a Z3 context:
    ```rust
    use bums;
    use z3::*;

   let cfg = Config::new();
    let ctx = Context::new(&cfg);
    ```
2. Initialize an engine with a program (as Vec<String>) and the context:
```rust
    let start_label = "test".to_string();
    program.push(start_label);
    program.push("add x0,x0,#1".to_string());
    program.push("ret".to_string());
   
    let mut engine = bums::engine::ExecutionEngine::new(program, &ctx);
```

3. Initialize any known machine state, such as register values or memory
```rust
    engine.add_immediate(String::from("x0"), 1);
    engine.add_region(RegionType::READ, "Input".to_string(), 64);
```

4. Run symbolic execution and handle the result
```rust
    let res = engine.start(start_label);
```

#### Contents
- [engine](src/engine.rs) handles symbolic execution, including running instructions, control flow, and loop acceleration
- [computer](src/computer.rs) is a model of an Arm Cortex-A computer which transforms and returns values with an ```execute``` function
- [memory safety checks](src/computer/memory.rs) are handled within the computer logic on loads and stores
- [parser](src/instruction_parser.rs) parses unstructured string inputs into an instruction type

Notes about x86 data parsing and endianness
- The x86 backend stores multi-byte directives (.word/.long/.quad/.hword) in little-endian byte order into the per-byte MemorySafeRegion. This enables conservative per-byte reasoning for SIMD/vector loads.
- .align handling: small numeric arguments (<=8) are treated as exponents (1 << n). Larger numeric arguments are treated as explicit byte counts.
- String directives (.asciz/.string/.ascii) support common escapes (\\\", \\\\\\, \\n, \\t, \\r, \\0) and hex escapes (\\xNN). Commas and spaces inside quoted strings are preserved.

Two-pass data emission and conservative fallback
 - The x86 emitter performs data emission in two passes. The first pass scans directives and labels to compute addresses and record emission descriptors without writing bytes into memory. The second pass resolves label-offset emissions and writes concrete bytes into the per-byte memory map.
 - For tokens that reference labels (e.g., "LABEL - 320b"), the second pass will resolve the label address and emit the numeric value in little-endian bytes at the intended location when possible. If a label remains unresolved after the first pass, the emitter writes conservative zero bytes at the intended addresses so that memory region sizes are preserved and analysis remains conservative.
 - This approach allows forward references to labels (labels defined later in the file) to be relocated correctly while keeping a safe fallback when labels cannot be resolved.
