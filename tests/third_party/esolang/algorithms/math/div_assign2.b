[x´ = x / y (wrapping)

Attribution: Softengy 17:06, 7 April 2020 (UTC)

This algorithm will compute x / y, put the remainder into x and put the quotient
into q

Layout: x y q temp0 temp1 †]

†>>>>>[-]
x<<<<<[
 temp1>>>>+[
  y<<<[x<-[temp1>>>>+†>]temp1<-temp0<+y<<-]
  temp0>>[y<<+temp0>>-]q<+temp1>>
 ]
]
x<<<<[y>[temp0>>+x<<<+y>-]temp0>>[y<<+temp0>>-]q<-†>>>]

[† Move to any location with a value of 0]
