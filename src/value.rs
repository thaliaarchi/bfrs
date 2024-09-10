use std::{
    fmt::{self, Debug, Formatter},
    mem,
};

use crate::ir::BasicBlock;

/// Abstract model of a cell.
#[derive(Clone, PartialEq, Eq)]
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
    pub fn add(mut lhs: Box<Self>, mut rhs: Box<Self>) -> Self {
        if let Value::Const(_) = lhs.as_ref() {
            mem::swap(&mut lhs, &mut rhs);
        }
        match (lhs.as_ref(), rhs.as_ref()) {
            (&Value::Const(lhs), &Value::Const(rhs)) => Value::Const(lhs.wrapping_add(rhs)),
            (_, Value::Const(0)) => *lhs,
            (_, _) if lhs == rhs => {
                *rhs = Value::Const(2);
                Value::mul(lhs, rhs)
            }
            (Value::Add(..), _) => {
                let Value::Add(a, b) = mem::take(lhs.as_mut()) else {
                    unreachable!();
                };
                match (b.as_ref(), rhs.as_ref()) {
                    (&Value::Const(b), &Value::Const(c)) => {
                        *rhs = Value::Const(b.wrapping_add(c));
                        Value::add(a, rhs)
                    }
                    (&Value::Const(_), _) => {
                        *lhs = Value::add(a, rhs);
                        Value::add(lhs, b)
                    }
                    _ => {
                        *lhs = Value::Add(a, b);
                        Value::Add(lhs, rhs)
                    }
                }
            }
            (_, Value::Add(..)) => {
                let Value::Add(b, c) = mem::take(rhs.as_mut()) else {
                    unreachable!();
                };
                *rhs = Value::add(lhs, b);
                Value::add(rhs, c)
            }
            _ => Value::Add(lhs, rhs),
        }
    }

    pub fn add_const(&mut self, value: u8) {
        let lhs = match self {
            Value::Add(_, rhs) => rhs.as_mut(),
            _ => self,
        };
        match lhs {
            Value::Const(lhs) => *lhs = lhs.wrapping_add(value),
            _ => {
                let lhs = mem::take(self);
                *self = Value::add(Box::new(lhs), Box::new(Value::Const(value)));
            }
        }
    }

    pub fn mul(mut lhs: Box<Self>, mut rhs: Box<Self>) -> Self {
        if let Value::Const(_) = lhs.as_ref() {
            mem::swap(&mut lhs, &mut rhs);
        }
        match (lhs.as_ref(), rhs.as_ref()) {
            (&Value::Const(lhs), &Value::Const(rhs)) => Value::Const(lhs.wrapping_mul(rhs)),
            (_, Value::Const(1)) => *lhs,
            (Value::Mul(..), _) => {
                let Value::Mul(a, b) = mem::take(lhs.as_mut()) else {
                    unreachable!();
                };
                match (b.as_ref(), rhs.as_ref()) {
                    (&Value::Const(b), &Value::Const(c)) => {
                        *rhs = Value::Const(b.wrapping_mul(c));
                        Value::mul(a, rhs)
                    }
                    (&Value::Const(_), _) => {
                        *lhs = Value::mul(a, rhs);
                        Value::mul(lhs, b)
                    }
                    _ => {
                        *lhs = Value::Mul(a, b);
                        Value::Mul(lhs, rhs)
                    }
                }
            }
            (_, Value::Mul(..)) => {
                let Value::Mul(b, c) = mem::take(rhs.as_mut()) else {
                    unreachable!();
                };
                *rhs = Value::mul(lhs, b);
                Value::mul(rhs, c)
            }
            _ => Value::Mul(lhs, rhs),
        }
    }

    pub(crate) fn rebase(self, bb: &BasicBlock) -> Self {
        match self {
            Value::Copy(offset) => bb.cell_copy(bb.offset() + offset),
            Value::Const(c) => Value::Const(c),
            Value::Input { id } => Value::Input {
                id: id + bb.inputs(),
            },
            Value::Add(mut lhs, mut rhs) => {
                *lhs = lhs.rebase(bb);
                *rhs = rhs.rebase(bb);
                Value::add(lhs, rhs)
            }
            Value::Mul(mut lhs, mut rhs) => {
                *lhs = lhs.rebase(bb);
                *rhs = rhs.rebase(bb);
                Value::mul(lhs, rhs)
            }
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

impl Debug for Value {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        fn group(f: &mut Formatter<'_>, v: &Value, grouped: bool) -> fmt::Result {
            if grouped {
                write!(f, "({v:?})")
            } else {
                write!(f, "{v:?}")
            }
        }
        match self {
            Value::Copy(offset) => write!(f, "%{offset}"),
            Value::Const(value) => write!(f, "{value}"),
            Value::Input { id } => write!(f, "in{id}"),
            Value::Add(lhs, rhs) => {
                write!(f, "{lhs:?} + ")?;
                group(f, rhs, matches!(**rhs, Value::Add(..)))
            }
            Value::Mul(lhs, rhs) => {
                group(f, lhs, matches!(**lhs, Value::Add(..)))?;
                write!(f, " * ")?;
                group(f, rhs, matches!(**rhs, Value::Add(..) | Value::Mul(..)))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::Value;

    #[test]
    fn idealize_add() {
        let add = Value::add(
            Box::new(Value::Add(
                Box::new(Value::Copy(0)),
                Box::new(Value::Const(1)),
            )),
            Box::new(Value::Add(
                Box::new(Value::Copy(2)),
                Box::new(Value::Const(3)),
            )),
        );
        let expected = Value::Add(
            Box::new(Value::Add(
                Box::new(Value::Copy(0)),
                Box::new(Value::Copy(2)),
            )),
            Box::new(Value::Const(4)),
        );
        assert_eq!(add, expected);
    }
}
