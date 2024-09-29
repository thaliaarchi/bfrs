use std::{
    collections::{BTreeMap, BTreeSet},
    fs::{self, File},
    io::Write,
};

use bfrs::{
    graph::{Graph, NodeId},
    Ast,
};
use glob::glob;

fn test_lower(src: &str, expect: &str) {
    let g = Graph::new();
    let ast = Ast::parse(src.as_bytes()).unwrap();
    let ir = g.lower(&ast);
    assert!(g.get(ir).compare_pretty(expect));
}

fn test_optimize(src: &str, expect: &str) {
    let g = Graph::new();
    let ast = Ast::parse(src.as_bytes()).unwrap();
    let ir = g.lower(&ast);
    g.optimize(g.get_mut(ir));
    assert!(g.get(ir).compare_pretty(expect));
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
fn inner_loops() {
    let mut inner_loops = BTreeMap::<String, (NodeId, BTreeSet<String>)>::new();
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
        each_inner_loop(&ast, &mut |loop_ast| {
            inner_loops
                .entry(format!("{loop_ast}"))
                .or_insert_with(|| {
                    let loop_ir = g.lower(&Ast::Root(vec![loop_ast.clone()]));
                    g.optimize(g.get_mut(loop_ir));
                    (loop_ir, BTreeSet::new())
                })
                .1
                .insert(relative_path.to_owned());
        });
    }
    let mut out = File::create("inner_loops.txt").unwrap();
    let mut first = true;
    for (ast, (ir, paths)) in &inner_loops {
        if !first {
            writeln!(out).unwrap();
        }
        first = false;
        writeln!(out, "{ast}").unwrap();
        write!(out, "{}", g.get(*ir)).unwrap();
        for path in paths {
            writeln!(out, "    {path}").unwrap();
        }
    }
}

fn each_inner_loop(ast: &Ast, each: &mut impl FnMut(&Ast)) -> bool {
    match ast {
        Ast::Right | Ast::Left | Ast::Inc | Ast::Dec | Ast::Output | Ast::Input => true,
        Ast::Loop(body) => {
            let mut is_inner = true;
            for ast in body {
                is_inner &= each_inner_loop(ast, each);
            }
            if is_inner {
                each(ast);
            }
            false
        }
        Ast::Root(body) => {
            for ast in body {
                each_inner_loop(ast, each);
            }
            false
        }
    }
}
