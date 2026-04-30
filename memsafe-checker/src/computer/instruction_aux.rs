use crate::computer::*;

impl<'ctx> ARMCORTEXA<'ctx> {
    pub fn arithmetic(
        &mut self,
        op_string: &str,
        op: impl Fn(i64, i64) -> i64,
        operands: Vec<Operand>,
    ) {
        let mut reg_iter = operands.iter();

        let reg0 = reg_iter.next().expect("Need destination register");
        let reg1 = reg_iter.next().expect("Need first source register");
        let reg2 = reg_iter.next().expect("Need second source register");

        let r1 = self.get_register(reg1);
        let mut r2 = self.get_register(reg2);

        if let Some(Operand::Bitwise(op, num)) = &reg_iter.next() {
            let op_str = match op {
                crate::instruction_parser::ShiftType::Lsl => "lsl",
                crate::instruction_parser::ShiftType::Lsr => "lsr",
                crate::instruction_parser::ShiftType::Asr => "asr",
                crate::instruction_parser::ShiftType::Ror => "ror",
                crate::instruction_parser::ShiftType::Uxtw => "uxtw",
                crate::instruction_parser::ShiftType::Uxtb => "uxtb",
                crate::instruction_parser::ShiftType::Uxth => "uxth",
                crate::instruction_parser::ShiftType::Other(s) => s.as_str(),
            };
            r2 = shift_imm(op_str.to_string(), r2.clone(), *num);
        }

        if r1.kind == r2.kind {
            match r1.kind {
                RegisterKind::RegisterBase => {
                    let base = match (r1.clone().base, r2.clone().base) {
                        (Some(reg1base), Some(reg2base)) => {
                            Some(generate_expression(op_string, reg1base, reg2base))
                        }
                        (Some(reg1base), None) => Some(reg1base),
                        (None, Some(reg2base)) => Some(reg2base),
                        (None, None) => None,
                    };
                    self.set_register(
                        reg0,
                        RegisterKind::RegisterBase,
                        base,
                        op(r1.offset, r2.offset),
                    )
                }
                RegisterKind::Number => {
                    // abstract numbers, value doesn't matter
                    self.set_register(reg0, RegisterKind::Number, None, 0)
                }
                RegisterKind::Immediate => self.set_register(
                    reg0,
                    RegisterKind::Immediate,
                    None,
                    op(r1.offset, r2.offset),
                ),
            }
        } else if r1.kind == RegisterKind::Immediate {
            self.set_register(
                reg0,
                r2.kind.clone(),
                r2.base.clone(),
                op(r1.offset, r2.offset),
            );
        } else if r2.kind == RegisterKind::Immediate {
            self.set_register(
                reg0,
                r1.kind.clone(),
                r1.base.clone(),
                op(r1.offset, r2.offset),
            );
        } else if r1.kind == RegisterKind::Number || r2.kind == RegisterKind::Number {
            // abstract numbers, value doesn't matter
            self.set_register(reg0, RegisterKind::Number, None, 0)
        } else if r1.kind == RegisterKind::RegisterBase || r2.kind == RegisterKind::RegisterBase {
            let base = match (r1.clone().base, r2.clone().base) {
                (Some(reg1base), Some(reg2base)) => {
                    Some(generate_expression(op_string, reg1base, reg2base))
                }
                (Some(reg1base), None) => Some(reg1base),
                (None, Some(reg2base)) => Some(reg2base),
                (None, None) => None,
            };
            self.set_register(
                reg0,
                RegisterKind::RegisterBase,
                base,
                op(r1.offset, r2.offset),
            )
        } else {
            // Debug print for unexpected arithmetic cases; use log::debug so it
            // is gated by the log level instead of printing to stdout.
            log::debug!("arithmetic: op={:?} r1={:?} r2={:?}", op_string, r1, r2);
            log::error!("Cannot perform arithmetic on these two registers")
        }
    }

    pub fn shift_reg(&mut self, reg1: &Operand, reg2: &Operand, reg3: &Operand) {
        let r2 = self.get_register(reg2);

        let shifted_reg = match reg3 {
            Operand::Bitwise(op, shift) => shift_imm(op.to_string(), r2, *shift),
            Operand::Immediate(shift) => shift_imm("lsl".to_string(), r2, *shift),
            _ => {
                log::error!("Cannot shift with this operand: {:?}", reg3);
                return;
            }
        };

        self.set_register(reg1, shifted_reg.kind, shifted_reg.base, shifted_reg.offset);
    }

    pub fn cmp(&mut self, reg1: &Operand, reg2: &Operand) {
        let r1 = self.get_register(reg1);
        let r2 = self.get_register(reg2);

        if r1 == r2 {
            self.neg = Some(FlagValue::Real(false));
            self.zero = Some(FlagValue::Real(true));
            self.carry = Some(FlagValue::Real(false));
            self.overflow = Some(FlagValue::Real(false));
            return;
        }

        if r1.kind == r2.kind {
            match r1.kind {
                RegisterKind::RegisterBase => {
                    if r1.base.eq(&r2.base) {
                        self.neg = if r1.offset < r2.offset {
                            Some(FlagValue::Real(true))
                        } else {
                            Some(FlagValue::Real(false))
                        };
                        self.zero = if r1.offset == r2.offset {
                            Some(FlagValue::Real(true))
                        } else {
                            Some(FlagValue::Real(false))
                        };
                        // signed vs signed distinction, maybe make offset generic to handle both?
                        self.carry = if r2.offset > r1.offset && r1.offset - r2.offset > 0 {
                            Some(FlagValue::Real(true))
                        } else {
                            Some(FlagValue::Real(false))
                        };
                        self.overflow = if r2.offset > r1.offset && r1.offset - r2.offset > 0 {
                            Some(FlagValue::Real(true))
                        } else {
                            Some(FlagValue::Real(false))
                        };
                    } else {
                        let expression = AbstractExpression::Expression(
                            "-".to_string(),
                            Box::new(AbstractExpression::Register(Box::new(r1))),
                            Box::new(AbstractExpression::Register(Box::new(r2))),
                        );
                        self.neg = Some(FlagValue::Abstract(AbstractComparison::new(
                            "<",
                            expression.clone(),
                            AbstractExpression::Immediate(0),
                        )));
                        self.zero = Some(FlagValue::Abstract(AbstractComparison::new(
                            "==",
                            expression.clone(),
                            AbstractExpression::Immediate(0),
                        )));
                        // FIX carry + overflow
                        self.carry = Some(FlagValue::Abstract(AbstractComparison::new(
                            "<",
                            expression.clone(),
                            AbstractExpression::Immediate(i64::MIN),
                        )));
                        self.overflow = Some(FlagValue::Abstract(AbstractComparison::new(
                            "<",
                            expression,
                            AbstractExpression::Immediate(i64::MIN),
                        )));
                    }
                }
                RegisterKind::Number => {
                    log::error!("Cannot compare these two registers")
                }
                RegisterKind::Immediate => {
                    self.neg = if r1.offset < r2.offset {
                        Some(FlagValue::Real(true))
                    } else {
                        Some(FlagValue::Real(false))
                    };
                    self.zero = if r1.offset == r2.offset {
                        Some(FlagValue::Real(true))
                    } else {
                        Some(FlagValue::Real(false))
                    };
                    // signed vs signed distinction, maybe make offset generic to handle both?
                    self.carry = if r2.offset > r1.offset && r1.offset - r2.offset > 0 {
                        Some(FlagValue::Real(true))
                    } else {
                        Some(FlagValue::Real(false))
                    };
                    self.overflow = if r2.offset > r1.offset && r1.offset - r2.offset > 0 {
                        Some(FlagValue::Real(true))
                    } else {
                        Some(FlagValue::Real(false))
                    };
                }
            }
        } else if r1.kind == RegisterKind::RegisterBase || r2.kind == RegisterKind::RegisterBase {
            let expression = AbstractExpression::Expression(
                "-".to_string(),
                Box::new(AbstractExpression::Register(Box::new(r1))),
                Box::new(AbstractExpression::Register(Box::new(r2))),
            );
            self.neg = Some(FlagValue::Abstract(AbstractComparison::new(
                "<",
                expression.clone(),
                AbstractExpression::Immediate(0),
            )));
            self.zero = Some(FlagValue::Abstract(AbstractComparison::new(
                "==",
                expression.clone(),
                AbstractExpression::Immediate(0),
            )));
            // FIX carry + overflow
            self.carry = Some(FlagValue::Abstract(AbstractComparison::new(
                "<",
                expression.clone(),
                AbstractExpression::Immediate(i64::MIN),
            )));
            self.overflow = Some(FlagValue::Abstract(AbstractComparison::new(
                "<",
                expression,
                AbstractExpression::Immediate(i64::MIN),
            )));
        }
    }

    pub fn cmn(&mut self, reg1: &Operand, reg2: &Operand) {
        let r1 = self.get_register(reg1);
        let r2 = self.get_register(reg2);

        if r1 == r2 {
            self.neg = Some(FlagValue::Real(false));
            self.zero = Some(FlagValue::Real(true));
            self.carry = Some(FlagValue::Real(false));
            self.overflow = Some(FlagValue::Real(false));

            return;
        }

        if r1.kind == r2.kind {
            match r1.kind {
                RegisterKind::RegisterBase => {
                    if r1.base.eq(&r2.base) {
                        self.neg = if r1.offset + r2.offset < 0 {
                            Some(FlagValue::Real(true))
                        } else {
                            Some(FlagValue::Real(false))
                        };
                        self.zero = Some(FlagValue::Real(r1.offset + r2.offset == 0));
                        let sum_i128 = (r2.offset as i128) + (r1.offset as i128);
                        let overflowed = sum_i128 > (i64::MAX as i128);
                        self.carry = Some(FlagValue::Real(overflowed));
                        self.overflow = Some(FlagValue::Real(overflowed));
                    } else {
                        let expression = AbstractExpression::Expression(
                            "+".to_string(),
                            Box::new(AbstractExpression::Register(Box::new(r1))),
                            Box::new(AbstractExpression::Register(Box::new(r2))),
                        );
                        self.neg = Some(FlagValue::Abstract(AbstractComparison::new(
                            "<",
                            expression.clone(),
                            AbstractExpression::Immediate(0),
                        )));
                        self.zero = Some(FlagValue::Abstract(AbstractComparison::new(
                            "==",
                            expression.clone(),
                            AbstractExpression::Immediate(0),
                        )));
                        // FIX carry + overflow
                        self.carry = Some(FlagValue::Abstract(AbstractComparison::new(
                            ">",
                            expression.clone(),
                            AbstractExpression::Immediate(i64::MAX),
                        )));
                        self.overflow = Some(FlagValue::Abstract(AbstractComparison::new(
                            ">",
                            expression,
                            AbstractExpression::Immediate(i64::MAX),
                        )));
                    }
                }
                RegisterKind::Number => {
                    log::error!("Cannot compare these two registers")
                }
                RegisterKind::Immediate => {
                    self.neg = if r1.offset + r2.offset < 0 {
                        Some(FlagValue::Real(true))
                    } else {
                        Some(FlagValue::Real(false))
                    };
                    self.zero = Some(FlagValue::Real(r1.offset + r2.offset == 0));
                    // signed vs signed distinction, maybe make offset generic to handle both?
                    let sum_i128 = (r2.offset as i128) + (r1.offset as i128);
                    let overflowed = sum_i128 > (i64::MAX as i128);
                    self.carry = Some(FlagValue::Real(overflowed));
                    self.overflow = Some(FlagValue::Real(overflowed));
                }
            }
        } else if r1.kind == RegisterKind::RegisterBase || r2.kind == RegisterKind::RegisterBase {
            let expression = AbstractExpression::Expression(
                "+".to_string(),
                Box::new(AbstractExpression::Register(Box::new(r1))),
                Box::new(AbstractExpression::Register(Box::new(r2))),
            );
            self.neg = Some(FlagValue::Abstract(AbstractComparison::new(
                "<",
                expression.clone(),
                AbstractExpression::Immediate(0),
            )));
            self.zero = Some(FlagValue::Abstract(AbstractComparison::new(
                "==",
                expression.clone(),
                AbstractExpression::Immediate(0),
            )));
            // FIX carry + overflow
            self.carry = Some(FlagValue::Abstract(AbstractComparison::new(
                ">",
                expression.clone(),
                AbstractExpression::Immediate(i64::MAX),
            )));
            self.overflow = Some(FlagValue::Abstract(AbstractComparison::new(
                ">",
                expression,
                AbstractExpression::Immediate(i64::MAX),
            )));
        }
    }
}

pub fn shift_imm(op: String, register: RegisterValue, shift: i64) -> RegisterValue {
    match op.as_str() {
        "lsl" => {
            let new_offset = register.offset << shift;
            let base = Some(generate_expression(
                &op,
                register.base.unwrap_or(AbstractExpression::Empty),
                AbstractExpression::Immediate(shift),
            ));
            let mut rv = RegisterValue::new(register.kind, base, new_offset);
            rv.known_mask = register.known_mask;
            rv.known_value = register.known_value;
            rv
        }
        "lsr" => {
            let new_offset = register.offset << shift;
            let base = Some(generate_expression(
                &op,
                register.base.unwrap_or(AbstractExpression::Empty),
                AbstractExpression::Immediate(shift),
            ));
            let mut rv = RegisterValue::new(register.kind, base, new_offset);
            rv.known_mask = register.known_mask;
            rv.known_value = register.known_value;
            rv
        }
        "ror" => {
            let new_offset = register.offset >> shift;
            let base = Some(generate_expression(
                &op,
                register.base.unwrap_or(AbstractExpression::Empty),
                AbstractExpression::Immediate(shift),
            ));
            let mut rv = RegisterValue::new(register.kind, base, new_offset);
            rv.known_mask = register.known_mask;
            rv.known_value = register.known_value;
            rv
        }
        "" => {
            let new_offset = register.offset + shift;
            let mut rv = RegisterValue::new(register.kind, register.base, new_offset);
            rv.known_mask = register.known_mask;
            rv.known_value = register.known_value;
            rv
        }
        "uxtw" => {
            // Unsigned extend with shift-left by 0 for now (placeholder).
            // Precise semantics for width extension are not required for
            // current conservative checks; preserve the existing value.
            let mut rv = RegisterValue::new(register.kind, register.base, register.offset);
            rv.known_mask = register.known_mask;
            rv.known_value = register.known_value;
            rv
        }
        _ => todo!("{}", op),
    }
}
