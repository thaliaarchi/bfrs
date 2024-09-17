use std::fmt::{self, Debug, Formatter};

use crate::{
    graph::{ByteId, Graph},
    memory::MemoryBuilder,
    node::Byte,
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

/// The root of the IR.
#[derive(Clone, PartialEq, Eq)]
pub struct Ir {
    pub blocks: Vec<Cfg>,
}

/// A control node in the IR.
#[derive(Clone, PartialEq, Eq)]
pub enum Cfg {
    /// A basic block of non-branching instructions.
    BasicBlock(Region),
    /// Loop while some condition is true.
    Loop {
        /// Loop condition.
        condition: Condition,
        /// The contained blocks.
        body: Vec<Cfg>,
    },
}

/// A loop condition.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Condition {
    /// Loop while the current cell is non-zero.
    WhileNonZero,
    /// Execute if the current cell is non-zero.
    IfNonZero,
    /// Loop a fixed number of times.
    Count(ByteId),
}

impl Ir {
    pub fn lower(ast: &[Ast], g: &mut Graph) -> Self {
        Ir {
            blocks: Ir::lower_blocks(ast, g),
        }
    }

    fn lower_blocks(mut ast: &[Ast], g: &mut Graph) -> Vec<Cfg> {
        let mut memory = MemoryBuilder::new();
        let mut ir = vec![];
        while let Some((inst, rest)) = ast.split_first() {
            if let Ast::Loop(body) = inst {
                ir.push(Cfg::Loop {
                    condition: Condition::WhileNonZero,
                    body: Ir::lower_blocks(body, g),
                });
                ast = rest;
            } else {
                let i = ast
                    .iter()
                    .position(|inst| matches!(inst, Ast::Loop(_)))
                    .unwrap_or(ast.len());
                let (linear_insts, rest) = ast.split_at(i);
                ir.push(Cfg::BasicBlock(Region::from_basic_block(
                    linear_insts,
                    &mut memory,
                    g,
                )));
                ast = rest;
            }
        }
        ir
    }

    pub fn optimize(&mut self, g: &mut Graph) {
        let first_non_loop = self
            .blocks
            .iter()
            .position(|block| !matches!(block, Cfg::Loop { .. }))
            .unwrap_or(0);
        self.blocks.drain(..first_non_loop);
        Self::optimize_blocks(&mut self.blocks, g);
    }

    fn optimize_blocks(ir: &mut Vec<Cfg>, g: &mut Graph) {
        ir.dedup_by(|block2, block1| match (block1, block2) {
            (
                Cfg::Loop {
                    condition: Condition::WhileNonZero,
                    ..
                },
                Cfg::Loop {
                    condition: Condition::WhileNonZero,
                    ..
                },
            ) => true,
            _ => false,
        });
        for block in ir.iter_mut() {
            block.optimize(g);
        }
        ir.dedup_by(|block2, block1| match (block1, block2) {
            (Cfg::BasicBlock(block1), Cfg::BasicBlock(block2)) => {
                block1.concat(block2, g);
                true
            }
            _ => false,
        });
    }
}

impl Cfg {
    /// Optimizes decrement loops and if-style loops.
    pub fn optimize(&mut self, g: &mut Graph) {
        if let Cfg::BasicBlock(bb) = self {
            bb.join_outputs(g);
        } else if let Cfg::Loop { condition, body } = self {
            Ir::optimize_blocks(body, g);
            if let [Cfg::BasicBlock(bb)] = body.as_mut_slice() {
                if bb.memory.offset() == 0 {
                    if let Some(current) = bb.memory.get_cell(0) {
                        if let Byte::Add(lhs, rhs) = g[current] {
                            if let (Byte::Copy(0), &Byte::Const(rhs)) = (&g[lhs], &g[rhs]) {
                                if let Some(iterations) = mod_inverse(rhs.wrapping_neg()) {
                                    let addend = Byte::Mul(lhs, Byte::Const(iterations).insert(g))
                                        .idealize(g);
                                    if !bb.memory.iter().any(|(offset, cell)| {
                                        cell.get(g).references_other(offset)
                                            || matches!(g[cell], Byte::Input { .. } | Byte::Mul(..))
                                    }) {
                                        if bb
                                            .effects
                                            .iter()
                                            .all(|effect| matches!(effect, Effect::GuardShift(_)))
                                        {
                                            *bb.memory.get_cell_mut(0) =
                                                Some(Byte::Const(0).insert(g));
                                            for (_, cell) in bb.memory.iter_mut() {
                                                if let Byte::Add(lhs, rhs) = g[*cell] {
                                                    *cell = Byte::Add(
                                                        lhs,
                                                        Byte::Mul(rhs, addend).idealize(g),
                                                    )
                                                    .idealize(g);
                                                }
                                            }
                                            let bb = body.drain(..).next().unwrap();
                                            *self = bb;
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
            let mut guaranteed_zero = false;
            let mut offset = 0;
            for block in body.iter().rev() {
                match block {
                    Cfg::BasicBlock(bb) => {
                        if let Some(v) = bb.memory.get_cell(bb.memory.offset()) {
                            match g[v] {
                                Byte::Const(0) => {
                                    guaranteed_zero = true;
                                    break;
                                }
                                Byte::Copy(0) => {}
                                _ => {
                                    guaranteed_zero = false;
                                    break;
                                }
                            }
                        }
                        offset += bb.memory.offset();
                    }
                    Cfg::Loop { .. } => {
                        guaranteed_zero = offset == 0;
                        break;
                    }
                }
            }
            if guaranteed_zero {
                *condition = Condition::IfNonZero;
            }
        }
    }
}

impl Debug for Ir {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "Ir ")?;
        f.debug_list().entry(&self.blocks).finish()
    }
}

impl Debug for Cfg {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Cfg::BasicBlock(bb) => Debug::fmt(bb, f),
            Cfg::Loop { condition, body } => {
                write!(f, "Loop({condition:?}) ")?;
                f.debug_list().entries(body).finish()
            }
        }
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
    use crate::{
        graph::Graph,
        ir::{Cfg, Ir},
        Ast,
    };

    fn assert_node_count(src: &str, nodes: usize) {
        let mut g = Graph::new();
        Ir::lower(&Ast::parse(src.as_bytes()).unwrap(), &mut g);
        assert_eq!(g.len(), nodes, "{src:?} => {g:?}");
    }

    #[test]
    fn lazy_node_construction() {
        assert_node_count(">>>", 0);
        assert_node_count(">>>+", 3);
        assert_node_count(">>>-", 3);
        assert_node_count(">>>,", 1);
        assert_node_count(">>>.", 1);
        assert_node_count(">+<>++++-><-+-+>><<-+-+++", 3);
    }

    #[test]
    fn concat() {
        let g = &mut Graph::new();

        let src1 = "<+>,-.>";
        let ir1 = Ir::lower(&Ast::parse(src1.as_bytes()).unwrap(), g);
        let expect1 = "
            guard_shift -1
            in0 = input
            output in0 - 1
            guard_shift 1
            @-1 = @-1 + 1
            @0 = in0 - 1
            shift 1
        ";
        assert!(ir1.compare_pretty(expect1, &g));

        let src2 = ",<-";
        let ir2 = Ir::lower(&Ast::parse(src2.as_bytes()).unwrap(), g);
        let expect2 = "
            in0 = input
            guard_shift -1
            @-1 = @-1 - 1
            @0 = in0
            shift -1
        ";
        assert!(ir2.compare_pretty(expect2, &g));

        let (mut bb1, mut bb2) = match (&*ir1.blocks, &*ir2.blocks) {
            ([Cfg::BasicBlock(bb1)], [Cfg::BasicBlock(bb2)]) => (bb1.clone(), bb2.clone()),
            _ => panic!("not single basic blocks: {ir1:?}, {ir2:?}"),
        };
        bb1.concat(&mut bb2, g);
        let expect = "
            guard_shift -1
            in0 = input
            output in0 - 1
            guard_shift 1
            in1 = input
            @-1 = @-1 + 1
            @0 = in0 - 2
            @1 = in1
        ";
        assert!(bb1.compare_pretty(expect, g));
    }
}
