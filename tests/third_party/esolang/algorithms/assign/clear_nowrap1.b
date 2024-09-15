[x = 0 (non-wrapping)

Attribution: quintopia <https://esolangs.org/wiki/User:Quintopia>

This will clear a cell no matter its sign in an unbounded signed
implementation. Also clobbers the cell to the left of x and the cell to the
right of temp.]

temp[-]†
>[-]†
x<[-]>[†
  temp>-[x+temp+>+]
  x[temp>]
  <[+[x-temp>-<-]x<]
  >
]
temp[-]
>[+]

[† Each of these lines should have their polarities reversed if temp, the cell
to the right of temp, and the cell to the left of x, respectively, contain a
negative value.

Note that rather than just clearing the two cells at temp, one could condition
upon the sign that x had.]
