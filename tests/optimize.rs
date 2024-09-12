use std::fs;

use bfrs::{graph::Graph, ir::Ir, Ast};

fn test_lower(src: &str, expect: &str) {
    let mut g = Graph::new();
    let ast = Ast::parse(src.as_bytes()).unwrap();
    let ir = Ir::lower(&ast, &mut g);
    assert!(Ir::compare_pretty_root(&ir, expect, &g));
}

fn test_optimize(src: &str, expect: &str) {
    let mut g = Graph::new();
    let ast = Ast::parse(src.as_bytes()).unwrap();
    let mut ir = Ir::lower(&ast, &mut g);
    Ir::optimize_root(&mut ir, &mut g);
    assert!(Ir::compare_pretty_root(&ir, expect, &g));
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
fn closed_form_loops() {
    test_optimize("[-]", "@0 = 0");
    test_optimize(
        "[->+<]",
        "
            guard_shift 1
            @0 = 0
            @1 = @1 + @0
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
            @1 = @1 + @0 * 3
        ",
    );
    test_optimize(
        "[->-->+++<<]",
        "
            guard_shift 1
            guard_shift 2
            @0 = 0
            @1 = @1 + @0 * -2
            @2 = @2 + @0 * 3
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
            @1 = @1 + @0 * -85
            @2 = @2 + @0 * 86
            @3 = @3 + @0 * 85
            @4 = @4 + @0 * -86
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
