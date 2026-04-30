#[derive(Debug, Clone, PartialEq)]
pub enum X86Operand {
    Reg(String),
    Imm(i64),
    Mem {
        base: Option<String>,
        index: Option<String>,
        scale: i64,
        disp: i64,
    },
    Label(String),
    Other(String),
}

#[derive(Debug, Clone, PartialEq)]
pub struct X86Instruction {
    pub opcode: String,
    pub operands: Vec<X86Operand>,
}

pub fn parse_x86_line(line: &str) -> Option<X86Instruction> {
    let s = line.trim();
    if s.is_empty() {
        return None;
    }
    if s.starts_with('.') {
        return None;
    }
    if s.ends_with(':') {
        return None;
    }

    // very small parser: split by whitespace for opcode, then split operands by commas
    let mut parts = s.splitn(2, |c: char| c.is_whitespace());
    let opcode = match parts.next() {
        Some(op) => op.to_string(),
        None => return None,
    };
    let rest = parts.next().unwrap_or("");
    let ops: Vec<X86Operand> = rest
        .split(',')
        .map(|p| parse_operand(p.trim()))
        .filter_map(|o| o)
        .collect();
    Some(X86Instruction {
        opcode,
        operands: ops,
    })
}

fn parse_operand(s: &str) -> Option<X86Operand> {
    if s.is_empty() {
        return None;
    }
    // immediates
    if s.starts_with("0x") {
        if let Ok(v) = i64::from_str_radix(&s[2..].trim_matches(','), 16) {
            return Some(X86Operand::Imm(v));
        }
    }
    if let Ok(v) = s.trim_start_matches('#').parse::<i64>() {
        return Some(X86Operand::Imm(v));
    }

    // memory forms like [rcx + rdx*2 + 4] or [rip + .L_label] or with size prefixes like "dword ptr [rdi + 4]"
    if let (Some(lbr), Some(rbr)) = (s.find('['), s.rfind(']')) {
        let inner_raw = &s[lbr + 1..rbr];
        // normalize: turn '-' into '+-' so we can split on '+' and keep negative displacements
        let inner = inner_raw.replace('-', "+-");
        let mut base = None;
        let mut index = None;
        let mut scale: i64 = 1;
        let mut disp: i64 = 0;
        for part in inner.split('+') {
            let p = part.trim();
            if p.is_empty() {
                continue;
            }

            // handle index*scale
            if p.contains('*') {
                let mut it = p.split('*');
                let reg = it.next().unwrap().trim();
                let sc = it.next().map(|s| s.trim()).unwrap_or("1");
                let mut parsed_scale: i64 = 1;
                if sc.starts_with("0x") {
                    if let Ok(v) = i64::from_str_radix(&sc[2..], 16) {
                        parsed_scale = v;
                    }
                } else if let Ok(v) = sc.parse::<i64>() {
                    parsed_scale = v;
                }
                if reg.starts_with('r')
                    || reg.starts_with('e')
                    || reg.starts_with('a')
                    || reg.starts_with('b')
                {
                    if index.is_none() {
                        index = Some(reg.to_string());
                        scale = parsed_scale;
                    } else if base.is_none() {
                        base = Some(reg.to_string());
                    }
                } else {
                    // fallback: treat as displacement*scale (rare)
                    if let Ok(v) = reg.parse::<i64>() {
                        disp += v * parsed_scale;
                    }
                }
                continue;
            }

            if p.ends_with("ptr") {
                // ignore size qualifier
                continue;
            }
            if p.starts_with("rip") || p.starts_with("RIP") {
                base = Some("rip".to_string());
                continue;
            }
            if p.starts_with('r') || p.starts_with('e') || p.starts_with('a') || p.starts_with('b')
            {
                // crude: treat as register
                if base.is_none() {
                    base = Some(p.to_string());
                } else if index.is_none() {
                    index = Some(p.to_string());
                } else {
                    /* ignore */
                }
                continue;
            }
            if p.starts_with("-") || p.chars().next().unwrap().is_numeric() || p.starts_with("0x") {
                // displacement
                if p.starts_with("0x") {
                    if let Ok(v) = i64::from_str_radix(&p[2..], 16) {
                        disp += v;
                    }
                } else if let Ok(v) = p.parse::<i64>() {
                    disp += v;
                }
                continue;
            }
            // label or symbol: prefer not to overwrite an existing base (e.g. "rip + .L_label")
            if base.is_none() {
                base = Some(p.to_string());
            } else if index.is_none() {
                index = Some(p.to_string());
            } else {
                // ignore additional tokens
            }
        }
        return Some(X86Operand::Mem {
            base,
            index,
            scale,
            disp,
        });
    }

    // distinguish registers from bare labels/symbols. Many tokens are
    // alphanumeric; treat as register only if they resemble common x86
    // register naming (start with r/e, or vector regs like xmm/ymm/zmm).
    fn looks_like_register(tok: &str) -> bool {
        let t = tok.trim();
        // r followed by digits (r0..r15) or r + 2-3 letters (rax, rbx, r10)
        if t.starts_with('r') {
            let tail = &t[1..];
            if !tail.is_empty() && tail.chars().all(|c| c.is_digit(10)) {
                return true;
            }
            if tail.len() >= 2 && tail.len() <= 3 && tail.chars().all(|c| c.is_alphabetic()) {
                return true;
            }
        }
        // e-regs like eax, ebx, ecx, edx, esi, edi, esp, ebp
        match t {
            "eax" | "ebx" | "ecx" | "edx" | "esi" | "edi" | "esp" | "ebp" => return true,
            _ => {}
        }
        // vector regs xmm/ymm/zmm with digits (xmm0, ymm1)
        if t.starts_with("xmm") || t.starts_with("ymm") || t.starts_with("zmm") {
            let tail = &t[3..];
            return !tail.is_empty() && tail.chars().all(|c| c.is_digit(10));
        }
        // low regs like al/ax/bl/bx/cl/cx/dl/dx
        match t {
            "al" | "ax" | "bl" | "bx" | "cl" | "cx" | "dl" | "dx" => return true,
            _ => {}
        }
        false
    }

    if s.chars().all(|c| c.is_alphanumeric()) {
        if looks_like_register(s) {
            return Some(X86Operand::Reg(s.trim().to_string()));
        }
        // otherwise treat as a bare label/symbol
        return Some(X86Operand::Label(s.trim().to_string()));
    }

    // label starting with dot or underscore
    if s.starts_with('.') || s.starts_with('_') || s.starts_with("L_") {
        return Some(X86Operand::Label(s.trim().to_string()));
    }

    Some(X86Operand::Other(s.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_mov_mem() {
        let l = "mov r11d, dword ptr [rdi + 4]";
        let ins = parse_x86_line(l).unwrap();
        assert_eq!(ins.opcode, "mov");
    }

    #[test]
    fn parse_vmovdqu() {
        let l = "vmovdqu ymm0, ymmword ptr [rsi]";
        let ins = parse_x86_line(l).unwrap();
        assert_eq!(ins.opcode, "vmovdqu");
    }

    #[test]
    fn parse_mem_index_scale_and_disp() {
        let l = "mov rax, qword ptr [rcx + rdx*2 + -8]";
        let ins = parse_x86_line(l).unwrap();
        assert_eq!(ins.opcode, "mov");
        if let X86Operand::Mem {
            base,
            index,
            scale,
            disp,
        } = &ins.operands[1]
        {
            assert_eq!(base.as_ref().map(|s| s.as_str()), Some("rcx"));
            assert_eq!(index.as_ref().map(|s| s.as_str()), Some("rdx"));
            assert_eq!(*scale, 2);
            assert_eq!(*disp, -8);
        } else {
            panic!("expected mem operand");
        }
    }

    #[test]
    fn parse_rip_relative_label() {
        let l = "vmovdqu ymm0, ymmword ptr [rip + .L_table]";
        let ins = parse_x86_line(l).unwrap();
        assert_eq!(ins.opcode, "vmovdqu");
        if let X86Operand::Mem {
            base,
            index: _,
            scale,
            disp,
        } = &ins.operands[1]
        {
            assert_eq!(base.as_ref().map(|s| s.as_str()), Some("rip"));
            // scale default is 1
            assert_eq!(*scale, 1);
            // displacement absent -> 0
            assert_eq!(*disp, 0);
        } else {
            panic!("expected mem operand");
        }
    }
}
