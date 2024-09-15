[Input a decimal number

Attribution: Urban Müller (1993)

Value is input into the current cell, uses three more cells to the right. End of
the number is a newline, or eof. All other character are treated the same as
digits. Works correctly with bignum cells.]

[-]>[-]+    // Clear sum
[[-]                // Begin loop on first temp
>[-],               // Clear the inp buffer to detect leave on eof and input
    [
        +[                          // Check for minus one on eof
            -----------[            // Check for newline
                >[-]++++++[<------>-]       // Subtract 38 to get the char in zero to nine
                <--<<[->>++++++++++<<]      // Multiply the existing value by ten
                >>[-<<+>>]          // and add in the new char
            <+>]
        ]
    ]
<]
<
// Current cell is the number input
