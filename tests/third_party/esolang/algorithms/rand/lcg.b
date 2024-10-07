[x = pseudo-random number

Attribution: Jeffry Johnston

This algorithm employs a linear congruential generator of the form:
 V = (A * V + B) % M

Where:
 A = 31821, B = 13849, M = period = 65536, V = initial seed

A and B values were obtained from the book:
Texas Instruments TMS320 DSP DESIGNER'S NOTEBOOK Number 43 Random Number
Generation on a TMS320C5x, by Eric Wilbur

Assumes 8-bit cells. After the code is executed, the variable "x" holds a
pseudo-random number from 0 to 255 (the high byte of V, above). The variable
cells "randomh" and "randoml" are the internal random number seed and should not
be altered while random numbers are being generated.

Layout: x randomh randoml temp0 temp1 temp2 temp3 temp4 temp5]

temp0>>>[-]
temp1>[-]
temp2>[-]
temp3>[-]
temp4>[-]
temp5>[-]
randomh<<<<<<<[temp0>>+randomh<<-]
randoml>[temp1>>+randoml<<-]
temp3>>>>+++++++[temp2<+++++++++++@temp3>-]
temp2<[
 temp0<<[randomh<<+temp3>>>>>+temp0<<<-]
 temp3>>>[temp0<<<+temp3>>>-]
 temp1<<[randomh<<<+temp3>>>>>+temp4>+temp1<<<-]
 temp4>>>[temp1<<<+temp4>>>-]
 temp3<[
  randoml<<<<+[temp4>>>>>+temp5>+randoml<<<<<<-]
  temp5>>>>>>[randoml<<<<<<+temp5>>>>>>-]+
  temp4<[temp5>-temp4<[-]]
  temp5>[randomh<<<<<<<+temp5>>>>>>>-]
 temp3<<-]
temp2<-]
++++++[temp3>++++++++temp2<-]
temp3>-[
 temp1<<[randomh<<<+temp2>>>>+temp1<-]
 temp2>[temp1<+temp2>-]
temp3>-]
temp0<<<[-]temp1>[-]+++++[temp0<+++++temp1>-]
temp0<[
 randoml<+[temp1>>+temp2>+randoml<<<-]
 temp2>>>[randoml<<<+temp2>>>-]+
 temp1<[temp2>-temp1<[-]]
 temp2>[randomh<<<<+temp2>>>>-]
temp0<<-]
++++++[randomh<<+++++++++temp0>>-]
randomh<<[x<+temp0>>>+randomh<<-]
temp0>>[randomh<<+temp0>>-]