[z = x or y (boolean, logical) (wrapping)

Attribution: Yuval Meshorer <https://esolangs.org/wiki/User:YuvalM>

Consumes x and y, does not use a temporary cell. Makes z 1 (true) or 0 (false)
if either x or y are one.

Layout: x y z]

z>>[-]
x<<[y>+x<-]
y>[[-]
z>+y<]
