use crate::common::*;
use crate::x86_64::computer::*;
use crate::x86_64::instruction_parser::*;
use std::collections::VecDeque;
use std::fs::File;
use std::io::{BufRead, BufReader};
use z3::ast::Ast;
use z3::*; // bring simplify() and other Ast methods into scope

// Helper: convert an X86Operand into an AbstractExpression using the computer state.
fn operand_to_expr<'a>(op: &X86Operand, comp: &X86Computer<'a>) -> AbstractExpression {
    match op {
        X86Operand::Reg(r) => {
            let rv = comp.get_register_value(r);
            match rv.kind {
                crate::common::RegisterKind::Immediate => AbstractExpression::Immediate(rv.offset),
                crate::common::RegisterKind::RegisterBase => {
                    if let Some(base) = rv.base.clone() {
                        if rv.offset != 0 {
                            generate_expression("+", base, AbstractExpression::Immediate(rv.offset))
                        } else {
                            base
                        }
                    } else {
                        AbstractExpression::Immediate(rv.offset)
                    }
                }
                crate::common::RegisterKind::Number => AbstractExpression::Register(Box::new(rv)),
            }
        }

        X86Operand::Imm(v) => AbstractExpression::Immediate(*v),
        X86Operand::Mem {
            base,
            index,
            scale,
            disp,
        } => {
            // resolve base and index possibly through registers
            let mut expr = AbstractExpression::Immediate(0);
            if let Some(bn) = base {
                // if base refers to a register name, reduce it
                let base_rv = comp.get_register_value(bn);
                expr = match base_rv.kind {
                    crate::common::RegisterKind::Immediate => {
                        AbstractExpression::Immediate(base_rv.offset)
                    }
                    crate::common::RegisterKind::RegisterBase => {
                        if let Some(b) = base_rv.base.clone() {
                            if base_rv.offset != 0 {
                                generate_expression(
                                    "+",
                                    b,
                                    AbstractExpression::Immediate(base_rv.offset),
                                )
                            } else {
                                b
                            }
                        } else {
                            AbstractExpression::Immediate(base_rv.offset)
                        }
                    }
                    crate::common::RegisterKind::Number => {
                        AbstractExpression::Register(Box::new(base_rv))
                    }
                };
            }

            if let Some(idx) = index {
                let idx_rv = comp.get_register_value(idx);
                let idx_expr = match idx_rv.kind {
                    crate::common::RegisterKind::Immediate => {
                        AbstractExpression::Immediate(idx_rv.offset)
                    }
                    crate::common::RegisterKind::RegisterBase => {
                        if let Some(b) = idx_rv.base.clone() {
                            if idx_rv.offset != 0 {
                                generate_expression(
                                    "+",
                                    b,
                                    AbstractExpression::Immediate(idx_rv.offset),
                                )
                            } else {
                                b
                            }
                        } else {
                            AbstractExpression::Immediate(idx_rv.offset)
                        }
                    }
                    crate::common::RegisterKind::Number => {
                        AbstractExpression::Register(Box::new(idx_rv))
                    }
                };
                let scaled = if *scale != 1 {
                    generate_expression("*", idx_expr, AbstractExpression::Immediate(*scale))
                } else {
                    idx_expr
                };
                expr = generate_expression("+", expr, scaled);
            }

            if *disp != 0 {
                expr = generate_expression("+", expr, AbstractExpression::Immediate(*disp));
            }
            expr
        }
        X86Operand::Label(l) => AbstractExpression::Abstract(l.clone()),
        X86Operand::Other(_) => AbstractExpression::Immediate(0),
    }
}

pub struct ExecutionEngineX86<'ctx> {
    pub computer: X86Computer<'ctx>,
    pub program: Vec<X86Instruction>,
    pub label_map: std::collections::HashMap<String, usize>,
    // history used to detect loop patterns and enable K/K+1 heuristics
    pub jump_history: Vec<(
        usize,
        bool,
        AbstractComparison,
        Vec<crate::common::MemoryAccess>,
        X86Computer<'ctx>,
    )>,
    pub in_loop: bool,
}

impl<'ctx> ExecutionEngineX86<'ctx> {
    pub fn from_asm_file(path: &str, ctx: &'ctx Context) -> Self {
        let f = File::open(path).expect("open asm");
        let reader = BufReader::new(f);

        let mut lines: Vec<String> = Vec::new();
        for line in reader.lines() {
            lines.push(line.unwrap_or(String::from("")));
        }

        // separate directives/definitions (starting with '.') and labels from instructions
        let mut data_defs: Vec<String> = Vec::new();
        let mut program: Vec<X86Instruction> = Vec::new();
        let mut label_map: std::collections::HashMap<String, usize> =
            std::collections::HashMap::new();
        let mut idx = 0usize;
        // track current section (e.g., .rodata/.data/.text) so labels within
        // data sections are treated as data labels
        let mut current_section: Option<String> = None;
        while idx < lines.len() {
            let s = lines[idx].trim();
            idx += 1;
            if s.is_empty() {
                continue;
            }

            // handle explicit section directives which influence how we
            // interpret following labels
            if s.starts_with(".section") || s.starts_with(".rodata") || s.starts_with(".data") {
                // attempt to extract a canonical section name
                if s.starts_with(".section") {
                    let mut parts = s.split_whitespace().filter(|p| !p.is_empty());
                    // skip the .section token
                    parts.next();
                    if let Some(sec) = parts.next() {
                        // strip trailing commas and surrounding quotes
                        let sec = sec.trim().trim_matches(',').trim_matches('"').to_string();
                        current_section = Some(sec);
                    }
                } else {
                    // .rodata or .data simple form
                    let sec = s.split_whitespace().next().unwrap_or(s).to_string();
                    current_section = Some(sec);
                }
                data_defs.push(s.to_string());
                continue;
            }

            // If the line is a label, decide whether it's a data label or a
            // code label. We treat labels in known data sections or labels
            // immediately followed by data directives as data labels.
            if s.ends_with(':') {
                let label = s.strip_suffix(':').unwrap_or(s).to_string();
                let mut j = idx;
                let mut seen_directive = false;

                // If we're inside a known data section, treat label as data
                if let Some(sec) = &current_section {
                    if sec.contains("rodata") || sec.contains("data") {
                        seen_directive = true;
                        if data_defs
                            .last()
                            .map(|x| x != &s.to_string())
                            .unwrap_or(true)
                        {
                            data_defs.push(s.to_string());
                        }
                        while j < lines.len() {
                            let u = lines[j].trim();
                            if u.is_empty() {
                                j += 1;
                                continue;
                            }
                            if u.starts_with('.') {
                                data_defs.push(u.to_string());
                                j += 1;
                                continue;
                            }
                            break;
                        }
                    }
                }

                // If not already decided by section, peek ahead to see if the
                // label is directly followed by data directives.
                if !seen_directive {
                    let mut k = idx;
                    while k < lines.len() {
                        let t = lines[k].trim();
                        k += 1;
                        if t.is_empty() {
                            continue;
                        }
                        if t.starts_with('.') {
                            let mut parts = t
                                .split(|c: char| c.is_whitespace() || c == ',')
                                .filter(|p| !p.is_empty());
                            if let Some(kw) = parts.next() {
                                let kw = kw.trim();
                                let data_kw = [
                                    ".byte",
                                    ".long",
                                    ".quad",
                                    ".asciz",
                                    ".string",
                                    ".ascii",
                                    ".word",
                                    ".hword",
                                    ".align",
                                    ".balign",
                                    ".globl",
                                    ".global",
                                    ".private_extern",
                                ];
                                if data_kw.iter().any(|d| d == &kw) {
                                    seen_directive = true;
                                    if data_defs
                                        .last()
                                        .map(|x| x != &s.to_string())
                                        .unwrap_or(true)
                                    {
                                        data_defs.push(s.to_string());
                                    }
                                    // consume contiguous directive lines
                                    j = k - 1;
                                    while j < lines.len() {
                                        let u = lines[j].trim();
                                        if u.is_empty() {
                                            j += 1;
                                            continue;
                                        }
                                        if u.starts_with('.') {
                                            data_defs.push(u.to_string());
                                            j += 1;
                                            continue;
                                        }
                                        break;
                                    }
                                }
                            }
                        }
                        break;
                    }
                }

                if seen_directive {
                    idx = j;
                    continue;
                }

                // otherwise treat as a code label
                label_map.insert(label, program.len());
                continue;
            }

            if s.starts_with('.') {
                data_defs.push(s.to_string());
                continue;
            }

            if let Some(i) = parse_x86_line(s) {
                program.push(i);
            }
        }

        let mut computer = X86Computer::new(ctx);

        // load static data from defs into computer.memory and memory_labels
        // We parse the collected data_defs strings into structured directives
        // for more robust handling.
        let mut address: i64 = 4;

        #[derive(Debug)]
        enum DataToken {
            Num(i64),
            // general expression form for label arithmetic etc.
            Expr(crate::common::AbstractExpression),
        }

        #[derive(Debug)]
        enum DataDirective {
            Align(i64),
            Byte(Vec<DataToken>),
            Long(Vec<DataToken>),
            Quad(Vec<DataToken>),
            Halfword(Vec<DataToken>),
            StringLit(String, bool), // (contents, is_asciz)
            Globl(String),
            Section(String),
            Other(String),
        }
        #[derive(Debug)]
        enum DataItem {
            Label(String),
            Directive(DataDirective),
        }

        fn parse_data_def(def: &str) -> Option<DataItem> {
            let s = def.trim();
            if s.is_empty() {
                return None;
            }
            if s.ends_with(':') {
                return Some(DataItem::Label(s.strip_suffix(':').unwrap().to_string()));
            }
            if !s.starts_with('.') {
                return Some(DataItem::Directive(DataDirective::Other(s.to_string())));
            }

            // tokenize by whitespace but keep quoted strings intact simplistically
            let parts: Vec<&str> = s.split_whitespace().collect();
            if parts.is_empty() {
                return None;
            }
            let kw = parts[0];
            match kw {
                ".align" | ".balign" => {
                    if parts.len() > 1 {
                        let arg = parts[1].trim().trim_matches(',');
                        // Some assemblers specify alignment as a power (e.g., 2 means 2^2=4)
                        if let Ok(n) = arg.parse::<i64>() {
                            // if the argument looks like a small integer (<= 8)
                            // treat it as power-of-two exponent; otherwise use as bytes
                            if n > 0 && n <= 8 {
                                let bytes = 1i64 << n;
                                return Some(DataItem::Directive(DataDirective::Align(bytes)));
                            }
                            return Some(DataItem::Directive(DataDirective::Align(n)));
                        }
                    }
                    return Some(DataItem::Directive(DataDirective::Align(0)));
                }
                ".byte" => {
                    let mut v: Vec<DataToken> = Vec::new();
                    let tail = s[5..].trim();
                    for tok in tail.split(',') {
                        let t = tok.trim();
                        if t.is_empty() {
                            continue;
                        }
                        // try label - number or label - label form
                        if let Some(dt) = parse_label_arith_token(t) {
                            v.push(dt);
                            continue;
                        }
                        if let Some(num) = parse_numeric_token(t) {
                            v.push(DataToken::Num(num));
                        } else {
                            v.push(DataToken::Expr(
                                crate::common::AbstractExpression::Abstract(t.to_string()),
                            ));
                        }
                    }
                    return Some(DataItem::Directive(DataDirective::Byte(v)));
                }
                ".long" => {
                    let mut v: Vec<DataToken> = Vec::new();
                    let tail = s[5..].trim();
                    for tok in tail.split(',') {
                        let t = tok.trim();
                        if t.is_empty() {
                            continue;
                        }
                        if let Some(dt) = parse_label_arith_token(t) {
                            v.push(dt);
                            continue;
                        }
                        if let Some(num) = parse_numeric_token(t) {
                            v.push(DataToken::Num(num));
                        } else {
                            v.push(DataToken::Expr(
                                crate::common::AbstractExpression::Abstract(t.to_string()),
                            ));
                        }
                    }
                    return Some(DataItem::Directive(DataDirective::Long(v)));
                }
                ".quad" => {
                    let mut v: Vec<DataToken> = Vec::new();
                    let tail = s[5..].trim();
                    for tok in tail.split(',') {
                        let t = tok.trim();
                        if t.is_empty() {
                            continue;
                        }
                        if let Some(dt) = parse_label_arith_token(t) {
                            v.push(dt);
                            continue;
                        }
                        if let Some(num) = parse_numeric_token(t) {
                            v.push(DataToken::Num(num));
                        } else {
                            v.push(DataToken::Expr(
                                crate::common::AbstractExpression::Abstract(t.to_string()),
                            ));
                        }
                    }
                    return Some(DataItem::Directive(DataDirective::Quad(v)));
                }
                ".asciz" | ".string" | ".ascii" => {
                    // Parse quoted string content, supporting common escapes like
                    // \" \\ \n \t \r and hex escapes \xNN. Allow commas/spaces
                    // inside the string.
                    let mut content = String::new();
                    let is_asciz = kw == ".asciz";
                    if let Some(start) = s.find('"') {
                        let mut i = start + 1;
                        let bytes = s.as_bytes();
                        while i < s.len() {
                            let c = bytes[i] as char;
                            if c == '"' {
                                // end of string
                                break;
                            }
                            if c == '\\' && i + 1 < s.len() {
                                let nc = bytes[i + 1] as char;
                                match nc {
                                    '"' => {
                                        content.push('"');
                                        i += 2;
                                        continue;
                                    }
                                    '\\' => {
                                        content.push('\\');
                                        i += 2;
                                        continue;
                                    }
                                    'n' => {
                                        content.push('\n');
                                        i += 2;
                                        continue;
                                    }
                                    't' => {
                                        content.push('\t');
                                        i += 2;
                                        continue;
                                    }
                                    'r' => {
                                        content.push('\r');
                                        i += 2;
                                        continue;
                                    }
                                    '0' => {
                                        content.push('\0');
                                        i += 2;
                                        continue;
                                    }
                                    'x' => {
                                        // parse two hex digits if present
                                        if i + 3 < s.len() {
                                            let hx = &s[i + 2..i + 4];
                                            if let Ok(v) = i64::from_str_radix(hx, 16) {
                                                content.push(v as u8 as char);
                                                i += 4;
                                                continue;
                                            }
                                        }
                                        // fallback: treat literally
                                        content.push('x');
                                        i += 2;
                                        continue;
                                    }
                                    _ => {
                                        // unknown escape, push the next char literally
                                        content.push(nc);
                                        i += 2;
                                        continue;
                                    }
                                }
                            }
                            // regular char
                            content.push(c);
                            i += 1;
                        }
                    }
                    return Some(DataItem::Directive(DataDirective::StringLit(
                        content, is_asciz,
                    )));
                }
                ".globl" | ".global" | ".private_extern" => {
                    if parts.len() > 1 {
                        return Some(DataItem::Directive(DataDirective::Globl(
                            parts[1].trim().to_string(),
                        )));
                    }
                    return None;
                }
                ".word" => {
                    // treat .word as 4-byte values on x86
                    let mut v: Vec<DataToken> = Vec::new();
                    let tail = s[5..].trim();
                    for tok in tail.split(',') {
                        let t = tok.trim();
                        if t.is_empty() {
                            continue;
                        }
                        if let Some(dt) = parse_label_arith_token(t) {
                            v.push(dt);
                            continue;
                        }
                        if let Some(num) = parse_numeric_token(t) {
                            v.push(DataToken::Num(num));
                        } else {
                            v.push(DataToken::Expr(
                                crate::common::AbstractExpression::Abstract(t.to_string()),
                            ));
                        }
                    }
                    return Some(DataItem::Directive(DataDirective::Long(v)));
                }
                ".hword" => {
                    // halfword: 2-byte values; store as lower 16 bits of i64 in memory
                    let mut v: Vec<DataToken> = Vec::new();
                    let tail = s[6..].trim();
                    for tok in tail.split(',') {
                        let t = tok.trim();
                        if t.is_empty() {
                            continue;
                        }
                        if let Some(dt) = parse_label_arith_token(t) {
                            v.push(dt);
                            continue;
                        }
                        if let Some(num) = parse_numeric_token(t) {
                            v.push(DataToken::Num(num));
                        } else {
                            v.push(DataToken::Expr(
                                crate::common::AbstractExpression::Abstract(t.to_string()),
                            ));
                        }
                    }
                    return Some(DataItem::Directive(DataDirective::Halfword(v)));
                }
                ".section" => {
                    if parts.len() > 1 {
                        return Some(DataItem::Directive(DataDirective::Section(
                            parts[1].trim().to_string(),
                        )));
                    }
                    return Some(DataItem::Directive(DataDirective::Other(s.to_string())));
                }
                _ => return Some(DataItem::Directive(DataDirective::Other(s.to_string()))),
            }
        }

        // convert data_defs (strings) into structured DataItem entries and process
        // helper: parse numeric tokens that may include simple expressions like "16 * 12" and suffixes like "320b"
        fn parse_numeric_token(tok: &str) -> Option<i64> {
            let mut s = tok.trim();
            if s.is_empty() {
                return None;
            }

            // support trailing byte suffix like 320b
            if s.ends_with('b') || s.ends_with('B') {
                s = s[..s.len() - 1].trim();
                if s.is_empty() {
                    return None;
                }
            }

            // recursive descent parser for simple integer arithmetic supporting + - * and parentheses
            let bytes = s.as_bytes();
            let len = bytes.len();
            let mut pos: usize = 0;

            fn skip_ws(bytes: &[u8], len: usize, pos: &mut usize) {
                while *pos < len && (bytes[*pos] as char).is_whitespace() {
                    *pos += 1;
                }
            }

            fn parse_number(bytes: &[u8], len: usize, pos: &mut usize) -> Option<i64> {
                skip_ws(bytes, len, pos);
                if *pos >= len {
                    return None;
                }
                // hex: 0x...
                if *pos + 1 < len
                    && bytes[*pos] as char == '0'
                    && (bytes[*pos + 1] as char == 'x' || bytes[*pos + 1] as char == 'X')
                {
                    *pos += 2;
                    let start = *pos;
                    while *pos < len && ((bytes[*pos] as char).is_digit(16)) {
                        *pos += 1;
                    }
                    if start == *pos {
                        return None;
                    }
                    let s = std::str::from_utf8(&bytes[start..*pos]).ok()?;
                    return i64::from_str_radix(s, 16).ok();
                }

                // decimal
                let start = *pos;
                if *pos < len && ((bytes[*pos] as char) == '+' || (bytes[*pos] as char) == '-') {
                    *pos += 1; // allow unary sign in number parsing here
                }
                while *pos < len && (bytes[*pos] as char).is_ascii_digit() {
                    *pos += 1;
                }
                if start == *pos {
                    return None;
                }
                let s = std::str::from_utf8(&bytes[start..*pos]).ok()?;
                s.parse::<i64>().ok()
            }

            fn parse_factor(bytes: &[u8], len: usize, pos: &mut usize) -> Option<i64> {
                skip_ws(bytes, len, pos);
                if *pos >= len {
                    return None;
                }
                let ch = bytes[*pos] as char;
                if ch == '(' {
                    *pos += 1;
                    let v = parse_expr(bytes, len, pos)?;
                    skip_ws(bytes, len, pos);
                    if *pos < len && (bytes[*pos] as char) == ')' {
                        *pos += 1;
                        return Some(v);
                    } else {
                        return None;
                    }
                }
                if ch == '+' || ch == '-' {
                    // unary
                    *pos += 1;
                    let v = parse_factor(bytes, len, pos)?;
                    if ch == '-' {
                        return Some(-v);
                    } else {
                        return Some(v);
                    }
                }
                parse_number(bytes, len, pos)
            }

            fn parse_term(bytes: &[u8], len: usize, pos: &mut usize) -> Option<i64> {
                let mut v = parse_factor(bytes, len, pos)?;
                loop {
                    skip_ws(bytes, len, pos);
                    if *pos < len {
                        let ch = bytes[*pos] as char;
                        if ch == '*' {
                            *pos += 1;
                            let rhs = parse_factor(bytes, len, pos)?;
                            v = v.checked_mul(rhs)?;
                            continue;
                        } else if ch == '/' {
                            *pos += 1;
                            let rhs = parse_factor(bytes, len, pos)?;
                            // avoid division by zero
                            if rhs == 0 {
                                return None;
                            }
                            v = v.checked_div(rhs)?;
                            continue;
                        } else if ch == '%' {
                            *pos += 1;
                            let rhs = parse_factor(bytes, len, pos)?;
                            if rhs == 0 {
                                return None;
                            }
                            v = v.checked_rem(rhs)?;
                            continue;
                        }
                    }
                    break;
                }
                Some(v)
            }

            fn parse_expr(bytes: &[u8], len: usize, pos: &mut usize) -> Option<i64> {
                let mut v = parse_term(bytes, len, pos)?;
                loop {
                    skip_ws(bytes, len, pos);
                    if *pos < len {
                        let ch = bytes[*pos] as char;
                        if ch == '+' || ch == '-' {
                            *pos += 1;
                            let rhs = parse_term(bytes, len, pos)?;
                            if ch == '+' {
                                v = v.checked_add(rhs)?;
                            } else {
                                v = v.checked_sub(rhs)?;
                            }
                        } else {
                            break;
                        }
                    } else {
                        break;
                    }
                }
                Some(v)
            }

            let res = parse_expr(bytes, len, &mut pos);
            if res.is_some() {
                skip_ws(bytes, len, &mut pos);
                if pos == len {
                    return res;
                }
            }
            None
        }

        // helper: parse tokens like "LABEL - 320b", "LABEL-320b", "LABEL + 4", or "LABEL1 - LABEL2"
        // returns a DataToken::Expr containing an AbstractExpression representing the arithmetic
        fn parse_label_arith_token(tok: &str) -> Option<DataToken> {
            let s = tok.trim();
            // find last occurrence of '+' or '-' (prefer the rightmost operator)
            let pos_plus = s.rfind('+');
            let pos_minus = s.rfind('-');
            let opt = match (pos_plus, pos_minus) {
                (Some(p), Some(m)) => Some(if p > m { (p, '+') } else { (m, '-') }),
                (Some(p), None) => Some((p, '+')),
                (None, Some(m)) => Some((m, '-')),
                _ => None,
            };
            if let Some((pos, op)) = opt {
                let (left, right) = s.split_at(pos);
                let mut left_label = left.trim().to_string();
                let mut right_tok = right[1..].trim().to_string(); // skip the '+'/'-'
                if right_tok.is_empty() {
                    return None;
                }
                // if there is no left label (e.g., token begins with '-' or '+'), do not
                // treat this as label op number; allow numeric parser to handle unary
                if left_label.is_empty() {
                    return None;
                }
                // strip surrounding parentheses
                if left_label.starts_with('(') && left_label.ends_with(')') {
                    left_label = left_label[1..left_label.len() - 1].to_string();
                }
                if right_tok.starts_with('(') && right_tok.ends_with(')') {
                    right_tok = right_tok[1..right_tok.len() - 1].to_string();
                }

                // try numeric on right side
                if let Some(n) = parse_numeric_token(&right_tok) {
                    let expr = crate::common::generate_expression(
                        &op.to_string(),
                        crate::common::AbstractExpression::Abstract(left_label.trim().to_string()),
                        crate::common::AbstractExpression::Immediate(n),
                    );
                    return Some(DataToken::Expr(expr));
                }

                // otherwise treat as label op label
                let expr = crate::common::generate_expression(
                    &op.to_string(),
                    crate::common::AbstractExpression::Abstract(left_label.trim().to_string()),
                    crate::common::AbstractExpression::Abstract(right_tok.trim().to_string()),
                );
                return Some(DataToken::Expr(expr));
            }
            None
        }

        let mut items: Vec<DataItem> = Vec::new();
        for def in data_defs.iter() {
            if let Some(it) = parse_data_def(def) {
                items.push(it);
            }
        }

        // Emission descriptors for two-pass emission. First pass computes
        // addresses and records labels; second pass writes bytes into memory
        // using resolved labels.
        #[derive(Debug)]
        enum Emission {
            Bytes {
                addr: i64,
                bytes: Vec<i64>,
            },
            // generic expression emission: evaluate expr -> little-endian bytes of given size
            Expr {
                addr: i64,
                expr: crate::common::AbstractExpression,
                size: usize,
            },
        }

        let mut emissions: Vec<Emission> = Vec::new();

        // First pass: compute addresses and record labels/emissions
        for it in items.iter() {
            match it {
                DataItem::Label(l) => {
                    computer.add_memory_label(l.clone(), address);
                }
                DataItem::Directive(d) => match d {
                    DataDirective::Align(n) => {
                        if *n > 0 {
                            let rem = address % *n;
                            if rem != 0 {
                                address += *n - rem;
                            }
                        }
                    }
                    DataDirective::Byte(vec) => {
                        for tok in vec.iter() {
                            match tok {
                                DataToken::Num(n) => {
                                    emissions.push(Emission::Bytes {
                                        addr: address,
                                        bytes: vec![*n],
                                    });
                                    address += 1;
                                }
                                DataToken::Expr(e) => {
                                    emissions.push(Emission::Expr {
                                        addr: address,
                                        expr: e.clone(),
                                        size: 1,
                                    });
                                    address += 1;
                                }
                            }
                        }
                    }
                    DataDirective::Long(vec) => {
                        for tok in vec.iter() {
                            match tok {
                                DataToken::Num(num) => {
                                    let u = *num as i64 as i128 as u128 as u64;
                                    let mut bytes: Vec<i64> = Vec::new();
                                    for i in 0..4 {
                                        let b = ((u >> (8 * i)) & 0xff) as i64;
                                        bytes.push(b);
                                    }
                                    emissions.push(Emission::Bytes {
                                        addr: address,
                                        bytes,
                                    });
                                    address += 4;
                                }
                                DataToken::Expr(e) => {
                                    emissions.push(Emission::Expr {
                                        addr: address,
                                        expr: e.clone(),
                                        size: 4,
                                    });
                                    address += 4;
                                }
                            }
                        }
                    }
                    DataDirective::Quad(vec) => {
                        for tok in vec.iter() {
                            match tok {
                                DataToken::Num(num) => {
                                    let u = *num as i64 as i128 as u128 as u64;
                                    let mut bytes: Vec<i64> = Vec::new();
                                    for i in 0..8 {
                                        let b = ((u >> (8 * i)) & 0xff) as i64;
                                        bytes.push(b);
                                    }
                                    emissions.push(Emission::Bytes {
                                        addr: address,
                                        bytes,
                                    });
                                    address += 8;
                                }
                                DataToken::Expr(e) => {
                                    emissions.push(Emission::Expr {
                                        addr: address,
                                        expr: e.clone(),
                                        size: 8,
                                    });
                                    address += 8;
                                }
                            }
                        }
                    }
                    DataDirective::StringLit(s, is_asciz) => {
                        let mut bytes: Vec<i64> = s.bytes().map(|b| b as i64).collect();
                        if *is_asciz {
                            bytes.push(0);
                        }
                        // emit each byte as a contiguous Bytes emission
                        if !bytes.is_empty() {
                            emissions.push(Emission::Bytes {
                                addr: address,
                                bytes: bytes.clone(),
                            });
                            address += bytes.len() as i64;
                        }
                    }
                    DataDirective::Globl(name) => {
                        computer.add_memory_label(name.clone(), address);
                    }
                    DataDirective::Section(_s) => {
                        // section affects parsing but at this stage we do not change address
                    }
                    DataDirective::Halfword(vec) => {
                        for tok in vec.iter() {
                            match tok {
                                DataToken::Num(num) => {
                                    let low = (num & 0xff) as i64;
                                    let high = ((num >> 8) & 0xff) as i64;
                                    emissions.push(Emission::Bytes {
                                        addr: address,
                                        bytes: vec![low, high],
                                    });
                                    address += 2;
                                }
                                DataToken::Expr(e) => {
                                    emissions.push(Emission::Expr {
                                        addr: address,
                                        expr: e.clone(),
                                        size: 2,
                                    });
                                    address += 2;
                                }
                            }
                        }
                    }
                    DataDirective::Other(_s) => {
                        // ignore other unknown directives conservatively
                    }
                },
            }
        }

        // Second pass: resolve label-offset emissions where possible and write
        // bytes into computer.memory. If a label remains unresolved, fall back
        // to conservative zero bytes at the intended address.
        for em in emissions.iter() {
            match em {
                Emission::Bytes { addr, bytes } => {
                    for (i, b) in bytes.iter().enumerate() {
                        computer.add_memory_value("memory".to_string(), addr + i as i64, *b);
                    }
                }
                Emission::Expr { addr, expr, size } => {
                    if let Some(val) =
                        crate::common::try_resolve_expr(expr, &computer.memory_labels, &computer)
                    {
                        let u = val as i128 as u128 as u64;
                        for i in 0..*size {
                            let b = ((u >> (8 * i)) & 0xff) as i64;
                            computer.add_memory_value("memory".to_string(), addr + i as i64, b);
                        }
                    } else {
                        // conservative placeholder zeros
                        for i in 0..*size {
                            computer.add_memory_value("memory".to_string(), addr + i as i64, 0);
                        }
                        // record relocation metadata so caller can resolve later if desired
                        computer.add_relocation(*addr, expr.clone(), *size);
                    }
                }
            }
        }

        // Diagnostic printing for small unit-test files to help debug parsing issues.
        // keep debug logging via the log crate rather than direct println!
        log::debug!(
            "[debug parse] program len={} label_map={:?}",
            program.len(),
            label_map
        );
        for (i, ins) in program.iter().enumerate() {
            log::debug!("[debug parse] {}: {} {:?}", i, ins.opcode, ins.operands);
        }

        ExecutionEngineX86 {
            computer,
            program,
            label_map,
            jump_history: Vec::new(),
            in_loop: false,
        }
    }

    /// Return and clear any recorded relocations from the underlying computer
    pub fn take_relocations(&mut self) -> Vec<crate::x86_64::computer::Relocation> {
        self.computer.take_relocations()
    }

    /// Return a human-readable list of current relocations (does not clear them)
    pub fn list_relocations(&self) -> Vec<String> {
        self.computer
            .relocations
            .iter()
            .map(|r| format!("addr={} size={} expr={}", r.addr, r.size, r.expr))
            .collect()
    }

    /// Export relocations as JSON via the underlying computer
    pub fn export_relocations_json(&self) -> String {
        self.computer.export_relocations_json()
    }

    /// Export relocations to a file path via the underlying computer
    pub fn export_relocations_to_file(&self, path: &str) -> std::io::Result<()> {
        self.computer.export_relocations_to_file(path)
    }

    /// Export linker-style relocation records to a file
    pub fn export_linker_relocs_to_file(&self, path: &str) -> std::io::Result<()> {
        self.computer.export_linker_relocs_to_file(path)
    }

    /// Return a snapshot of current memory labels as strings for debugging
    pub fn dump_memory_labels(&self) -> Vec<String> {
        self.computer
            .memory_labels
            .iter()
            .map(|(k, v)| format!("{} -> {}", k, v))
            .collect()
    }

    // Evaluate a branch condition using the solver and a simple loop heuristic.
    // Returns Some(true) if the taken branch is the only feasible one,
    // Some(false) if only not-taken is feasible, or None if both need exploring.
    fn evaluate_branch_condition(
        &mut self,
        pc: usize,
        expression: AbstractComparison,
        rw_list: Vec<crate::common::MemoryAccess>,
    ) -> Option<bool> {
        // try solver-based quick checks
        let ctx = self.computer.context;
        if let Some(cond) = comparison_to_ast(ctx, expression.clone()) {
            let taken = self
                .computer
                .solver
                .check_assumptions(&[cond.clone().simplify()]);
            let not_taken = self
                .computer
                .solver
                .check_assumptions(&[cond.not().simplify()]);
            // use log::debug instead of println!
            log::debug!(
                "[dbg eval_branch] pc={} expr={} taken={:?} not_taken={:?}",
                pc,
                expression,
                taken,
                not_taken
            );
            log::debug!(
                "[dbg] eval_branch pc={} expr={} taken={:?} not_taken={:?} rw_list_len={}",
                pc,
                expression,
                taken,
                not_taken,
                rw_list.len()
            );
            // If one side is provably UNSAT, that branch is infeasible -> choose the other.
            match (taken, not_taken) {
                (SatResult::Sat, SatResult::Unsat) => return Some(true),
                (SatResult::Unsat, SatResult::Sat) => return Some(false),
                (SatResult::Unsat, _) => return Some(false),
                (_, SatResult::Unsat) => return Some(true),
                // both satisfiable -> need to explore both
                (SatResult::Sat, SatResult::Sat) => (),
                // any Unknown result is treated conservatively: if neither side is Unsat we must explore both
                _ => return None,
            }
        }

        // Simple heuristic copied from ARM engine: if expression mentions abstract
        // loop-related symbols, try K/K+1 reasoning by inspecting jump_history.
        // If the comparison mentions any abstract symbols (registers/labels),
        // be conservative and do not attempt solver-driven loop heuristics here.
        // Returning None will make the caller explore both branches.
        if !expression.get_abstracts().is_empty() {
            return None;
        }

        if !self.in_loop {
            for j in self.jump_history.clone().into_iter().rev() {
                let (last_jump_label, branch_decision, _, last_rw_list, _last_state) = j;
                if last_jump_label == pc {
                    // If the memory access patterns for this path and the previous
                    // one are identical, prefer the same branch decision (simple heuristic).
                    if last_rw_list == rw_list {
                        return Some(branch_decision);
                    }

                    // Conservative behavior: only use a very cheap heuristic here.
                    // If the previous jump at the same pc had an identical memory
                    // access pattern, prefer the same branch decision. Avoid
                    // pushing assertions into the engine-level solver or trying
                    // to perform K/K+1 reasoning here — that can interfere with
                    // other paths. If we can't decide, fall back to exploring
                    // both branches.
                    if last_rw_list == rw_list {
                        return Some(branch_decision);
                    }
                }
            }
            return None;
        } else {
            // in loop: try simple K+1 detection -- if we see a matching history entry
            for j in self.jump_history.clone().into_iter().rev() {
                let (last_jump_label, branch_decision, last_jump_exp, _last_rw_list, _last_state) =
                    j;
                if last_jump_label == pc && last_jump_exp == expression {
                    // pop the pushed context and assert the condition directly
                    self.computer.solver.pop(1);
                    if let Some(condition) =
                        comparison_to_ast(self.computer.context, expression.clone())
                    {
                        self.computer.solver.assert(&condition.simplify());
                    }
                    self.in_loop = false;
                    return Some(!branch_decision);
                }
            }
            return None;
        }
    }

    pub fn start(&mut self, _label: &str) -> Result<(), String> {
        // Worklist-based interpreter: each work item has a pc, a cloned computer, and optional last cmp
        #[derive(Clone)]
        struct State<'a> {
            id: usize,
            pc: usize,
            comp: X86Computer<'a>,
            last_cmp: Option<AbstractComparison>,
            rw_list: Vec<crate::common::MemoryAccess>,
        }

        // start at first instruction
        let mut work: VecDeque<State> = VecDeque::new();
        // local counter to assign stable ids to states so traces can be correlated
        let mut next_state_id: usize = 1;

        // helpers to push states onto the worklist while logging and assigning ids
        fn push_front_log<'a>(
            work: &mut VecDeque<State<'a>>,
            mut s: State<'a>,
            next_id: &mut usize,
        ) {
            s.id = *next_id;
            *next_id += 1;
            work.push_front(s);
            log::debug!(
                "[queue] pushed_front id={} pc={} queue_len={}",
                work.front().unwrap().id,
                work.front().unwrap().pc,
                work.len()
            );
        }
        fn push_back_log<'a>(
            work: &mut VecDeque<State<'a>>,
            mut s: State<'a>,
            next_id: &mut usize,
        ) {
            s.id = *next_id;
            *next_id += 1;
            work.push_back(s);
            let back = work.back().unwrap();
            log::debug!(
                "[queue] pushed_back id={} pc={} queue_len={}",
                back.id,
                back.pc,
                work.len()
            );
        }

        push_back_log(
            &mut work,
            State {
                id: 0,
                pc: 0,
                comp: self.computer.clone_shallow(),
                last_cmp: None,
                rw_list: Vec::new(),
            },
            &mut next_state_id,
        );

        // collect terminal (completed) states so we can optionally commit
        // a single-path result back into self.computer for tests that
        // expect the engine's canonical computer to reflect final state.
        let mut terminal_states: Vec<State> = Vec::new();

        while let Some(mut state) = work.pop_front() {
            log::debug!(
                "[trace] [pop] id={} pc={} rcx={:?} rw_list_len={} queue_len={}",
                state.id,
                state.pc,
                state.comp.get_register_value("rcx"),
                state.rw_list.len(),
                work.len()
            );
            if state.pc < self.program.len() {
                let ins = &self.program[state.pc];
                log::debug!(
                    "[trace] executing pc={} opcode={} operands={:?}",
                    state.pc,
                    ins.opcode,
                    ins.operands
                );
            }
            if state.pc >= self.program.len() {
                // reached end of program for this path -> record terminal state
                terminal_states.push(state);
                continue;
            }

            let ins = &self.program[state.pc];
            // clone opcode to avoid holding immutable borrow on self while we
            // call methods that need &mut self (e.g., evaluate_branch_condition)
            let opcode = ins.opcode.clone();

            match opcode.as_str() {
                "cmp" | "test" => {
                    // record comparison operands; actual relational operator will be
                    // determined by the following conditional jump mnemonic.
                    if ins.operands.len() >= 2 {
                        let left = operand_to_expr(&ins.operands[0], &state.comp);
                        let right = operand_to_expr(&ins.operands[1], &state.comp);
                        if ins.opcode == "test" {
                            // common pattern: test reg, reg -> check reg == 0
                            if let X86Operand::Reg(a) = &ins.operands[0] {
                                if let X86Operand::Reg(b) = &ins.operands[1] {
                                    if a == b {
                                        state.last_cmp = Some(generate_comparison(
                                            "==",
                                            left,
                                            AbstractExpression::Immediate(0),
                                        ));
                                    } else {
                                        // unsupported complex test -> clear
                                        state.last_cmp = None;
                                    }
                                } else {
                                    state.last_cmp = None;
                                }
                            } else {
                                state.last_cmp = None;
                            }
                        } else {
                            state.last_cmp = Some(generate_comparison("==", left, right));
                        }
                    }
                    state.pc += 1;
                    work.push_front(state);
                    continue;
                }
                // conditional jumps: je/jz -> equal, jne/jnz -> not equal
                "je" | "jz" | "jne" | "jnz" => {
                    log::debug!(
                        "[dbg] conditional jump encountered at pc={} opcode={} last_cmp_is_some={}",
                        state.pc,
                        opcode,
                        state.last_cmp.is_some()
                    );
                    log::debug!("[dbg] entering conditional handling for pc={}", state.pc);
                    // resolve target label
                    if ins.operands.is_empty() {
                        return Err(format!("Conditional jump with no target: {:?}", ins));
                    }
                    // target expected to be a label operand
                    let target_pc = if let X86Operand::Label(l) = &ins.operands[0] {
                        match self.label_map.get(l) {
                            Some(idx) => *idx,
                            None => {
                                // unknown label -> skip
                                state.pc += 1;
                                work.push_front(state);
                                continue;
                            }
                        }
                    } else {
                        state.pc += 1;
                        work.push_front(state);
                        continue;
                    };

                    // if we don't have a last comparison, conservatively explore both
                    if state.last_cmp.is_none() {
                        log::debug!(
                            "[dbg] no last_cmp at pc={}, splitting both conservatively",
                            state.pc
                        );
                        // taken
                        let mut taken = state.clone();
                        // ensure fresh solver for each clone to avoid cross-path interference
                        taken.comp = state.comp.clone_shallow();
                        taken.pc = target_pc;
                        taken.last_cmp = None;
                        // not-taken
                        let mut not_taken = state.clone();
                        not_taken.comp = state.comp.clone_shallow();
                        not_taken.pc = state.pc + 1;
                        not_taken.last_cmp = None;
                        push_front_log(&mut work, not_taken, &mut next_state_id);
                        push_front_log(&mut work, taken, &mut next_state_id);
                        continue;
                    }

                    // Try lightweight solver-driven pruning/heuristics.
                    // evaluate_branch_condition will return Some(true) if only the
                    // taken branch is feasible, Some(false) if only not-taken is
                    // feasible, or None if both should be explored.
                    if let Some(cond) = state.last_cmp.clone() {
                        if let Some(decision) = self.evaluate_branch_condition(
                            state.pc,
                            cond.clone(),
                            state.rw_list.clone(),
                        ) {
                            // decision describes feasibility of `cond` being true vs false.
                            // Map that to whether the branch target is taken for this mnemonic.
                            let taken_when_cond_true = match opcode.as_str() {
                                "je" | "jz" => true,
                                "jne" | "jnz" => false,
                                _ => true,
                            };
                            if decision == taken_when_cond_true {
                                // only taken
                                let mut taken = state.clone();
                                taken.comp = state.comp.clone_shallow();
                                taken.pc = target_pc;
                                taken.last_cmp = None;
                                log::debug!(
                                    "[dbg] prune: only taken at pc={} taken_pc={} rcx={:?}",
                                    state.pc,
                                    taken.pc,
                                    state.comp.get_register_value("rcx")
                                );
                                push_front_log(&mut work, taken, &mut next_state_id);
                                continue;
                            } else {
                                // only not-taken
                                let mut not_taken = state.clone();
                                not_taken.comp = state.comp.clone_shallow();
                                not_taken.pc = state.pc + 1;
                                not_taken.last_cmp = None;
                                log::debug!(
                                    "[dbg] prune: only not-taken at pc={} not_taken_pc={} rcx={:?}",
                                    state.pc,
                                    not_taken.pc,
                                    state.comp.get_register_value("rcx")
                                );
                                push_front_log(&mut work, not_taken, &mut next_state_id);
                                continue;
                            }
                        }
                    }

                    // Fallback: explore both conservatively
                    let mut taken = state.clone();
                    taken.comp = state.comp.clone_shallow();
                    taken.pc = target_pc;
                    taken.last_cmp = None;
                    let mut not_taken = state.clone();
                    not_taken.comp = state.comp.clone_shallow();
                    not_taken.pc = state.pc + 1;
                    not_taken.last_cmp = None;

                    log::debug!(
                        "[dbg] conservative split at pc={} taken_pc={} not_taken_pc={} rcx={:?}",
                        state.pc,
                        taken.pc,
                        not_taken.pc,
                        state.comp.get_register_value("rcx"),
                    );

                    push_front_log(&mut work, not_taken, &mut next_state_id);
                    push_front_log(&mut work, taken, &mut next_state_id);
                    continue;
                }
                "jmp" => {
                    if ins.operands.is_empty() {
                        return Err(format!("jmp with no target: {:?}", ins));
                    }
                    if let X86Operand::Label(l) = &ins.operands[0] {
                        if let Some(idx) = self.label_map.get(l) {
                            state.pc = *idx;
                            work.push_front(state);
                            continue;
                        }
                    }
                    // unknown target -> skip
                    continue;
                }
                _ => {}
            }

            // handle register-manipulating instructions similar to prior linear interpreter
            match opcode.as_str() {
                "lea" => {
                    if ins.operands.len() >= 2 {
                        if let X86Operand::Reg(dst) = &ins.operands[0] {
                            if let X86Operand::Mem {
                                base,
                                index,
                                scale,
                                disp,
                            } = &ins.operands[1]
                            {
                                let mut expr = if let Some(bn) = base {
                                    AbstractExpression::Abstract(bn.clone())
                                } else {
                                    AbstractExpression::Immediate(0)
                                };
                                if let Some(idx_name) = index {
                                    let idx_expr = AbstractExpression::Abstract(idx_name.clone());
                                    let scaled = if *scale != 1 {
                                        generate_expression(
                                            "*",
                                            idx_expr,
                                            AbstractExpression::Immediate(*scale),
                                        )
                                    } else {
                                        idx_expr
                                    };
                                    expr = generate_expression("+", expr, scaled);
                                }
                                if *disp != 0 {
                                    expr = generate_expression(
                                        "+",
                                        expr,
                                        AbstractExpression::Immediate(*disp),
                                    );
                                }
                                state.comp.set_register_abstract(dst, Some(expr), 0);
                                state.pc += 1;
                                work.push_front(state);
                                continue;
                            }
                        }
                    }
                }
                "mov" => {
                    if ins.operands.len() >= 2 {
                        match (&ins.operands[0], &ins.operands[1]) {
                            (X86Operand::Reg(dst), X86Operand::Imm(imm)) => {
                                // debug: trace reg immediate writes for failing test
                                state.comp.write_register_from_imm(dst, *imm);
                                state.pc += 1;
                                work.push_front(state);
                                continue;
                            }
                            (X86Operand::Reg(dst), X86Operand::Reg(src)) => {
                                let val = state.comp.get_register_value(src);
                                state.comp.write_register_from_value(dst, val);
                                state.pc += 1;
                                work.push_front(state);
                                continue;
                            }
                            _ => {}
                        }
                    }
                }
                "add" | "sub" => {
                    if ins.operands.len() >= 2 {
                        if let X86Operand::Reg(dst) = &ins.operands[0] {
                            match &ins.operands[1] {
                                X86Operand::Imm(imm) => {
                                    let val = state.comp.get_register_value(dst);
                                    let new_offset =
                                        val.offset + if ins.opcode == "add" { *imm } else { -*imm };
                                    match val.kind {
                                        crate::common::RegisterKind::Immediate => {
                                            state.comp.set_register_imm(dst, new_offset);
                                        }
                                        _ => {
                                            state.comp.set_register_abstract(
                                                dst,
                                                val.base.clone(),
                                                new_offset,
                                            );
                                        }
                                    }
                                    state.pc += 1;
                                    work.push_front(state);
                                    continue;
                                }
                                X86Operand::Reg(src) => {
                                    let s = state.comp.get_register_value(src);
                                    let d = state.comp.get_register_value(dst);
                                    let left = if let Some(b) = d.base {
                                        b
                                    } else {
                                        AbstractExpression::Immediate(d.offset)
                                    };
                                    let right = if let Some(b) = s.base {
                                        b
                                    } else {
                                        AbstractExpression::Immediate(s.offset)
                                    };
                                    let expr = generate_expression("+", left, right);
                                    state.comp.set_register_abstract(dst, Some(expr), 0);
                                    state.pc += 1;
                                    work.push_front(state);
                                    continue;
                                }
                                _ => {}
                            }
                        }
                    }
                }
                _ => {}
            }

            // memory operations: vector and scalar reads
            match opcode.as_str() {
                "vmovdqu" | "vmovd" => {
                    if ins.operands.len() >= 2 {
                        if let X86Operand::Mem {
                            base,
                            index: _,
                            scale: _,
                            disp: _,
                        } = &ins.operands[1]
                        {
                            let base_expr = if let Some(b) = base {
                                AbstractExpression::Abstract(b.clone())
                            } else {
                                AbstractExpression::Immediate(0)
                            };
                            // reconstruct addr_expr from mem operand
                            let mut addr_expr = base_expr;
                            if let X86Operand::Mem {
                                base: bopt,
                                index: iopt,
                                scale: sc,
                                disp: d,
                            } = &ins.operands[1]
                            {
                                addr_expr = if let Some(bn) = bopt {
                                    AbstractExpression::Abstract(bn.clone())
                                } else {
                                    AbstractExpression::Immediate(0)
                                };
                                if let Some(idx_name) = iopt {
                                    let idx_expr = AbstractExpression::Abstract(idx_name.clone());
                                    let scaled = if *sc != 1 {
                                        generate_expression(
                                            "*",
                                            idx_expr,
                                            AbstractExpression::Immediate(*sc),
                                        )
                                    } else {
                                        idx_expr
                                    };
                                    addr_expr = generate_expression("+", addr_expr, scaled);
                                }
                                if *d != 0 {
                                    addr_expr = generate_expression(
                                        "+",
                                        addr_expr,
                                        AbstractExpression::Immediate(*d),
                                    );
                                }
                            }

                            // Try to resolve a concrete base address (label + immediate)
                            // so mem_safe_access can operate on concrete addresses when possible.
                            let concrete_base_opt = crate::common::try_resolve_expr(
                                &addr_expr,
                                &state.comp.memory_labels,
                                &state.comp,
                            );
                            let start_base_expr = if let Some(addr) = concrete_base_opt {
                                AbstractExpression::Immediate(addr)
                            } else {
                                addr_expr.clone()
                            };

                            let start_check = state.comp.mem_safe_access(
                                start_base_expr.clone(),
                                0,
                                crate::common::RegionType::READ,
                            );
                            // record the memory read in this path's rw_list so loop heuristics
                            // and branch reasoning can consider memory access patterns.
                            // Try to represent the access with a label + offset when possible.
                            fn resolve_access_base(
                                expr: &AbstractExpression,
                                labels: &std::collections::HashMap<String, i64>,
                            ) -> (String, i64) {
                                match expr {
                                    AbstractExpression::Abstract(name) => {
                                        if let Some(_addr) = labels.get(name) {
                                            return (name.clone(), 0);
                                        }
                                        return ("memory".to_string(), 0);
                                    }
                                    AbstractExpression::Expression(op, a, b) => {
                                        if op == "+" {
                                            if let AbstractExpression::Abstract(name) = &**a {
                                                if let AbstractExpression::Immediate(off) = &**b {
                                                    return (name.clone(), *off);
                                                }
                                            }
                                        }
                                        ("memory".to_string(), 0)
                                    }
                                    AbstractExpression::Immediate(_) => ("memory".to_string(), 0),
                                    _ => ("memory".to_string(), 0),
                                }
                            }
                            let (access_base, access_offset) =
                                resolve_access_base(&addr_expr, &state.comp.memory_labels);
                            state.rw_list.push(crate::common::MemoryAccess {
                                kind: crate::common::RegionType::READ,
                                base: access_base,
                                offset: access_offset,
                            });
                            let end_addr = generate_expression(
                                "+",
                                start_base_expr.clone(),
                                AbstractExpression::Immediate(31),
                            );
                            let end_check = state.comp.mem_safe_access(
                                end_addr,
                                0,
                                crate::common::RegionType::READ,
                            );
                            match (start_check, end_check) {
                                (Ok(_), Ok(_)) => {
                                    if let X86Operand::Reg(dst) = &ins.operands[0] {
                                        if let Some(vec) = state.comp.simd_registers.get_mut(dst) {
                                            // try to resolve a concrete label base so we can
                                            // populate per-byte lane bases referencing the
                                            // original label (e.g., ".L_table + i").
                                            fn resolve_label_addr(
                                                expr: &AbstractExpression,
                                                labels: &std::collections::HashMap<String, i64>,
                                            ) -> Option<(String, i64)>
                                            {
                                                match expr {
                                                    AbstractExpression::Abstract(name) => {
                                                        if let Some(addr) = labels.get(name) {
                                                            return Some((name.clone(), *addr));
                                                        }
                                                        None
                                                    }
                                                    AbstractExpression::Expression(op, a, b) => {
                                                        // only support label + immediate here
                                                        if op == "+" {
                                                            if let AbstractExpression::Abstract(
                                                                name,
                                                            ) = &**a
                                                            {
                                                                if let AbstractExpression::Immediate(off) = &**b {
                                                                    if let Some(addr) = labels.get(name) {
                                                                        return Some((name.clone(), addr + off));
                                                                    }
                                                                }
                                                            }
                                                        }
                                                        None
                                                    }
                                                    _ => None,
                                                }
                                            }

                                            if let Some((label, _base_addr)) = resolve_label_addr(
                                                &addr_expr,
                                                &state.comp.memory_labels,
                                            ) {
                                                for i in 0..vec.len() {
                                                    let byte_base = generate_expression(
                                                        "+",
                                                        AbstractExpression::Abstract(label.clone()),
                                                        AbstractExpression::Immediate(i as i64),
                                                    );
                                                    vec[i] = RegisterValue::new(
                                                        RegisterKind::RegisterBase,
                                                        Some(byte_base),
                                                        0,
                                                    );
                                                }
                                            } else {
                                                for i in 0..vec.len() {
                                                    let byte_addr = generate_expression(
                                                        "+",
                                                        addr_expr.clone(),
                                                        AbstractExpression::Immediate(i as i64),
                                                    );
                                                    vec[i] = RegisterValue::new(
                                                        RegisterKind::RegisterBase,
                                                        Some(byte_addr),
                                                        0,
                                                    );
                                                }
                                            }
                                        }
                                    }
                                    // already recorded the memory read above
                                    state.pc += 1;
                                    work.push_front(state);
                                    continue;
                                }
                                (a, b) => {
                                    return Err(format!(
                                        "Memory safety error: start {:?} end {:?}",
                                        a.err().map(|e| e.to_string()),
                                        b.err().map(|e| e.to_string())
                                    ));
                                }
                            }
                        }
                    }
                }
                "mov" => {
                    if ins.operands.len() >= 2 {
                        if let X86Operand::Mem {
                            base,
                            index: _,
                            scale: _,
                            disp: _,
                        } = &ins.operands[1]
                        {
                            let mut width = 8i64;
                            if let X86Operand::Reg(rn) = &ins.operands[0] {
                                if rn.ends_with('d') || rn.starts_with('e') {
                                    width = 4;
                                } else if rn.ends_with('b') {
                                    width = 1;
                                }
                            }

                            let mut addr_expr = if let Some(bn) = base {
                                AbstractExpression::Abstract(bn.clone())
                            } else {
                                AbstractExpression::Immediate(0)
                            };
                            if let X86Operand::Mem {
                                base: _bopt,
                                index: iopt,
                                scale: sc,
                                disp: d,
                            } = &ins.operands[1]
                            {
                                if let Some(idx_name) = iopt {
                                    let idx_expr = AbstractExpression::Abstract(idx_name.clone());
                                    let scaled = if *sc != 1 {
                                        generate_expression(
                                            "*",
                                            idx_expr,
                                            AbstractExpression::Immediate(*sc),
                                        )
                                    } else {
                                        idx_expr
                                    };
                                    addr_expr = generate_expression("+", addr_expr, scaled);
                                }
                                if *d != 0 {
                                    addr_expr = generate_expression(
                                        "+",
                                        addr_expr,
                                        AbstractExpression::Immediate(*d),
                                    );
                                }
                            }

                            // record the memory read access for branch heuristics
                            fn resolve_access_base(
                                expr: &AbstractExpression,
                                labels: &std::collections::HashMap<String, i64>,
                            ) -> (String, i64) {
                                match expr {
                                    AbstractExpression::Abstract(name) => {
                                        if let Some(_addr) = labels.get(name) {
                                            return (name.clone(), 0);
                                        }
                                        return ("memory".to_string(), 0);
                                    }
                                    AbstractExpression::Expression(op, a, b) => {
                                        if op == "+" {
                                            if let AbstractExpression::Abstract(name) = &**a {
                                                if let AbstractExpression::Immediate(off) = &**b {
                                                    return (name.clone(), *off);
                                                }
                                            }
                                        }
                                        ("memory".to_string(), 0)
                                    }
                                    AbstractExpression::Immediate(_) => ("memory".to_string(), 0),
                                    _ => ("memory".to_string(), 0),
                                }
                            }
                            let (access_base, access_offset) =
                                resolve_access_base(&addr_expr, &state.comp.memory_labels);
                            state.rw_list.push(crate::common::MemoryAccess {
                                kind: crate::common::RegionType::READ,
                                base: access_base,
                                offset: access_offset,
                            });

                            // Try resolving to concrete address when possible
                            let concrete_base_opt = crate::common::try_resolve_expr(
                                &addr_expr,
                                &state.comp.memory_labels,
                                &state.comp,
                            );
                            let start_base_expr = if let Some(addr) = concrete_base_opt {
                                AbstractExpression::Immediate(addr)
                            } else {
                                addr_expr.clone()
                            };

                            let start_check = state.comp.mem_safe_access(
                                start_base_expr.clone(),
                                0,
                                crate::common::RegionType::READ,
                            );
                            let end_addr = generate_expression(
                                "+",
                                start_base_expr.clone(),
                                AbstractExpression::Immediate(width - 1),
                            );
                            let end_check = state.comp.mem_safe_access(
                                end_addr,
                                0,
                                crate::common::RegionType::READ,
                            );
                            match (start_check, end_check) {
                                (Ok(_), Ok(_)) => {
                                    if let X86Operand::Reg(dst_name) = &ins.operands[0] {
                                        // attempt to resolve concrete addr -> use memory_labels
                                        fn resolve_label_addr(
                                            expr: &AbstractExpression,
                                            labels: &std::collections::HashMap<String, i64>,
                                        ) -> Option<i64> {
                                            match expr {
                                                AbstractExpression::Immediate(n) => Some(*n),
                                                AbstractExpression::Abstract(name) => {
                                                    labels.get(name).cloned()
                                                }
                                                AbstractExpression::Expression(op, a, b) => {
                                                    if op != "+" {
                                                        return None;
                                                    }
                                                    let la = resolve_label_addr(a, labels);
                                                    let lb = resolve_label_addr(b, labels);
                                                    match (la, lb) {
                                                        (Some(x), Some(y)) => Some(x + y),
                                                        _ => None,
                                                    }
                                                }
                                                _ => None,
                                            }
                                        }

                                        if let Some(concrete_addr) = resolve_label_addr(
                                            &addr_expr,
                                            &state.comp.memory_labels,
                                        ) {
                                            if let Some(memreg) = state.comp.memory.get("memory") {
                                                if let Some(mv) = memreg.get(concrete_addr) {
                                                    match mv.kind {
                                                        crate::common::RegisterKind::Immediate => {
                                                            let mut v = mv.offset;
                                                            if dst_name.ends_with('d')
                                                                || dst_name.starts_with('e')
                                                            {
                                                                v = (v as u32) as i64;
                                                            }
                                                            state
                                                                .comp
                                                                .set_register_imm(dst_name, v);
                                                            state.pc += 1;
                                                            work.push_front(state);
                                                            continue;
                                                        }
                                                        _ => {}
                                                    }
                                                }
                                            }
                                        }

                                        state.comp.set_register_abstract(
                                            dst_name,
                                            Some(addr_expr.clone()),
                                            0,
                                        );
                                    }
                                    state.pc += 1;
                                    work.push_front(state);
                                    continue;
                                }
                                (a, b) => {
                                    return Err(format!(
                                        "Memory safety error: start {:?} end {:?}",
                                        a.err().map(|e| e.to_string()),
                                        b.err().map(|e| e.to_string())
                                    ));
                                }
                            }
                        }
                    }
                }
                _ => {}
            }

            // default: advance PC
            state.pc += 1;
            // clear last_cmp for next instruction unless explicitly carried
            state.last_cmp = None;
            work.push_front(state);
        }

        // If one or more terminal states were reached, commit a conservative
        // merged computer back into the engine so callers can inspect final
        // registers. For a single terminal state we simply move it back; for
        // multiple states we merge conservatively: if a register has the same
        // RegisterValue across all terminals keep it, otherwise mark as Number.
        if terminal_states.len() == 1 {
            self.computer = terminal_states.remove(0).comp;
        } else if terminal_states.len() > 1 {
            // start with a shallow clone of the first terminal's computer
            let mut merged = terminal_states[0].comp.clone_shallow();

            // debug: if this is the small branch_prune test, print terminals
            if self.label_map.contains_key("taken") && terminal_states.len() < 10 {
                log::debug!("[dbg terminals] count={}", terminal_states.len());
                for (i, s) in terminal_states.iter().enumerate() {
                    let rax = s.comp.get_register_value("rax");
                    log::debug!("[dbg terminal {}] pc={} rax={:?}", i, s.pc, rax);
                }
            }

            // merge registers
            for (name, _) in merged.registers.clone() {
                let mut values: Vec<crate::common::RegisterValue> = Vec::new();
                for s in &terminal_states {
                    values.push(s.comp.get_register_value(&name));
                }
                let first = values[0].clone();
                let mut ok = true;
                for v in values.iter().skip(1) {
                    if v != &first {
                        ok = false;
                        break;
                    }
                }
                if ok {
                    merged.set_register_value(&name, first.clone());
                } else {
                    merged.set_register_unknown(&name);
                }
            }

            // merge simd registers conservatively: if all lanes equal keep, else set to unknown bases
            for (sname, _vecv) in merged.simd_registers.clone() {
                if let Some(dst_vec) = merged.simd_registers.get_mut(&sname) {
                    for i in 0..dst_vec.len() {
                        let mut vals: Vec<crate::common::RegisterValue> = Vec::new();
                        for t in &terminal_states {
                            if let Some(v) = t.comp.simd_registers.get(&sname) {
                                vals.push(v[i].clone());
                            }
                        }
                        let first = vals[0].clone();
                        let mut same = true;
                        for v in vals.iter().skip(1) {
                            if v != &first {
                                same = false;
                                break;
                            }
                        }
                        if same {
                            dst_vec[i] = first;
                        } else {
                            dst_vec[i] = crate::common::RegisterValue::new(
                                crate::common::RegisterKind::Number,
                                None,
                                0,
                            );
                        }
                    }
                }
            }

            // merge memory labels and memory regions conservatively: if the same mapping exists across all states keep it, else keep original
            // For now prefer the first state's mapping (it's conservative enough).
            merged.memory_labels = terminal_states[0].comp.memory_labels.clone();
            merged.memory = terminal_states[0].comp.memory.clone();

            self.computer = merged;
        }

        Ok(())
    }
}

#[cfg(test)]
mod smoke {
    use super::*;

    #[test]
    fn smoke_run_icx_file() {
        // If the icx file is present in repo root, attempt to run the engine (non-fatal).
        let mut path = std::path::PathBuf::from(
            std::env::var("CARGO_MANIFEST_DIR").unwrap_or(".".to_string()),
        );
        path.push("icx_core_avx2_v3_gas.s");
        if !path.exists() {
            // skip if not present in this environment
            return;
        }
        let cfg = Config::new();
        let ctx = Context::new(&cfg);
        let mut engine = ExecutionEngineX86::from_asm_file(path.to_str().unwrap(), &ctx);
        let _ = engine.start("_start");
    }
}

#[cfg(test)]
mod reg_tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn mov_32bit_to_64bit_zero_extend() {
        // create a tiny asm file: mov eax, <imm>; mov rbx, eax
        let mut tmp = std::env::temp_dir();
        tmp.push("test_mov32.s");
        let mut f = std::fs::File::create(&tmp).expect("create tmp asm");
        let content = "mov eax, 0x11223344\nmov rbx, eax\n";
        f.write_all(content.as_bytes()).expect("write");
        f.flush().unwrap();

        let cfg = Config::new();
        let ctx = Context::new(&cfg);
        let mut engine = ExecutionEngineX86::from_asm_file(tmp.to_str().unwrap(), &ctx);
        // basic sanity-check of parsed program before running
        assert!(
            engine.program.len() >= 2,
            "program too short: {:?}",
            engine.program
        );

        let res = engine.start("test");
        if let Err(e) = &res {
            // surface the error for debugging
            panic!("engine.start returned error: {}", e);
        }
        // check rbx canonical value
        let rbx = engine.computer.get_register_value("rbx");
        assert_eq!(rbx.kind, crate::common::RegisterKind::Immediate);
        assert_eq!(rbx.offset as u64, 0x11223344u64);
    }
}

#[cfg(test)]
mod trunc_tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn mov_64_to_32_truncation_marks_unknown() {
        // mov rax, <large imm>; mov eax, rax -> writing to 32-bit alias from 64-bit
        let mut tmp = std::env::temp_dir();
        tmp.push("test_mov_trunc.s");
        let mut f = std::fs::File::create(&tmp).expect("create tmp asm");
        let content = "mov rax, 0x1122334455667788\nmov eax, rax\n";
        f.write_all(content.as_bytes()).expect("write");
        f.flush().unwrap();

        let cfg = Config::new();
        let ctx = Context::new(&cfg);
        let mut engine = ExecutionEngineX86::from_asm_file(tmp.to_str().unwrap(), &ctx);
        let _res = engine
            .start("test")
            .unwrap_or_else(|e| panic!("engine.start error: {}", e));
        let rax = engine.computer.get_register_value("rax");
        assert_eq!(rax.kind, crate::common::RegisterKind::Number);
    }
}

#[cfg(test)]
mod simd_tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn vmovdqu_populates_simd_register() {
        let mut tmp = std::env::temp_dir();
        tmp.push("test_simd.s");
        let mut f = std::fs::File::create(&tmp).expect("create tmp asm");
        // build a small .L_table with 32 bytes
        let mut content = String::new();
        content.push_str(".L_table:\n");
        content.push_str(".byte ");
        for i in 0..32 {
            content.push_str(&format!("0x{:02x}", i));
            if i != 31 {
                content.push_str(", ");
            }
        }
        content.push_str("\nvmovdqu ymm0, ymmword ptr [.L_table]\n");
        f.write_all(content.as_bytes()).expect("write");
        f.flush().unwrap();

        let cfg = Config::new();
        let ctx = Context::new(&cfg);
        let mut engine = ExecutionEngineX86::from_asm_file(tmp.to_str().unwrap(), &ctx);
        let res = engine.start("test");
        if let Err(e) = &res {
            panic!("engine.start error: {}", e);
        }
        // ensure ymm0 entries reference .L_table
        if let Some(vec) = engine.computer.simd_registers.get("ymm0") {
            let first = &vec[0];
            if let Some(base_expr) = &first.base {
                let abstracts = base_expr.get_abstracts();
                assert!(abstracts.iter().any(|s| s.contains(".L_table")));
            } else {
                panic!("expected base expression for simd element");
            }
        } else {
            panic!("no ymm0 register found");
        }
    }
}

#[cfg(test)]
mod branch_prune_tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn solver_prunes_infeasible_branch() {
        // Program:
        // mov rcx, 0
        // cmp rcx, 1
        // je taken
        // mov rax, 42
        // jmp end
        // taken: mov rax, 7
        // end: ret
        let mut tmp = std::env::temp_dir();
        tmp.push("test_branch_prune.s");
        let mut f = std::fs::File::create(&tmp).expect("create tmp asm");
        let content = "mov rcx, 0\ncmp rcx, 1\nje taken\nmov rax, 42\njmp end\ntaken: \nmov rax, 7\nend:\nret\n";
        f.write_all(content.as_bytes()).expect("write");
        f.flush().unwrap();

        let cfg = Config::new();
        let ctx = Context::new(&cfg);
        let mut engine = ExecutionEngineX86::from_asm_file(tmp.to_str().unwrap(), &ctx);
        let res = engine.start("test");
        if let Err(e) = &res {
            panic!("engine.start error: {}", e);
        }
        // If the solver correctly proves the taken branch infeasible, rax should be 42
        let rax = engine.computer.get_register_value("rax");
        log::debug!("[test dbg] final rax = {:?}", rax);
        assert_eq!(rax.kind, crate::common::RegisterKind::Immediate);
        assert_eq!(rax.offset, 42);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn from_asm_file_parses_data_sections() {
        // create a temporary asm file
        let mut tmp = std::env::temp_dir();
        tmp.push("test_data_asm.s");
        let mut f = std::fs::File::create(&tmp).expect("create tmp asm");
        let content = r#"
.globl L_start
.L_data:
.byte 0x41, 0x42
.long 0x12345678
.quad 0x1122334455667788
.asciz "hello"
"
"#;
        f.write_all(content.as_bytes()).expect("write");
        f.flush().unwrap();

        let cfg = Config::new();
        let ctx = Context::new(&cfg);
        let engine = ExecutionEngineX86::from_asm_file(tmp.to_str().unwrap(), &ctx);
        // labels should include .L_data
        assert!(
            engine.computer.memory_labels.contains_key(".L_data")
                || engine.computer.memory_labels.contains_key(".L_data:")
        );
    }

    #[test]
    fn align_semantics_exponent_and_bytes() {
        let mut tmp = std::env::temp_dir();
        tmp.push("test_align.s");
        let mut f = std::fs::File::create(&tmp).expect("create tmp asm");
        let content = r#"
.L_a:
.byte 0x01
.align 2
.L_b:
.byte 0x02
.align 16
.L_c:
.byte 0x03
"#;
        f.write_all(content.as_bytes()).expect("write");
        f.flush().unwrap();

        let cfg = Config::new();
        let ctx = Context::new(&cfg);
        let engine = ExecutionEngineX86::from_asm_file(tmp.to_str().unwrap(), &ctx);

        let a = engine
            .computer
            .memory_labels
            .get(".L_a")
            .cloned()
            .expect("label a");
        let b = engine
            .computer
            .memory_labels
            .get(".L_b")
            .cloned()
            .expect("label b");
        let c = engine
            .computer
            .memory_labels
            .get(".L_c")
            .cloned()
            .expect("label c");

        // initial base is 4
        assert_eq!(a, 4);
        // after .byte (1) and align 2 (2 -> 1<<2 == 4), .L_b should be at address 8
        assert_eq!(b, 8);
        // after .byte at 8 -> 9, .align 16 (treated as byte count) aligns to 16
        assert_eq!(c, 16);
    }
}

#[cfg(test)]
mod loop_tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn simple_loop_over_table() {
        // create a tiny asm file with a small table and a loop that iterates over it
        let mut tmp = std::env::temp_dir();
        tmp.push("test_loop.s");
        let mut f = std::fs::File::create(&tmp).expect("create tmp asm");
        let mut content = String::new();
        content.push_str(".L_table:\n");
        // 8 qwords -> 64 bytes of data so qword loads have concrete backing
        content.push_str(".byte ");
        for i in 0..64 {
            content.push_str(&format!("0x{:02x}", i));
            if i != 63 {
                content.push_str(", ");
            }
        }

        // Nested test functions removed and moved to top-level integration tests
        content.push_str("\n");
        content.push_str("mov rcx, 0\n");
        content.push_str(".loop:\n");
        content.push_str("mov rax, qword ptr [.L_table + rcx*8]\n");
        content.push_str("add rcx, 1\n");
        content.push_str("cmp rcx, 8\n");
        content.push_str("jne .loop\n");
        f.write_all(content.as_bytes()).expect("write");
        f.flush().unwrap();

        let cfg = Config::new();
        let ctx = Context::new(&cfg);
        let mut engine = ExecutionEngineX86::from_asm_file(tmp.to_str().unwrap(), &ctx);
        let res = engine.start("test");
        if let Err(e) = &res {
            panic!("engine.start error: {}", e);
        }

        // rcx should be 8 after loop
        let rcx = engine.computer.get_register_value("rcx");
        assert_eq!(rcx.kind, crate::common::RegisterKind::Immediate);
        assert_eq!(rcx.offset, 8);
    }

    #[test]
    fn directive_expression_parsing_multiply_and_byte_suffix() {
        let mut tmp = std::env::temp_dir();
        tmp.push("test_expr_dir.s");
        let mut f = std::fs::File::create(&tmp).expect("create tmp asm");
        let content = r#".L_e:
.hword 16 * 12
.hword 320b
"#;
        f.write_all(content.as_bytes()).expect("write");
        f.flush().unwrap();

        let cfg = Config::new();
        let ctx = Context::new(&cfg);
        let engine = ExecutionEngineX86::from_asm_file(tmp.to_str().unwrap(), &ctx);
        let addr = engine
            .computer
            .memory_labels
            .get(".L_e")
            .cloned()
            .expect("label");
        // First .hword is a single 2-byte entry with value 16*12 = 192 -> low byte 192 (0xc0)
        if let Some(mem) = engine.computer.memory.get("memory") {
            let b0 = mem.get(addr).expect("b0").offset as u8;
            let b1 = mem.get(addr + 1).expect("b1").offset as u8;
            let val = (b1 as u16) << 8 | (b0 as u16);
            assert_eq!(val, 192);
            // second .hword: 320b -> 320 decimal -> 0x0140 -> low byte 0x40
            let b2 = mem.get(addr + 2).expect("b2").offset as u8;
            let b3 = mem.get(addr + 3).expect("b3").offset as u8;
            let val2 = (b3 as u16) << 8 | (b2 as u16);
            assert_eq!(val2, 320);
        } else {
            panic!("memory region missing");
        }
    }

    #[test]
    fn solver_isolation_between_clones() {
        let cfg = Config::new();
        let ctx = Context::new(&cfg);
        let comp = X86Computer::new(&ctx);
        let c1 = comp.clone_shallow();
        let c2 = comp.clone_shallow();

        // create a simple symbolic int 'sym' and assert sym == 1 in c1's solver
        let sym = ast::Int::new_const(&ctx, "sym");
        let one = ast::Int::from_i64(&ctx, 1);
        c1.solver.assert(&sym._eq(&one));

        // In c1, sym == 2 should be UNSAT
        let two = ast::Int::from_i64(&ctx, 2);
        let res1 = c1.solver.check_assumptions(&[sym._eq(&two)]);
        assert_eq!(res1, SatResult::Unsat);

        // In c2 (fresh solver), sym == 2 should be SAT (no assertion was made)
        let res2 = c2.solver.check_assumptions(&[sym._eq(&two)]);
        assert_eq!(res2, SatResult::Sat);
    }

    #[test]
    fn evaluate_branch_prefers_history_when_rw_list_matches() {
        let cfg = Config::new();
        let ctx = Context::new(&cfg);
        // create a tiny empty asm file
        let mut tmp = std::env::temp_dir();
        tmp.push("test_eval_branch.s");
        let mut f = std::fs::File::create(&tmp).expect("create tmp asm");
        f.write_all(b"ret\n").expect("write");
        f.flush().unwrap();

        let mut engine = ExecutionEngineX86::from_asm_file(tmp.to_str().unwrap(), &ctx);
        // craft a comparison expression that is symbolic
        let expr = generate_comparison(
            "==",
            AbstractExpression::Abstract("symx".to_string()),
            AbstractExpression::Immediate(0),
        );
        let pc = 0usize;
        let rw = vec![crate::common::MemoryAccess {
            kind: crate::common::RegionType::READ,
            base: "memory".to_string(),
            offset: 0,
        }];
        // push a history entry that says at pc=0 the previous branch decision was true
        engine.jump_history.push((
            pc,
            true,
            expr.clone(),
            rw.clone(),
            engine.computer.clone_shallow(),
        ));

        // Now calling evaluate_branch_condition should be conservative
        // when the comparison mentions abstract symbols and therefore
        // return None (both branches explored). This verifies we do not
        // attempt aggressive heuristic pruning for abstract comparisons.
        let decision = engine.evaluate_branch_condition(pc, expr.clone(), rw.clone());
        assert_eq!(decision, None);
    }
}
