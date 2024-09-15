[if (x) { code1 } else { code2 }

Attribution: Jeffry Johnston <https://esolangs.org/wiki/User:Calamari>

(39 OPs)

Layout: x temp0 temp1 temp2]

temp0>[-]
temp1>[-]
x<<[temp0>+temp1>+x<<-]temp0>[x<+temp0>-]+
temp1>[
 code1 <<.>>
 temp0<-
temp1>[-]]
temp0<[
 code2 <,>
temp0-]
