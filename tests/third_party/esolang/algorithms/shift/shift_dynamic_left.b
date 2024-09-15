[Move pointer x (empty) cells to the left

Attribution: Kman <https://esolangs.org/wiki/User:Kman>

Move your pointer to the left as many times as the value of x. (Non-Wrapping)

(pointer on cell x)]

<++[-[+<]+[>]<-]<[[-]<]

[Note:
- All cells between x and x-x are zeroed unless you remove the [-] from the
  [[-]<]-s. If you do, they will all be incremented.
- If there are n nonzero cells between x and x-(x+n) (watch out, recursive), you
  will 'skip' the nonzero cells.
- Ensure there are zeroed cells after cell x, or the one next to cell x will
  become the new "cell x".
- If you are expecting to move as low as 1 cells, remove the <++ from the
  beginning, and you will move x-1. (As shown, only works with x>1)]
