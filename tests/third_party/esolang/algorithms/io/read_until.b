[while(c=getchar()!=X)

Uses X to represent the getchar-until char. Needs overflow and underflow in TIO.
Preserves result(equal 0, unequal 1)in t1. Preserves x and y.

Layout: x y t1 t2 t3 t4]

y>X++++++++++x<+[,[-t1>>+t3>>+x<<<<]y>[-t2>>+t4>>+y<<<<]t1>[-x<<+t1>>]t2>[-y<<+t2>>]t3>[-t4>+t3<]t4>[++t3<]t4>[-t1<<<+t4>>>]t1<<<]