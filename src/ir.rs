use std::{collections::VecDeque, mem};

use crate::Ast;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Ir {
    /// A basic block of non-branching instructions.
    BasicBlock {
        /// The effects of this basic block.
        memory: AbstractMemory,
    },
    /// A loop.
    Loop {
        /// The contained blocks.
        body: Vec<Ir>,
    },
}

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
    /// Copy the value of a cell from before this basic block.
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

impl Ir {
    pub fn lower(ast: &[Ast]) -> Vec<Self> {
        let mut ir = vec![];
        for inst in ast {
            if let Ast::Loop(body) = inst {
                ir.push(Ir::Loop {
                    body: Ir::lower(body),
                });
            } else {
                if !matches!(ir.last(), Some(Ir::BasicBlock { .. })) {
                    ir.push(Ir::BasicBlock {
                        memory: AbstractMemory::new(),
                    });
                }
                let Some(Ir::BasicBlock { memory }) = ir.last_mut() else {
                    unreachable!();
                };
                memory.apply(inst);
            }
        }
        ir
    }
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
            Ast::Loop(_) => panic!("loops must be lowered separately"),
        }
    }

    fn cell_mut(&mut self) -> &mut AbstractCell {
        let index = self.start_index as isize + self.offset;
        if index < 0 {
            let n = index.unsigned_abs();
            self.memory.reserve(n);
            let offset = self.offset - self.start_index as isize;
            for i in 0..n {
                self.memory.push_front(AbstractCell::Copy {
                    offset: offset - i as isize,
                });
            }
            self.start_index += n;
        } else if index as usize >= self.memory.len() {
            let n = index as usize - self.memory.len() + 1;
            self.memory.reserve(n);
            let offset = self.memory.len() - self.start_index;
            for i in 0..n {
                self.memory.push_back(AbstractCell::Copy {
                    offset: (offset + i) as isize,
                });
            }
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
                *value = value.wrapping_add(rhs);
                return;
            }
            _ => {
                let lhs = mem::replace(self, AbstractCell::Copy { offset: isize::MAX });
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
        ir::{AbstractCell, AbstractMemory, Effect, Ir},
        Ast,
    };

    #[test]
    fn apply_ops() {
        let mut memory = AbstractMemory::new();
        memory.apply(&Ast::Inc);
        memory.apply(&Ast::Inc);
        memory.apply(&Ast::Right);
        memory.apply(&Ast::Left);
        memory.apply(&Ast::Left);
        memory.apply(&Ast::Dec);
        memory.apply(&Ast::Right);
        memory.apply(&Ast::Output);
        memory.apply(&Ast::Input);
        memory.apply(&Ast::Right);
        memory.apply(&Ast::Right);
        let expected = AbstractMemory {
            memory: VecDeque::from([
                AbstractCell::Add {
                    lhs: Box::new(AbstractCell::Copy { offset: -1 }),
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
        };
        assert_eq!(memory, expected);
    }

    #[test]
    fn lower() {
        // Excerpt from https://www.brainfuck.org/collatz.b
        let src = b"[-[<->-]+[<<<<]]<[>+<-]";
        let ast = Ast::parse(src).unwrap();
        let ir = Ir::lower(&ast);
        let expected = vec![
            Ir::Loop {
                body: vec![
                    Ir::BasicBlock {
                        memory: AbstractMemory {
                            memory: VecDeque::from([AbstractCell::Add {
                                lhs: Box::new(AbstractCell::Copy { offset: 0 }),
                                rhs: Box::new(AbstractCell::Const { value: 255 }),
                            }]),
                            effects: vec![],
                            start_index: 0,
                            offset: 0,
                            guarded_left: 0,
                            guarded_right: 0,
                            inputs: 0,
                        },
                    },
                    Ir::Loop {
                        body: vec![Ir::BasicBlock {
                            memory: AbstractMemory {
                                memory: VecDeque::from([
                                    AbstractCell::Add {
                                        lhs: Box::new(AbstractCell::Copy { offset: -1 }),
                                        rhs: Box::new(AbstractCell::Const { value: 255 }),
                                    },
                                    AbstractCell::Add {
                                        lhs: Box::new(AbstractCell::Copy { offset: 0 }),
                                        rhs: Box::new(AbstractCell::Const { value: 255 }),
                                    },
                                ]),
                                effects: vec![Effect::GuardShift { offset: -1 }],
                                start_index: 1,
                                offset: 0,
                                guarded_left: -1,
                                guarded_right: 0,
                                inputs: 0,
                            },
                        }],
                    },
                    Ir::BasicBlock {
                        memory: AbstractMemory {
                            memory: VecDeque::from([AbstractCell::Add {
                                lhs: Box::new(AbstractCell::Copy { offset: 0 }),
                                rhs: Box::new(AbstractCell::Const { value: 1 }),
                            }]),
                            effects: vec![],
                            start_index: 0,
                            offset: 0,
                            guarded_left: 0,
                            guarded_right: 0,
                            inputs: 0,
                        },
                    },
                    Ir::Loop {
                        body: vec![Ir::BasicBlock {
                            memory: AbstractMemory {
                                memory: VecDeque::from([]),
                                effects: vec![
                                    Effect::GuardShift { offset: -1 },
                                    Effect::GuardShift { offset: -2 },
                                    Effect::GuardShift { offset: -3 },
                                    Effect::GuardShift { offset: -4 },
                                ],
                                start_index: 0,
                                offset: -4,
                                guarded_left: -4,
                                guarded_right: 0,
                                inputs: 0,
                            },
                        }],
                    },
                ],
            },
            Ir::BasicBlock {
                memory: AbstractMemory {
                    memory: VecDeque::from([]),
                    effects: vec![Effect::GuardShift { offset: -1 }],
                    start_index: 0,
                    offset: -1,
                    guarded_left: -1,
                    guarded_right: 0,
                    inputs: 0,
                },
            },
            Ir::Loop {
                body: vec![Ir::BasicBlock {
                    memory: AbstractMemory {
                        memory: VecDeque::from([
                            AbstractCell::Add {
                                lhs: Box::new(AbstractCell::Copy { offset: 0 }),
                                rhs: Box::new(AbstractCell::Const { value: 255 }),
                            },
                            AbstractCell::Add {
                                lhs: Box::new(AbstractCell::Copy { offset: 1 }),
                                rhs: Box::new(AbstractCell::Const { value: 1 }),
                            },
                        ]),
                        effects: vec![Effect::GuardShift { offset: 1 }],
                        start_index: 0,
                        offset: 0,
                        guarded_left: 0,
                        guarded_right: 1,
                        inputs: 0,
                    },
                }],
            },
        ];
        assert_eq!(ir, expected);
    }
}
