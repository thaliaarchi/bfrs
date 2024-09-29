use crate::{
    graph::{hash_arena::ArenaRefMut, Graph, NodeId},
    memory::MemoryBuilder,
    node::{Condition, Node},
    region::{Effect, Region},
    Ast,
};

// TODO:
// - Transforming a loop with shifts to its closed form is unsound, when those
//   shifts have not already been guarded.
// - Check for guaranteed zero recursively instead of iteratively.
// - Add infinite loop condition.
// - Move guard_shift out of loops with no net shift. Peel the first iteration
//   if necessary.

impl Graph {
    pub fn lower(&self, ast: &Ast) -> NodeId {
        let Ast::Root(root) = ast else {
            panic!("expect AST root");
        };
        Node::Root {
            blocks: self.lower_blocks(root),
        }
        .insert(self)
    }

    fn lower_blocks(&self, mut ast: &[Ast]) -> Vec<NodeId> {
        let mut memory = MemoryBuilder::new();
        let mut ir = vec![];
        while let Some((inst, rest)) = ast.split_first() {
            let node = if let Ast::Loop(body) = inst {
                ast = rest;
                Node::Loop {
                    condition: Condition::WhileNonZero,
                    body: self.lower_blocks(body),
                }
            } else {
                let i = ast
                    .iter()
                    .position(|inst| matches!(inst, Ast::Loop(_)))
                    .unwrap_or(ast.len());
                let (linear_insts, rest) = ast.split_at(i);
                ast = rest;
                Node::BasicBlock(Region::from_basic_block(linear_insts, &mut memory, self))
            };
            ir.push(node.insert(self));
        }
        ir
    }

    pub fn optimize<'g>(&'g self, mut node: ArenaRefMut<'g, Node>) {
        match node.value_mut() {
            Node::Root { blocks } => {
                let first_non_loop = blocks
                    .iter()
                    .position(|&block| !matches!(*self.get(block), Node::Loop { .. }))
                    .unwrap_or(0);
                blocks.drain(..first_non_loop);
                self.optimize_blocks(blocks);
            }
            Node::BasicBlock(bb) => {
                bb.join_outputs(self);
            }
            Node::Loop { condition, body } => {
                self.optimize_blocks(body);
                if let &[block] = body.as_slice() {
                    if let Node::BasicBlock(bb) = &mut *self.get_mut(block) {
                        if bb.memory.offset() == 0 {
                            if let Some(current) = bb.memory.get_cell(0) {
                                let current_ref = self.get(current);
                                if let Node::Add(lhs, rhs) = *current_ref {
                                    let (lhs_ref, rhs_ref) = (self.get(lhs), self.get(rhs));
                                    if let (Node::Copy(0), &Node::Const(rhs)) =
                                        (&*lhs_ref, &*rhs_ref)
                                    {
                                        if let Some(iterations) = mod_inverse(rhs.wrapping_neg()) {
                                            let addend = Node::Mul(
                                                lhs,
                                                Node::Const(iterations).insert(self),
                                            )
                                            .idealize(self);
                                            if !bb.memory.iter().any(|(offset, cell)| {
                                                let cell = self.get(cell);
                                                cell.references_other(offset)
                                                    || matches!(
                                                        *cell,
                                                        Node::Input { .. } | Node::Mul(..)
                                                    )
                                            }) {
                                                if bb.effects.iter().all(|effect| {
                                                    matches!(effect, Effect::GuardShift(_))
                                                }) {
                                                    *bb.memory.get_cell_mut(0) =
                                                        Some(Node::Const(0).insert(self));
                                                    for (_, cell) in bb.memory.iter_mut() {
                                                        let cell_ref = self.get(*cell);
                                                        if let Node::Add(lhs, rhs) = *cell_ref {
                                                            *cell = Node::Add(
                                                                lhs,
                                                                Node::Mul(rhs, addend)
                                                                    .idealize(self),
                                                            )
                                                            .idealize(self);
                                                        }
                                                    }
                                                    node.replace(block);
                                                    return;
                                                }
                                            }
                                            *condition = Condition::Count(addend);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                let mut guaranteed_zero = false;
                let mut offset = 0;
                for &block in body.iter().rev() {
                    match &*self.get(block) {
                        Node::BasicBlock(bb) => {
                            if let Some(v) = bb.memory.get_cell(bb.memory.offset()) {
                                match *self.get(v) {
                                    Node::Const(0) => {
                                        guaranteed_zero = true;
                                        break;
                                    }
                                    Node::Copy(0) => {}
                                    _ => {
                                        guaranteed_zero = false;
                                        break;
                                    }
                                }
                            }
                            offset += bb.memory.offset();
                        }
                        Node::Loop { .. } => {
                            guaranteed_zero = offset == 0;
                            break;
                        }
                        _ => unreachable!(),
                    }
                }
                if guaranteed_zero {
                    *condition = Condition::IfNonZero;
                }
            }
            _ => todo!(),
        }
    }

    fn optimize_blocks(&self, blocks: &mut Vec<NodeId>) {
        blocks.dedup_by(
            |block2, block1| match (&*self.get(*block1), &*self.get(*block2)) {
                (
                    Node::Loop {
                        condition: Condition::WhileNonZero,
                        ..
                    },
                    Node::Loop {
                        condition: Condition::WhileNonZero,
                        ..
                    },
                ) => true,
                _ => false,
            },
        );
        for &block in blocks.iter() {
            self.optimize(self.get_mut(block));
        }
        blocks.dedup_by(|block2, block1| {
            match (&mut *self.get_mut(*block1), &*self.get(*block2)) {
                (Node::BasicBlock(block1), Node::BasicBlock(block2)) => {
                    block1.concat(block2, self);
                    true
                }
                _ => false,
            }
        });
    }
}

/// Computes the multiplicative inverse of a number (mod 256).
fn mod_inverse(value: u8) -> Option<u8> {
    static INVERSES: [u8; 128] = [
        /*1*/ 1, /*3*/ 171, /*5*/ 205, /*7*/ 183, /*9*/ 57,
        /*11*/ 163, /*13*/ 197, /*15*/ 239, /*17*/ 241, /*19*/ 27,
        /*21*/ 61, /*23*/ 167, /*25*/ 41, /*27*/ 19, /*29*/ 53,
        /*31*/ 223, /*33*/ 225, /*35*/ 139, /*37*/ 173, /*39*/ 151,
        /*41*/ 25, /*43*/ 131, /*45*/ 165, /*47*/ 207, /*49*/ 209,
        /*51*/ 251, /*53*/ 29, /*55*/ 135, /*57*/ 9, /*59*/ 243,
        /*61*/ 21, /*63*/ 191, /*65*/ 193, /*67*/ 107, /*69*/ 141,
        /*71*/ 119, /*73*/ 249, /*75*/ 99, /*77*/ 133, /*79*/ 175,
        /*81*/ 177, /*83*/ 219, /*85*/ 253, /*87*/ 103, /*89*/ 233,
        /*91*/ 211, /*93*/ 245, /*95*/ 159, /*97*/ 161, /*99*/ 75,
        /*101*/ 109, /*103*/ 87, /*105*/ 217, /*107*/ 67, /*109*/ 101,
        /*111*/ 143, /*113*/ 145, /*115*/ 187, /*117*/ 221, /*119*/ 71,
        /*121*/ 201, /*123*/ 179, /*125*/ 213, /*127*/ 127, /*129*/ 129,
        /*131*/ 43, /*133*/ 77, /*135*/ 55, /*137*/ 185, /*139*/ 35,
        /*141*/ 69, /*143*/ 111, /*145*/ 113, /*147*/ 155, /*149*/ 189,
        /*151*/ 39, /*153*/ 169, /*155*/ 147, /*157*/ 181, /*159*/ 95,
        /*161*/ 97, /*163*/ 11, /*165*/ 45, /*167*/ 23, /*169*/ 153,
        /*171*/ 3, /*173*/ 37, /*175*/ 79, /*177*/ 81, /*179*/ 123,
        /*181*/ 157, /*183*/ 7, /*185*/ 137, /*187*/ 115, /*189*/ 149,
        /*191*/ 63, /*193*/ 65, /*195*/ 235, /*197*/ 13, /*199*/ 247,
        /*201*/ 121, /*203*/ 227, /*205*/ 5, /*207*/ 47, /*209*/ 49,
        /*211*/ 91, /*213*/ 125, /*215*/ 231, /*217*/ 105, /*219*/ 83,
        /*221*/ 117, /*223*/ 31, /*225*/ 33, /*227*/ 203, /*229*/ 237,
        /*231*/ 215, /*233*/ 89, /*235*/ 195, /*237*/ 229, /*239*/ 15,
        /*241*/ 17, /*243*/ 59, /*245*/ 93, /*247*/ 199, /*249*/ 73,
        /*251*/ 51, /*253*/ 85, /*255*/ 255,
    ];
    if value % 2 == 1 {
        Some(INVERSES[value as usize >> 1])
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use crate::{graph::Graph, Ast};

    fn assert_node_count(src: &str, nodes: usize) {
        let g = Graph::new();
        g.lower(&Ast::parse(src.as_bytes()).unwrap());
        assert_eq!(g.len(), nodes, "{src:?} => {g:?}");
    }

    #[test]
    fn lazy_node_construction() {
        assert_node_count(">>>", 2);
        assert_node_count(">>>+", 5);
        assert_node_count(">>>-", 5);
        assert_node_count(">>>,", 3);
        assert_node_count(">>>.", 3);
        assert_node_count(">+<>++++-><-+-+>><<-+-+++", 5);
    }
}
