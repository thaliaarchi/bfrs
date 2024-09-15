[x´ = x or y (boolean, logical) (wrapping)

Attribution: Yuval Meshorer <https://esolangs.org/wiki/User:YuvalM>

Returns 1 (x = 1) if either x or y are 1 (0 otherwise)

If you use it in the case that x>1 or y>1,please make sure it won't cause
overflow problem.

For example,if x=1 and y=255, than x will be 0.

Layout: x y]

x[
 y>+x<-]
y>[
 x<+y>[-]
]
