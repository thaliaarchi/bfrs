[z = x > y

Attribution: ais523 <https://esolangs.org/wiki/User:Ais523>

This uses balanced loops only, and requires a wrapping implementation (and will
be very slow with large numbers of bits, although the number of bits otherwise
doesn't matter.) The temporaries and x are left at 0; y is set to y-x. (You
could make a temporary copy of x via using another temporary that's incremented
during the loop.)

Layout: x y z temp0 temp1]

temp0>>>[-]temp1>[-]z<<[-]
x<<[ temp0>>>+
       y<<[- temp0>>[-] temp1>+ y<<<]
   temp0>>[- z<+ temp0>]
   temp1>[- y<<<+ temp1>>>]
   y<<<- x<- ]