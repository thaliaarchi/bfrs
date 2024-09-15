[x´ = x <= y

Attribution: Ian Kelly

x and y are unsigned. temp1 is the first of three consecutive temporary cells.
The algorithm returns either 0 (false) or 1 (true).

Layout: x y temp0 temp1 temp2 temp3]

temp0>>[-]
temp1>[-] >[-]+ >[-] <<
y<<[temp0>+ temp1>+ y<<-]
temp1>>[y<<+ temp1>>-]
x<<<[temp1>>>+ x<<<-]
temp1>>>[>-]> [< x<<<+ temp0>>[-] temp1> >->]<+<
temp0<[temp1>- [>-]> [< x<<<+ temp0>>[-]+ temp1> >->]<+< temp0<-]
