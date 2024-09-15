[if (x) { code1 } else { code2 }

Attribution: Ben-Arba

(25 OPs)

This is an alternate approach. It's more efficient since it doesn't require
copying x, but it does require that temp0 and temp1 follow x consecutively in
memory.

Layout: x temp0 temp1 temp2]

temp0>[-]+
temp1>[-]
x<<[
 code1 .
 x>-]>
[<
 code2 .
 x>->]<<
