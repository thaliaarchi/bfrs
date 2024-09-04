use std::{collections::VecDeque, mem};

use crate::Ast;

/// Abstract model of a sub-slice of memory and the effects in a basic block.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AbstractMemory {
    /// A sub-slice of the full memory.
    memory: VecDeque<AbstractCell>,
    /// A sequence of effects in a basic block.
    effects: Vec<Effect>,
    /// The index in `memory` of the initial cell at the start of this basic
    /// block.
    start_index: usize,
    /// The offset of the current cell relative to the initial cell of this
    /// basic block.
    offset: isize,
    /// The minimum shift left that has been guarded.
    guarded_left: isize,
    /// The maximum shift right that has been guarded.
    guarded_right: isize,
    /// The number of inputs read in this basic block.
    inputs: usize,
}

/// Abstract model of a cell.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AbstractCell {
    /// Copy the value of a cell from before this basic block. Offsets are
    /// relative to the index of the cell.
    Copy { offset: isize },
    /// A constant value.
    Const { value: u8 },
    /// A value read from the user.
    Input { id: usize },
    /// Addition of two values.
    Add {
        lhs: Box<AbstractCell>,
        rhs: Box<AbstractCell>,
    },
}

/// An observable effect.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Effect {
    /// Printing a value.
    Output { value: AbstractCell },
    /// Reading from the user.
    Input { id: usize },
    /// Guarding that a shift can be performed.
    GuardShift { offset: isize },
}

impl AbstractMemory {
    /// Constructs an abstract memory with no effects.
    pub fn new() -> Self {
        AbstractMemory {
            memory: VecDeque::new(),
            effects: Vec::new(),
            start_index: 0,
            offset: 0,
            guarded_left: 0,
            guarded_right: 0,
            inputs: 0,
        }
    }

    /// Applies an operation to the abstract memory.
    pub fn apply(&mut self, op: &Ast) {
        match op {
            Ast::Right => {
                self.offset += 1;
                if self.offset > self.guarded_right {
                    self.guarded_right = self.offset;
                    self.effects.push(Effect::GuardShift {
                        offset: self.guarded_right,
                    });
                }
            }
            Ast::Left => {
                self.offset -= 1;
                if self.offset < self.guarded_left {
                    self.guarded_left = self.offset;
                    self.effects.push(Effect::GuardShift {
                        offset: self.guarded_left,
                    });
                }
            }
            Ast::Inc => self.cell_mut().add(1),
            Ast::Dec => self.cell_mut().add(255),
            Ast::Output => {
                let value = self.cell_mut().clone();
                self.effects.push(Effect::Output { value });
            }
            Ast::Input => {
                *self.cell_mut() = AbstractCell::Input { id: self.inputs };
                self.effects.push(Effect::Input { id: self.inputs });
                self.inputs += 1;
            }
            Ast::Loop(_) => unimplemented!(),
        }
    }

    fn cell_mut(&mut self) -> &mut AbstractCell {
        let index = self.start_index as isize + self.offset;
        if index < 0 {
            let n = index.unsigned_abs();
            self.start_index += n;
            self.memory.reserve(n);
            for _ in 0..n {
                self.memory.push_front(AbstractCell::Copy { offset: 0 });
            }
        } else if index as usize >= self.memory.len() {
            self.memory
                .resize(index as usize + 1, AbstractCell::Copy { offset: 0 });
        }
        &mut self.memory[(self.start_index as isize + self.offset) as usize]
    }
}

impl AbstractCell {
    fn add(&mut self, rhs: u8) {
        let lhs = match self {
            AbstractCell::Add { rhs, .. } => rhs.as_mut(),
            _ => self,
        };
        match lhs {
            AbstractCell::Const { value } => {
                *value += rhs;
                return;
            }
            _ => {
                let lhs = mem::replace(self, AbstractCell::Copy { offset: 0 });
                *self = AbstractCell::Add {
                    lhs: Box::new(lhs),
                    rhs: Box::new(AbstractCell::Const { value: rhs }),
                };
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;

    use crate::{
        ir::{AbstractCell, AbstractMemory, Effect},
        Ast,
    };

    #[test]
    fn apply_ops() {
        let mut mem = AbstractMemory::new();
        mem.apply(&Ast::Inc);
        mem.apply(&Ast::Inc);
        mem.apply(&Ast::Right);
        mem.apply(&Ast::Left);
        mem.apply(&Ast::Left);
        mem.apply(&Ast::Dec);
        mem.apply(&Ast::Right);
        mem.apply(&Ast::Output);
        mem.apply(&Ast::Input);
        mem.apply(&Ast::Right);
        mem.apply(&Ast::Right);
        assert_eq!(
            mem,
            AbstractMemory {
                memory: VecDeque::from([
                    AbstractCell::Add {
                        lhs: Box::new(AbstractCell::Copy { offset: 0 }),
                        rhs: Box::new(AbstractCell::Const { value: 255 }),
                    },
                    AbstractCell::Input { id: 0 },
                ]),
                effects: vec![
                    Effect::GuardShift { offset: 1 },
                    Effect::GuardShift { offset: -1 },
                    Effect::Output {
                        value: AbstractCell::Add {
                            lhs: Box::new(AbstractCell::Copy { offset: 0 }),
                            rhs: Box::new(AbstractCell::Const { value: 2 }),
                        },
                    },
                    Effect::Input { id: 0 },
                    Effect::GuardShift { offset: 2 },
                ],
                start_index: 1,
                offset: 2,
                guarded_left: -1,
                guarded_right: 2,
                inputs: 1,
            }
        );
    }
}
