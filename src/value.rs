use std::mem;

use crate::ir::BasicBlock;

/// Abstract model of a cell.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Value {
    /// Copy the value of a cell from before this basic block.
    Copy(isize),
    /// A constant value.
    Const(u8),
    /// A value read from the user.
    Input { id: usize },
    /// Addition of two values.
    Add(Box<Value>, Box<Value>),
    /// Multiplication of two values.
    Mul(Box<Value>, Box<Value>),
}

impl Value {
    pub(crate) fn add(&mut self, rhs: u8) {
        let lhs = match self {
            Value::Add(_, rhs) => rhs.as_mut(),
            _ => self,
        };
        match lhs {
            Value::Const(value) => {
                *value = value.wrapping_add(rhs);
                return;
            }
            _ => {
                let lhs = mem::take(self);
                *self = Value::Add(Box::new(lhs), Box::new(Value::Const(rhs)));
            }
        }
    }

    pub(crate) fn rebase(&mut self, bb: &BasicBlock) {
        match self {
            Value::Copy(offset) => {
                *self = bb.cell_copy(bb.offset() + *offset);
            }
            Value::Const(_) => {}
            Value::Input { id } => *id += bb.inputs(),
            Value::Add(lhs, rhs) => {
                lhs.rebase(bb);
                rhs.rebase(bb);
                self.simplify();
            }
            Value::Mul(lhs, rhs) => {
                lhs.rebase(bb);
                rhs.rebase(bb);
                self.simplify();
            }
        }
    }

    pub(crate) fn simplify(&mut self) {
        match self {
            Value::Copy(_) | Value::Const(_) | Value::Input { .. } => {}
            Value::Add(lhs, rhs) => match (lhs.as_mut(), rhs.as_mut()) {
                (Value::Const(lhs), Value::Const(rhs)) => {
                    *self = Value::Const(lhs.wrapping_add(*rhs));
                }
                (value, Value::Const(0)) | (Value::Const(0), value) => {
                    *self = mem::take(value);
                }
                (Value::Const(_), _) => mem::swap(lhs, rhs),
                (Value::Add(lhs1, lhs2), _) => match lhs2.as_mut() {
                    Value::Const(value) => {
                        rhs.add(*value);
                        *lhs = lhs1.clone();
                    }
                    _ => {}
                },
                _ => {}
            },
            Value::Mul(lhs, rhs) => match (lhs.as_mut(), rhs.as_mut()) {
                (Value::Const(lhs), Value::Const(rhs)) => {
                    *self = Value::Const(lhs.wrapping_mul(*rhs));
                }
                (value, Value::Const(1)) | (Value::Const(1), value) => {
                    *self = mem::take(value);
                }
                (Value::Const(_), _) => mem::swap(lhs, rhs),
                _ => {}
            },
        }
    }

    /// Returns whether this cell references a cell besides itself.
    pub(crate) fn references_other(&self, offset: isize) -> bool {
        match self {
            Value::Copy(offset2) => *offset2 != offset,
            Value::Const(_) | Value::Input { .. } => false,
            Value::Add(lhs, rhs) | Value::Mul(lhs, rhs) => {
                lhs.references_other(offset) || rhs.references_other(offset)
            }
        }
    }
}

impl Default for Value {
    fn default() -> Self {
        Value::Copy(isize::MAX)
    }
}
