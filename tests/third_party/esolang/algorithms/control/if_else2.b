[if (x) { code1 } else { code2 }

Attribution: Daniel Marschall

(33 OPs)

Layout: x temp0 temp1]

temp0>[-]+
temp1>[-]
x<<[
 code1 .
 temp0>-
 x<[temp1>>+x<<-]
]
temp1>>[x<<+temp1>>-]
temp0<[
 code2 <.>
temp0-]
