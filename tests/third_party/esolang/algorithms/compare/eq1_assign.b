[x´ = x == y (wrapping)

Attribution: Jeffry Johnston <https://esolangs.org/wiki/User:Calamari>

The algorithm returns either 0 (false) or 1 (true) and preserves y.

Layout: x y temp0 temp1]

temp0>>[-]
temp1>[-]
x<<<[temp1>>>+x<<<-]+
y>[temp1>>-temp0<+y<-]
temp0>[y<+temp0>-]
temp1>[x<<<-temp1>>>[-]]
