[z = MUX(a, x, y) (boolean, logical)

Attribution: Yuval Meshorer <https://esolangs.org/wiki/User:YuvalM>

If a is equal to 1, then z is equal to y. Otherwise, if a is equal to 0, z will
be equal to x. When done, a, x, and y will all be 0 regardless of their starting
values. e.g: IN: x = 0, y = 1, a = 1 OUT: x = 0, y = 0, a = 0, z = 1

Layout: a x y z]

z>>>[-]
y<[
 a<<[z>>>+a<<<-]
y>>-]
x<[
 a<-[
  [-]z>>>[-]+
 a<<<]
x>-]
a<[-]

[
|   INPUT   | OUTPUT |
| a | x | y |   z    |
| --------- | ------ |
| 0 | 0 | 0 |   0    |
| 0 | 0 | 1 |   0    |
| 0 | 1 | 0 |   1    |
| 0 | 1 | 1 |   1    |
| 1 | 0 | 0 |   0    |
| 1 | 0 | 1 |   1    |
| 1 | 1 | 0 |   0    |
| 1 | 1 | 1 |   1    |
]
