[x´ = x / y

Attribution: Jeffry Johnston <https://esolangs.org/wiki/User:Calamari>

Layout: x y temp0 temp1 temp2 temp3]

temp0>>[-]
temp1>[-]
temp2>[-]
temp3>[-]
x<<<<<[temp0>>+x<<-]
temp0>>[
 y<[temp1>>+temp2>+y<<<-]
 temp2>>>[y<<<+temp2>>>-]
 temp1<[
  temp2>+
  temp0<<-[temp2>>[-]temp3>+temp0<<<-]
  temp3>>>[temp0<<<+temp3>>>-]
  temp2<[
   temp1<-
   [x<<<-temp1>>>[-]]+
  temp2>-]
 temp1<-]
 x<<<+
temp0>>]