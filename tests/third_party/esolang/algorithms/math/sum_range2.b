[Summing 1~n

Attribution: User:A

Puts the sum from 1 to y (inclusive) into z. x must be 1.

Layout: x y z temp0

Algorithm:
    x := 1
    y := input
    z := 0
    while y != 0 {
        z += x
        x += 1
        y -= 1
    }
    output z
]

x+y>,[x<[-z>>+temp0>+x<<<]temp0>>>[-x<<<+temp0>>>]x<<<+y>-]z>.
