use std::mem;

use crate::ir::BasicBlock;

/// Abstract model of a cell.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Value {
    /// Copy the value of a cell from before this basic block.
    Copy { offset: isize },
    /// A constant value.
    Const { value: u8 },
    /// A value read from the user.
    Input { id: usize },
    /// Addition of two values.
    Add { lhs: Box<Value>, rhs: Box<Value> },
    /// Multiplication of two values.
    Mul { lhs: Box<Value>, rhs: Box<Value> },
}

impl Value {
    pub(crate) fn add(&mut self, rhs: u8) {
        let lhs = match self {
            Value::Add { rhs, .. } => rhs.as_mut(),
            _ => self,
        };
        match lhs {
            Value::Const { value } => {
                *value = value.wrapping_add(rhs);
                return;
            }
            _ => {
                let lhs = mem::replace(self, Value::Copy { offset: isize::MAX });
                *self = Value::Add {
                    lhs: Box::new(lhs),
                    rhs: Box::new(Value::Const { value: rhs }),
                };
            }
        }
    }

    pub(crate) fn rebase(&mut self, bb: &BasicBlock) {
        match self {
            Value::Copy { offset } => {
                *self = bb.cell_copy(bb.offset() + *offset);
            }
            Value::Const { .. } => {}
            Value::Input { id } => *id += bb.inputs(),
            Value::Add { lhs, rhs } => {
                lhs.rebase(bb);
                rhs.rebase(bb);
                self.simplify();
            }
            Value::Mul { lhs, rhs } => {
                lhs.rebase(bb);
                rhs.rebase(bb);
                self.simplify();
            }
        }
    }

    pub(crate) fn simplify(&mut self) {
        match self {
            Value::Copy { .. } | Value::Const { .. } | Value::Input { .. } => {}
            Value::Add { lhs, rhs } => match (lhs.as_mut(), rhs.as_mut()) {
                (Value::Const { value: lhs_value }, Value::Const { value: rhs_value }) => {
                    *self = Value::Const {
                        value: lhs_value.wrapping_add(*rhs_value),
                    };
                }
                (value, Value::Const { value: 0 }) | (Value::Const { value: 0 }, value) => {
                    *self = mem::replace(value, Value::Copy { offset: isize::MAX });
                }
                (Value::Const { .. }, _) => mem::swap(lhs, rhs),
                (
                    Value::Add {
                        lhs: lhs1,
                        rhs: lhs2,
                    },
                    _,
                ) => match lhs2.as_mut() {
                    Value::Const { value } => {
                        rhs.add(*value);
                        *lhs = lhs1.clone();
                    }
                    _ => {}
                },
                _ => {}
            },
            Value::Mul { lhs, rhs } => match (lhs.as_mut(), rhs.as_mut()) {
                (Value::Const { value: lhs_value }, Value::Const { value: rhs_value }) => {
                    *self = Value::Const {
                        value: lhs_value.wrapping_mul(*rhs_value),
                    };
                }
                (value, Value::Const { value: 1 }) | (Value::Const { value: 1 }, value) => {
                    *self = mem::replace(value, Value::Copy { offset: isize::MAX });
                }
                (Value::Const { .. }, _) => mem::swap(lhs, rhs),
                _ => {}
            },
        }
    }

    /// Returns whether this cell references a cell besides itself.
    pub(crate) fn references_other(&self, offset: isize) -> bool {
        match self {
            Value::Copy { offset: offset2 } => *offset2 != offset,
            Value::Const { .. } | Value::Input { .. } => false,
            Value::Add { lhs, rhs } | Value::Mul { lhs, rhs } => {
                lhs.references_other(offset) || rhs.references_other(offset)
            }
        }
    }
}
