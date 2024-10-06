use std::{
    collections::{BTreeMap, BTreeSet},
    fs::{self, File},
    io::Write,
};

use bfrs::{
    graph::{Graph, NodeId},
    node::{Condition, Node},
    region::Region,
    Ast,
};
use glob::glob;

fn test_lower(src: &str, expect: &str) {
    let g = Graph::new();
    let ast = Ast::parse(src.as_bytes()).unwrap();
    let root = g.lower(&ast);
    assert!(g.get(root).compare_pretty(expect));
}

fn test_optimize(src: &str, expect: &str) {
    let g = Graph::new();
    let ast = Ast::parse(src.as_bytes()).unwrap();
    let root = g.lower(&ast);
    g.optimize(root);
    assert!(g.get(root).compare_pretty(expect));
}

fn test_lower_file(src_path: &str, expect_path: &str) {
    let src = fs::read_to_string(src_path).unwrap();
    let expect = fs::read_to_string(expect_path).unwrap();
    test_lower(&src, &expect);
}

fn test_optimize_file(src_path: &str, expect_path: &str) {
    let src = fs::read_to_string(src_path).unwrap();
    let expect = fs::read_to_string(expect_path).unwrap();
    test_optimize(&src, &expect);
}

#[test]
fn collatz_lower() {
    test_lower_file(
        "tests/third_party/cristofd/collatz.b",
        "tests/third_party/cristofd/collatz.noopt.ir",
    );
}

#[test]
fn collatz_optimize() {
    test_optimize_file(
        "tests/third_party/cristofd/collatz.b",
        "tests/third_party/cristofd/collatz.ir",
    );
}

#[test]
fn wikipedia_hello_world_optimize() {
    test_optimize_file(
        "tests/third_party/wikipedia/hello_world.b",
        "tests/third_party/wikipedia/hello_world.ir",
    );
}

#[test]
fn rosettacode_hello_world_optimize() {
    test_optimize_file(
        "tests/third_party/rosettacode/hello_world.b",
        "tests/third_party/rosettacode/hello_world.ir",
    );
}

#[test]
fn closed_form_loops() {
    test_optimize("[-]", "@0 = 0");
    test_optimize(
        "[->+<]",
        "
            guard_shift 1
            @0 = 0
            @1 = @0 + @1
        ",
    );
    test_optimize(
        "[<->-]",
        "
            guard_shift -1
            @-1 = @-1 + @0 * -1
            @0 = 0
        ",
    );
    test_optimize(
        "[->+++<]",
        "
            guard_shift 1
            @0 = 0
            @1 = @0 * 3 + @1
        ",
    );
    test_optimize(
        "[->-->+++<<]",
        "
            guard_shift 1
            guard_shift 2
            @0 = 0
            @1 = @0 * -2 + @1
            @2 = @0 * 3 + @2
        ",
    );
    test_optimize(
        "[--->+>++>->--<<<<]",
        "
            guard_shift 1
            guard_shift 2
            guard_shift 3
            guard_shift 4
            @0 = 0
            @1 = @0 * -85 + @1
            @2 = @0 * 86 + @2
            @3 = @0 * 85 + @3
            @4 = @0 * -86 + @4
        ",
    );

    test_optimize(
        "[->+<][->+<]",
        "
            guard_shift 1
            @0 = 0
            @1 = @0 + @1
        ",
    );
}

#[test]
fn fixed_repetition_loops() {
    test_optimize(
        "[.-]",
        "
            repeat @0 times {
                output @0
                @0 = @0 - 1
            }
        ",
    );
    test_optimize(
        "[+++++++++++++++.>++<]",
        "
            repeat @0 * 17 times {
                output @0 + 15
                guard_shift 1
                @0 = @0 + 15
                @1 = @1 + 2
            }
        ",
    );
}

#[test]
fn sub_eq() {
    // x - x => 0
    test_optimize(
        ",>[-]>[-]<<[->+>+<<]>[->-<]",
        "
            in0 = input
            guard_shift 1
            guard_shift 2
            @0 = 0
            @1 = 0
            @2 = 0
            shift 1
        ",
    );
    // -x + x => 0
    test_optimize(
        ",>[-]>[-]<<[->+>-<<]>[->+<]",
        "
            in0 = input
            guard_shift 1
            guard_shift 2
            @0 = 0
            @1 = 0
            @2 = 0
            shift 1
        ",
    );
    // -x + x => 0
    test_optimize(
        ",[->+>-<<]>[->+<]",
        "
            in0 = input
            guard_shift 1
            guard_shift 2
            @0 = 0
            @1 = 0
            @2 = @1 + @2
            shift 1
        ",
    );
}

#[test]
fn missed_optimizations() {
    test_optimize(
        "[]",
        "
            while @0 != 0 {
            }
        ",
    );
    test_optimize(
        "[[-]]",
        "
            if @0 != 0 {
                @0 = 0
            }
        ",
    );
    test_optimize(
        "><[[->+<]]",
        "
            {
                guard_shift 1
            }
            if @0 != 0 {
                guard_shift 1
                @0 = 0
                @1 = @0 + @1
            }
        ",
    );
    test_optimize(
        "[--]",
        "
            while @0 != 0 {
                @0 = @0 - 2
            }
        ",
    );
    // x - x => 0
    test_optimize(
        ",[->+>+<<]>[->-<]",
        "
            in0 = input
            guard_shift 1
            guard_shift 2
            @0 = 0
            @1 = 0
            @2 = (@1 + in0) * -1 + @2 + in0
            shift 1
        ",
    );
    test_optimize(
        "
            ,>[-]>[-]>[-]>[-]>[-]>[-]<<<<<<
            [>[-]++++[>++>+++>+++>+<<<<-]>+>+>->>+<<<<<<-]
        ",
        "
            in0 = input
            guard_shift 1
            guard_shift 2
            guard_shift 3
            guard_shift 4
            guard_shift 5
            guard_shift 6
            @0 = 0
            @1 = 0
            @2 = in0 * 9
            @3 = in0 * 13
            @4 = in0 * 11
            @5 = in0 * 4
            @6 = in0
        ",
    );
    test_optimize(
        "
            ,>[-]>[-]>[-]>[-]>[-]>[-]<<<<<<
            [>++++[>++>+++>+++>+<<<<-]>+>+>->>+<<<<<<-]
        ",
        "
            {
                in0 = input
                guard_shift 1
                guard_shift 2
                guard_shift 3
                guard_shift 4
                guard_shift 5
                guard_shift 6
                @0 = in0
                @1 = 0
                @2 = 0
                @3 = 0
                @4 = 0
                @5 = 0
                @6 = 0
            }
            repeat @0 times {
                guard_shift 1
                guard_shift 2
                guard_shift 3
                guard_shift 4
                guard_shift 5
                guard_shift 6
                @0 = @0 - 1
                @1 = 0
                @2 = (@1 + 4) * 2 + @2 + 1
                @3 = (@1 + 4) * 3 + @3 + 1
                @4 = (@1 + 4) * 3 + @4 - 1
                @5 = @1 + @5 + 4
                @6 = @6 + 1
            }
        ",
    );
    test_optimize(
        "
            ,>[-]>[-]>[-]>[-]>[-]>[-]<<<<<<
            [>++++[>++>+++>+++>+<<<<-]>+>+>->>+[<]<-]
        ",
        "
            {
                in0 = input
                guard_shift 1
                guard_shift 2
                guard_shift 3
                guard_shift 4
                guard_shift 5
                guard_shift 6
                @0 = in0
                @1 = 0
                @2 = 0
                @3 = 0
                @4 = 0
                @5 = 0
                @6 = 0
            }
            while @0 != 0 {
                {
                    guard_shift 1
                    guard_shift 2
                    guard_shift 3
                    guard_shift 4
                    guard_shift 5
                    guard_shift 6
                    @1 = 0
                    @2 = (@1 + 4) * 2 + @2 + 1
                    @3 = (@1 + 4) * 3 + @3 + 1
                    @4 = (@1 + 4) * 3 + @4 - 1
                    @5 = @1 + @5 + 4
                    @6 = @6 + 1
                    shift 6
                }
                while @0 != 0 {
                    guard_shift -1
                    shift -1
                }
                {
                    guard_shift -1
                    @-1 = @-1 - 1
                    shift -1
                }
            }
        ",
    );
}

#[test]
#[ignore = "generates a report"]
fn report_inner_loops() {
    let mut inner_loops = BTreeMap::<String, InnerLoopStats>::new();
    let g = Graph::new();
    for path in glob("tests/third_party/**/*.b")
        .unwrap()
        .chain(glob("tests/third_party/**/*.bf").unwrap())
    {
        let path = path.unwrap();
        let relative_path = path
            .to_str()
            .unwrap()
            .strip_prefix("tests/third_party/")
            .unwrap();
        let src = fs::read(&path).unwrap();
        let ast = Ast::parse(&src).unwrap();
        each_inner_loop(&ast, true, &mut |loop_ast| {
            inner_loops
                .entry(format!("{loop_ast}"))
                .or_insert_with(|| {
                    let loop_ir = g.lower(&Ast::Root(vec![loop_ast.clone()]));
                    g.combine_guards(loop_ir);
                    let region = get_loop_region(&g, loop_ir, loop_ast);
                    let unoptimized = g.get(loop_ir).to_string();
                    g.optimize(loop_ir);
                    let optimized = g.get(loop_ir).to_string();
                    InnerLoopStats {
                        region,
                        unoptimized,
                        optimized,
                        paths: BTreeSet::new(),
                    }
                })
                .paths
                .insert(relative_path.to_owned());
        });
    }

    let (balanced_loops, unbalanced_loops): (BTreeMap<_, _>, _) = inner_loops
        .iter()
        .partition(|(_, stats)| stats.is_balanced());

    let mut out = File::create("inner_loops.md").unwrap();
    writeln!(out, "# Inner loops\n").unwrap();
    writeln!(out, "## Balanced loops").unwrap();
    for (ast, inner_loop) in &balanced_loops {
        writeln!(out).unwrap();
        inner_loop.print(&mut out, ast);
    }
    writeln!(out, "\n\n## Unbalanced loops").unwrap();
    for (ast, inner_loop) in &unbalanced_loops {
        writeln!(out).unwrap();
        inner_loop.print(&mut out, ast);
    }
}

struct InnerLoopStats {
    region: Option<Region>,
    unoptimized: String,
    optimized: String,
    paths: BTreeSet<String>,
}

fn each_inner_loop(ast: &Ast, after_zero: bool, each: &mut impl FnMut(&Ast)) -> bool {
    match ast {
        Ast::Right | Ast::Left | Ast::Inc | Ast::Dec | Ast::Output | Ast::Input => true,
        Ast::Loop(body) | Ast::Root(body) => {
            let mut is_inner_loop = true;
            let mut child_after_zero = after_zero;
            for ast in body {
                is_inner_loop &= each_inner_loop(ast, child_after_zero, each);
                // Overly simplistic dead code elimination, intended only for
                // comment loops.
                child_after_zero =
                    matches!(ast, Ast::Loop(_)) || child_after_zero && ast == &Ast::Output;
            }
            if is_inner_loop && !after_zero && matches!(ast, Ast::Loop(_)) {
                each(ast);
            }
            false
        }
    }
}

fn get_loop_region(g: &Graph, root: NodeId, ast: &Ast) -> Option<Region> {
    let Node::Root { blocks } = &**g.get(root) else {
        panic!("not root: {ast}");
    };
    let &[block] = blocks.as_slice() else {
        panic!("not one block: {ast}");
    };
    let Node::Loop {
        condition: Condition::WhileNonZero,
        body,
    } = &**g.get(block)
    else {
        panic!("not a loop: {ast}");
    };
    if body.is_empty() {
        return None;
    }
    let &[block] = body.as_slice() else {
        panic!("not a basic block in a loop: {ast}");
    };
    let Node::BasicBlock(region) = &**g.get(block) else {
        panic!("not a basic block in a loop: {ast}");
    };
    Some(region.clone())
}

impl InnerLoopStats {
    fn print(&self, w: &mut dyn Write, ast: &str) {
        writeln!(w, "```brainfuck\n{ast}\n```").unwrap();
        writeln!(w, "Unoptimized:\n```ir\n{}```", self.unoptimized).unwrap();
        if self.unoptimized != self.optimized {
            writeln!(w, "Optimized:\n```ir\n{}```", self.optimized).unwrap();
        }
        for path in &self.paths {
            writeln!(w, "- {path}").unwrap();
        }
        writeln!(w, "\n---").unwrap();
    }

    fn is_balanced(&self) -> bool {
        !self
            .region
            .as_ref()
            .is_some_and(|region| region.memory.offset() != 0)
    }
}
