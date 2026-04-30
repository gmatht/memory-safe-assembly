use crate::common::*;
use std::collections::HashMap;
use z3::ast::Ast;
use z3::*;

#[derive(Clone, Debug)]
pub struct Relocation {
    pub addr: i64,
    pub expr: crate::common::AbstractExpression,
    pub size: usize,
}

#[derive(Clone, Debug)]
pub struct LinkerReloc {
    pub addr: i64,
    pub size: usize,
    pub symbol: Option<String>,
    pub addend: Option<i64>,
    pub expr: String,
    pub kind: String,
}

#[derive(Clone)]
pub struct X86Computer<'ctx> {
    pub registers: HashMap<String, RegisterValue>,
    pub simd_registers: HashMap<String, Vec<RegisterValue>>,
    pub memory: HashMap<String, MemorySafeRegion>,
    pub memory_labels: HashMap<String, i64>,
    pub relocations: Vec<Relocation>,
    pub context: &'ctx Context,
    pub solver: Solver<'ctx>,
}

impl<'ctx> X86Computer<'ctx> {
    /// Create a shallow clone of this computer bound to the same Z3 context but with a fresh solver.
    pub fn clone_shallow(&self) -> Self {
        let mut new = X86Computer::new(self.context);
        new.registers = self.registers.clone();
        new.simd_registers = self.simd_registers.clone();
        new.memory = self.memory.clone();
        new.memory_labels = self.memory_labels.clone();
        new.relocations = self.relocations.clone();
        new
    }
    pub fn new(context: &'ctx Context) -> Self {
        let solver = Solver::new(context);
        let mut registers = HashMap::new();
        for r in &[
            "rax", "rbx", "rcx", "rdx", "rsi", "rdi", "r8", "r9", "r10", "r11", "r12", "r13",
            "r14", "r15", "rsp", "rbp",
        ] {
            registers.insert(r.to_string(), RegisterValue::new_empty(r));
        }

        let mut simd_registers = HashMap::new();
        for i in 0..16 {
            simd_registers.insert(format!("ymm{}", i), vec![RegisterValue::new_empty("v"); 32]);
        }

        let mut memory = HashMap::new();
        // add stack region by default
        memory.insert(
            "sp".to_string(),
            MemorySafeRegion::new(
                AbstractExpression::Abstract("STACK_MAX".to_string()),
                RegionType::RW,
            ),
        );

        X86Computer {
            registers,
            simd_registers,
            memory,
            memory_labels: HashMap::new(),
            relocations: Vec::new(),
            context,
            solver,
        }
    }

    pub fn add_relocation(
        &mut self,
        addr: i64,
        expr: crate::common::AbstractExpression,
        size: usize,
    ) {
        self.relocations.push(Relocation { addr, expr, size });
    }

    /// Return and clear current relocations
    pub fn take_relocations(&mut self) -> Vec<Relocation> {
        let out = self.relocations.clone();
        self.relocations.clear();
        out
    }

    /// Export current relocations as a JSON array string. Does not clear relocations.
    pub fn export_relocations_json(&self) -> String {
        // build JSON manually to avoid adding serde dependency
        let mut items: Vec<String> = Vec::new();
        for r in &self.relocations {
            // escape backslashes and quotes in expr's string form
            let expr_s = format!("{}", r.expr)
                .replace('\\', "\\\\")
                .replace('"', "\\\"");
            let it = format!(
                "{{\"addr\":{},\"size\":{},\"expr\":\"{}\"}}",
                r.addr, r.size, expr_s
            );
            items.push(it);
        }
        format!("[{}]", items.join(","))
    }

    /// Write relocations JSON to a file at `path`. Returns io::Result<()>.
    pub fn export_relocations_to_file(&self, path: &str) -> std::io::Result<()> {
        let json = self.export_relocations_json();
        std::fs::write(path, json.as_bytes())
    }

    /// Attempt to convert recorded relocations into linker-style records.
    /// For simple expressions like `LABEL` or `LABEL + 8` this will extract
    /// the symbol name and addend. For expressions that are label differences
    /// (LABEL1 - LABEL2) or complex expressions, symbol will be None and kind
    /// will be set to "DIFF" or "EXPR" respectively.
    pub fn relocations_to_linker_records(&self) -> Vec<LinkerReloc> {
        fn extract_symbol_addend(
            expr: &crate::common::AbstractExpression,
            labels: &std::collections::HashMap<String, i64>,
            comp: &X86Computer,
        ) -> Option<(String, i64)> {
            use crate::common::AbstractExpression::*;
            match expr {
                Abstract(name) => Some((name.clone(), 0)),
                Immediate(_) => None,
                Expression(op, a, b) => {
                    // try to find a primary symbol on either side and accumulate immediates
                    if op == "+" {
                        if let Some((sym, off)) = extract_symbol_addend(a, labels, comp) {
                            if let Some(n) = crate::common::try_resolve_expr(b, labels, comp) {
                                return Some((sym, off + n));
                            }
                        }
                        if let Some((sym, off)) = extract_symbol_addend(b, labels, comp) {
                            if let Some(n) = crate::common::try_resolve_expr(a, labels, comp) {
                                return Some((sym, off + n));
                            }
                        }
                        None
                    } else if op == "-" {
                        // symbol - immediate
                        if let Some((sym, off)) = extract_symbol_addend(a, labels, comp) {
                            if let Some(n) = crate::common::try_resolve_expr(b, labels, comp) {
                                return Some((sym, off - n));
                            }
                        }
                        None
                    } else {
                        None
                    }
                }
                _ => None,
            }
        }

        let mut out: Vec<LinkerReloc> = Vec::new();
        for r in &self.relocations {
            let expr_s = format!("{}", r.expr);
            let mut kind = "EXPR".to_string();
            let mut symbol: Option<String> = None;
            let mut addend: Option<i64> = None;
            // detect label - label difference specially
            if let crate::common::AbstractExpression::Expression(op, a, b) = &r.expr {
                if op == "-" {
                    if matches!(&**a, crate::common::AbstractExpression::Abstract(_))
                        && matches!(&**b, crate::common::AbstractExpression::Abstract(_))
                    {
                        kind = "DIFF".to_string();
                    }
                }
            }

            if let Some((sym, off)) = extract_symbol_addend(&r.expr, &self.memory_labels, self) {
                symbol = Some(sym);
                addend = Some(off);
                kind = "SYM_ADD".to_string();
            }

            out.push(LinkerReloc {
                addr: r.addr,
                size: r.size,
                symbol,
                addend,
                expr: expr_s,
                kind,
            });
        }
        out
    }

    /// Export linker-style relocations as JSON string
    pub fn export_linker_relocs_json(&self) -> String {
        let mut items: Vec<String> = Vec::new();
        for lr in self.relocations_to_linker_records() {
            let expr_s = lr.expr.replace('\\', "\\\\").replace('"', "\\\"");
            let sym_field = match lr.symbol {
                Some(s) => format!("\"{}\"", s.replace('"', "\\\"")),
                None => "null".to_string(),
            };
            let add_field = match lr.addend {
                Some(a) => format!("{}", a),
                None => "null".to_string(),
            };
            let it = format!(
                "{{\"addr\":{},\"size\":{},\"symbol\":{},\"addend\":{},\"expr\":\"{}\",\"kind\":\"{}\"}}",
                lr.addr, lr.size, sym_field, add_field, expr_s, lr.kind
            );
            items.push(it);
        }
        format!("[{}]", items.join(","))
    }

    /// Write linker relocations JSON to file
    pub fn export_linker_relocs_to_file(&self, path: &str) -> std::io::Result<()> {
        let json = self.export_linker_relocs_json();
        std::fs::write(path, json.as_bytes())
    }

    /// Conservative memory-safety check for an access expressed as an AbstractExpression + offset.
    /// This mirrors the logic used by the ARM engine's mem_safe_access. It uses the engine's
    /// solver, memory map and memory_labels to determine whether the access can be proven
    /// to be inside a declared region.
    pub fn mem_safe_access(
        &self,
        base_expr: AbstractExpression,
        offset: i64,
        ty: RegionType,
    ) -> Result<(), MemorySafetyError> {
        // Try to evaluate purely concrete arithmetic expressions into a single Immediate
        fn eval_immediate(expr: &AbstractExpression) -> Option<i64> {
            match expr {
                AbstractExpression::Immediate(n) => Some(*n),
                AbstractExpression::Expression(op, a, b) => {
                    let la = eval_immediate(a);
                    let lb = eval_immediate(b);
                    match (op.as_str(), la, lb) {
                        ("+", Some(x), Some(y)) => Some(x + y),
                        ("*", Some(x), Some(y)) => Some(x * y),
                        ("-", Some(x), Some(y)) => Some(x - y),
                        _ => None,
                    }
                }
                _ => None,
            }
        }

        // If base_expr is a reducible pure-immediate expression, reduce it to Immediate here
        let base_expr = match base_expr {
            AbstractExpression::Immediate(_) => base_expr,
            other => {
                if let Some(n) = eval_immediate(&other) {
                    AbstractExpression::Immediate(n)
                } else {
                    other
                }
            }
        };
        let mut symbolic_base = false;
        let (region, base, base_access) = match base_expr.clone() {
            AbstractExpression::Abstract(regbase) => {
                if let Some(region) = self.memory.get(&regbase.clone()) {
                    (
                        region,
                        ast::Int::new_const(self.context, regbase.clone()),
                        ast::Int::new_const(self.context, regbase),
                    )
                } else {
                    if let Some(address) = self.memory_labels.get(&regbase.clone()) {
                        // Use the concrete address as the AST base; do NOT add it
                        // into `offset` (that would double-count the address later).
                        (
                            self.memory
                                .get(&"memory".to_string())
                                .expect("memory should exist"),
                            ast::Int::from_i64(self.context, *address),
                            ast::Int::from_i64(self.context, *address),
                        )
                    } else {
                        return Err(MemorySafetyError::new(
                            format!("unknown memory base {:?}", regbase).as_str(),
                        ));
                    }
                }
            }
            AbstractExpression::Immediate(n) => {
                // Concrete absolute address: treat as part of the generic "memory" region
                if let Some(mem) = self.memory.get("memory") {
                    (
                        mem,
                        ast::Int::from_i64(self.context, n),
                        ast::Int::from_i64(self.context, n),
                    )
                } else {
                    return Err(MemorySafetyError::new(
                        format!(
                            "No matching region found for access Immediate({:?}), {:?}",
                            n, offset
                        )
                        .as_str(),
                    ));
                }
            }
            _ => {
                symbolic_base = true;
                let abstracts = base_expr.get_abstracts();
                let mut result: Option<(&MemorySafeRegion, z3::ast::Int<'_>, z3::ast::Int<'_>)> =
                    None;
                for r in self.memory.keys() {
                    if abstracts.contains(r) {
                        result = Some((
                            self.memory.get(r).expect("Region not in memory 2"),
                            expression_to_ast(
                                self.context,
                                AbstractExpression::Abstract(r.to_string()),
                            )
                            .expect("computer25"),
                            expression_to_ast(self.context, base_expr.clone())
                                .expect("computer251"),
                        ));
                        break;
                    }
                    for r in self.memory_labels.clone() {
                        if abstracts.contains(&r.0) {
                            if let Some(_address) = self.memory_labels.get(&r.0.clone()) {
                                result = Some((
                                    self.memory.get("memory").expect("Region not in memory 2"),
                                    expression_to_ast(
                                        self.context,
                                        AbstractExpression::Abstract(r.0.to_string()),
                                    )
                                    .expect("computer25"),
                                    expression_to_ast(self.context, base_expr.clone())
                                        .expect("computer251"),
                                ));
                                break;
                            }
                        }
                    }
                }
                if let Some(res) = result {
                    res
                } else {
                    return Err(MemorySafetyError::new(
                        format!(
                            "No matching region found for access {:?}, {:?}",
                            base_expr, offset
                        )
                        .as_str(),
                    ));
                }
            }
        };

        if ty == RegionType::WRITE && region.kind == RegionType::READ {
            return Err(MemorySafetyError::new(&format!(
                "Access does not match region type {:#?} {:?} {:?}",
                region.kind, ty, base_expr
            )));
        }

        let mut abs_offset = ast::Int::from_i64(self.context, offset);
        if base_expr.contains("sp") {
            abs_offset = ast::Int::from_i64(self.context, offset.abs());
        }
        let access = ast::Int::add(self.context, &[&base_access, &abs_offset]);

        let lowerbound_value = ast::Int::from_i64(self.context, 0);
        let low_access = ast::Int::add(self.context, &[&base, &lowerbound_value]);
        let upperbound_value =
            expression_to_ast(self.context, region.get_length()).expect("computer26");
        let up_access = ast::Int::add(self.context, &[&base, &upperbound_value]);
        let l = access.lt(&low_access);
        let u = {
            if offset == 0 && !symbolic_base {
                access.ge(&up_access)
            } else {
                access.gt(&up_access)
            }
        };

        // Build any extra assumptions tying abstract labels to concrete addresses
        // when we can resolve them from memory_labels. These are passed as
        // assumptions to check_assumptions so we don't permanently assert them.
        let mut extra_assumptions: Vec<ast::Bool> = Vec::new();
        if symbolic_base {
            for name in base_expr.get_abstracts() {
                if let Some(addr) = self.memory_labels.get(&name) {
                    let var = ast::Int::new_const(self.context, name.clone());
                    let val = ast::Int::from_i64(self.context, *addr);
                    // use the Ast::_eq helper to produce a Bool<'ctx> equality
                    extra_assumptions.push(var._eq(&val));
                }
            }
        }

        // Combine extra assumptions with the impossibility checks
        let mut low_args = extra_assumptions.clone();
        low_args.push(l.clone());
        let mut up_args = extra_assumptions.clone();
        up_args.push(u.clone());

        match (
            self.solver.check_assumptions(&low_args),
            self.solver.check_assumptions(&up_args),
        ) {
            (SatResult::Unsat, SatResult::Unsat) => {
                log::info!("Memory safe with solver's check!");
                log::info!("Unsat core {:?}", self.solver.get_unsat_core());
                return Ok(());
            }
            (a, b) => {
                log::info!("Load from address {:?} + {} unsafe", base_expr, offset);
                log::info!(
                    "impossibility lower bound {:?}, impossibility upper bound {:?}, model: {:?}",
                    a,
                    b,
                    self.solver.get_model()
                );
                log::info!("Memory unsafe with solver's check!");
            }
        }
        return Err(MemorySafetyError::new(
            format!(
                "Accessing address outside allowable memory regions {:?}, {:?}",
                base_expr, offset
            )
            .as_str(),
        ));
    }

    pub fn set_register_imm(&mut self, name: &str, val: i64) {
        let key = Self::normalize_register_name(name);
        if let Some(r) = self.registers.get_mut(&key) {
            r.set(RegisterKind::Immediate, None, val);
        }
    }

    /// Write an immediate value to `name`, taking into account x86 partial-register semantics.
    /// - 32-bit alias (ends with 'd' or starts with 'e'): zero-extend into canonical 64-bit register
    /// - 16/8-bit aliases: conservative => mark canonical register unknown
    /// - full 64-bit registers: store immediate directly
    pub fn write_register_from_imm(&mut self, name: &str, val: i64) {
        // detect alias form on original name
        if name.ends_with('d') || name.starts_with('e') {
            // zero-extend into low 32 bits and set known bits for low 32
            let imm64 = (val as u32) as i64;
            self.set_register_imm(name, imm64);
            return;
        }
        if name.ends_with('b') || name.ends_with('w') {
            // 8-bit and 16-bit aliases: update known bits when possible
            let key = Self::normalize_register_name(name);
            if let Some(r) = self.registers.get_mut(&key) {
                let mask: u64 = if name.ends_with('b') { 0xff } else { 0xffff };
                let imm_u = val as u64 & mask;
                // merge known bits: if canonical has some known bits, update them; otherwise set known bits for the low part
                r.set_known_bits(mask, imm_u);
                return;
            } else {
                self.set_register_unknown(name);
                return;
            }
        }
        // default: full 64-bit register
        self.set_register_imm(name, val);
    }

    /// Write a RegisterValue into `name` respecting partial-register semantics where possible.
    /// Conservative: if semantics unclear, mark the canonical register unknown.
    pub fn write_register_from_value(&mut self, name: &str, value: RegisterValue) {
        // 32-bit destination aliases zero-extend
        if name.ends_with('d') || name.starts_with('e') {
            match value.kind {
                RegisterKind::Immediate => {
                    let imm32 = (value.offset as u32) as i64;
                    if imm32 != value.offset {
                        self.set_register_unknown(name);
                        return;
                    }
                    let imm64 = imm32 as i64;
                    self.set_register_imm(name, imm64);
                    return;
                }
                _ => {
                    // cannot safely represent zero-extend of non-immediate abstract value
                    self.set_register_unknown(name);
                    return;
                }
            }
        }

        // 8-bit / 16-bit aliases: try to preserve upper bits if canonical register is immediate
        if name.ends_with('b') || name.ends_with('w') {
            let key = Self::normalize_register_name(name);
            if let Some(r) = self.registers.get_mut(&key) {
                let mask: u64 = if name.ends_with('b') { 0xff } else { 0xffff };
                match value.kind {
                    RegisterKind::Immediate => {
                        r.set_known_bits(mask, value.offset as u64 & mask);
                        return;
                    }
                    RegisterKind::RegisterBase | RegisterKind::Number => {
                        // incoming value is abstract; we can't trust low bits, but we mark them unknown
                        // by clearing those known bits
                        r.known_mask &= !mask;
                        // if no known bits remain, mark as Number
                        if r.known_mask == 0 {
                            r.set(RegisterKind::Number, None, 0);
                        }
                        return;
                    }
                }
            } else {
                self.set_register_unknown(name);
                return;
            }
        }

        // full-width destination: write the value as-is
        self.set_register_value(name, value);
    }

    /// Set the register to a full RegisterValue (overwrite)
    pub fn set_register_value(&mut self, name: &str, value: RegisterValue) {
        let key = Self::normalize_register_name(name);
        if let Some(r) = self.registers.get_mut(&key) {
            *r = value;
        }
    }

    /// Set a register to an abstract expression (RegisterKind::RegisterBase)
    pub fn set_register_abstract(
        &mut self,
        name: &str,
        base: Option<AbstractExpression>,
        offset: i64,
    ) {
        let key = Self::normalize_register_name(name);
        if let Some(r) = self.registers.get_mut(&key) {
            r.set(RegisterKind::RegisterBase, base, offset);
        }
    }

    /// Get the current RegisterValue for a register name. If unknown, returns an empty RegisterValue.
    pub fn get_register_value(&self, name: &str) -> RegisterValue {
        let n = name.trim();
        let key = Self::normalize_register_name(n);
        if let Some(r) = self.registers.get(&key) {
            let mut out = r.clone();
            // handle common alias reads to return the appropriate subset when safe
            match n {
                // low 8-bit aliases
                "al" | "bl" | "cl" | "dl" | "sil" | "dil" | "spl" | "bpl" => {
                    if out.known_mask == !0u64 || (out.known_mask & 0xff) == 0xff {
                        // low byte known
                        out.offset = (out.known_value & 0xff) as i64;
                        out.kind = RegisterKind::Immediate;
                    } else {
                        out.set(RegisterKind::Number, None, 0);
                    }
                    return out;
                }
                // low 16-bit aliases
                "ax" | "bx" | "cx" | "dx" => {
                    if out.known_mask == !0u64 || (out.known_mask & 0xffff) == 0xffff {
                        out.offset = (out.known_value & 0xffff) as i64;
                        out.kind = RegisterKind::Immediate;
                    } else {
                        out.set(RegisterKind::Number, None, 0);
                    }
                    return out;
                }
                _ => {}
            }

            // suffix-based aliases (e.g., r11b, r11w, r11d, r11)
            if n.ends_with('b') {
                if out.known_mask == !0u64 || (out.known_mask & 0xff) == 0xff {
                    out.offset = (out.known_value & 0xff) as i64;
                    out.kind = RegisterKind::Immediate;
                } else {
                    out.set(RegisterKind::Number, None, 0);
                }
                return out;
            }
            if n.ends_with('w') {
                if out.known_mask == !0u64 || (out.known_mask & 0xffff) == 0xffff {
                    out.offset = (out.known_value & 0xffff) as i64;
                    out.kind = RegisterKind::Immediate;
                } else {
                    out.set(RegisterKind::Number, None, 0);
                }
                return out;
            }
            if n.ends_with('d') || n.starts_with('e') {
                if out.known_mask == !0u64 || (out.known_mask & 0xffffffff) == 0xffffffff {
                    out.offset = (out.known_value & 0xffffffff) as i64;
                    out.kind = RegisterKind::Immediate;
                } else {
                    out.set(RegisterKind::Number, None, 0);
                }
                return out;
            }

            return out;
        }
        // return a default empty register value mapping to the canonical name
        RegisterValue::new(
            RegisterKind::RegisterBase,
            Some(AbstractExpression::Abstract(key.clone())),
            0,
        )
    }

    fn normalize_register_name(name: &str) -> String {
        let n = name.trim();
        // handle common sized aliases:
        // r11d/r11w/r11b -> r11
        if n.starts_with('r') && n.len() > 2 {
            if let Some(last) = n.chars().last() {
                if last == 'd' || last == 'w' || last == 'b' {
                    return n[..n.len() - 1].to_string();
                }
            }
        }
        // eax -> rax, ebx -> rbx (e-regs)
        if n.len() == 3 && n.starts_with('e') {
            let mut s = String::from("r");
            s.push_str(&n[1..]);
            return s;
        }
        // two-letter low regs like al/ax -> rax and similar
        if n.len() == 2 {
            match n {
                "al" | "ax" => return String::from("rax"),
                "bl" | "bx" => return String::from("rbx"),
                "cl" | "cx" => return String::from("rcx"),
                "dl" | "dx" => return String::from("rdx"),
                _ => {}
            }
        }
        // 3-letter special low regs (sil/dil/spl/bpl) -> rsi/rdi/rsp/rbp
        if n == "sil" {
            return String::from("rsi");
        }
        if n == "dil" {
            return String::from("rdi");
        }
        if n == "spl" {
            return String::from("rsp");
        }
        if n == "bpl" {
            return String::from("rbp");
        }
        // default: return as-is
        n.to_string()
    }

    /// Mark register as unknown numeric value
    pub fn set_register_unknown(&mut self, name: &str) {
        let key = Self::normalize_register_name(name);
        if let Some(r) = self.registers.get_mut(&key) {
            r.set(RegisterKind::Number, None, 0);
        }
    }

    pub fn add_memory_region(&mut self, name: String, ty: RegionType, length: AbstractExpression) {
        self.memory.insert(name, MemorySafeRegion::new(length, ty));
    }

    pub fn add_memory_value(&mut self, region: String, address: i64, value: i64) {
        let reg_value = RegisterValue::new(RegisterKind::Immediate, None, value);
        match self.memory.get_mut(&region) {
            Some(r) => {
                r.insert(address, reg_value);
            }
            None => {
                let mut region_map =
                    MemorySafeRegion::new(AbstractExpression::Immediate(0), RegionType::RW);
                region_map.insert(address, reg_value);
                self.memory.insert(region, region_map);
            }
        }
    }

    pub fn add_memory_value_abstract(
        &mut self,
        region: String,
        address: i64,
        value: AbstractExpression,
    ) {
        let reg_value = RegisterValue::new(RegisterKind::RegisterBase, Some(value), 0);
        match self.memory.get_mut(&region) {
            Some(r) => {
                r.insert(address, reg_value);
            }
            None => {
                let mut region_map =
                    MemorySafeRegion::new(AbstractExpression::Immediate(0), RegionType::RW);
                region_map.insert(address, reg_value);
                self.memory.insert(region, region_map);
            }
        }
    }

    pub fn add_memory_label(&mut self, name: String, offset: i64) {
        self.memory_labels.insert(name, offset);
    }
}

// Implement RegisterProvider for X86Computer so common utilities can query
// register immediates without depending on the concrete type in common.rs.
impl<'ctx> crate::common::RegisterProvider for X86Computer<'ctx> {
    fn get_register_value(&self, name: &str) -> crate::common::RegisterValue {
        self.get_register_value(name)
    }
}
