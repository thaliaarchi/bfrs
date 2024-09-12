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

#[test]
fn lower_bb() {
    test_lower(
        "++><<->.,>>",
        "
            guard_shift 1
            guard_shift -1
            output @0 + 2
            in0 = input
            guard_shift 2
            @-1 = @-1 - 1
            @0 = in0
            shift 2
        ",
    );
}

#[test]
fn lower() {
    // Excerpt from https://www.brainfuck.org/collatz.b
    let src = "[-[<->-]+[<<<<]]<[>+<-]";
    let expect = "
        while @0 != 0 {
            {
                @0 = @0 - 1
            }
            while @0 != 0 {
                guard_shift -1
                @-1 = @-1 - 1
                @0 = @0 - 1
            }
            {
                @0 = @0 + 1
            }
            while @0 != 0 {
                guard_shift -1
                guard_shift -2
                guard_shift -3
                guard_shift -4
                shift -4
            }
        }
        {
            guard_shift -1
            shift -1
        }
        while @0 != 0 {
            guard_shift 1
            @0 = @0 - 1
            @1 = @1 + 1
        }
    ";
    test_lower(src, expect);
    let expect = "
        if @0 != 0 {
            {
                guard_shift -1
                @-1 = @-1 + (@0 - 1) * -1
                @0 = 1
            }
            while @0 != 0 {
                guard_shift -1
                guard_shift -2
                guard_shift -3
                guard_shift -4
                shift -4
            }
        }
        {
            guard_shift -1
            @-1 = 0
            @0 = @0 + @-1
            shift -1
        }
    ";
    test_optimize(src, expect);
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
