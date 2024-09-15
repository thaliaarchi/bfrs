# break and continue

Attribution: [Sunjay Varma](https://esolangs.org/wiki/User:Sunjay)

To implement break and continue statements in loops, consider that the following
two pieces of pseudocode are functionally equivalent:

```javascript
while (foo) {
 if (bar == foo) {
  if (x > 2) {
   break;
  }
  else {
   // do stuff
  }
  // do stuff
 }
 // update foo for the next iteration
}

// Equivalent without break statement:
while (foo) {
 shouldBreak = false
 if (bar == foo) {
  if (x > 2) {
   shouldBreak = true
  }
  else {
   // do stuff
  }

  // don't evaluate any more code in the loop after breaking
  if (!shouldBreak) {
   // do stuff
  }
 }
 if (shouldBreak) {
  // so that the loop stops
  foo = 0
 }
 else {
  // update foo for the next iteration
 }
}
```

Notice that we need to guard all code after the break statement in the loop to
prevent it from running. We don't need to guard in the else statement
immediately after the break statement because that will never run after the
break statement has run.

This approach allows us to implement break and continue statements in brainfuck
despite the lack of sophisticated jump instructions. All we're doing is
combining the concept of an if statement (defined below) with the while loop we
just defined and applying it here.

Implementing a continue statement is the same thing except you never guard the
loop updating code:

```javascript
while (foo) {
 if (bar == foo) {
  if (x > 2) {
   continue;
  }
  else {
   // do stuff
  }
  // do stuff
 }
 // update foo for the next iteration
}

// Equivalent without continue statement:
while (foo) {
 shouldContinue = false
 if (bar == foo) {
  if (x > 2) {
   shouldContinue = true
  }
  else {
   // do stuff
  }

  // don't evaluate any more code in the loop after continuing
  if (!shouldContinue) {
   // do stuff
  }
 }

 // This code stays the same after a continue because we still want to move on to the next iteration of the loop
 // update foo for the next iteration
}
```

To implement both break and continue, you can compose the concepts here and make
any combination you want. You can consider break and continue statements to be
"sugar" that needs to be "desugared" in your brainfuck code.
