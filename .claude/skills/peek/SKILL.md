---
name: peek
description: Shows a range of lines from a file and explains what that code does. Usage is /peek FILE START END.
---

The user ran: /peek $ARGUMENTS

1. Use the Read tool to read the file `$ARGUMENTS[0]`.
2. Take only lines $1 to $2 (line numbers start at 1).
3. Print those lines in a code block, each one prefixed with its line number.
4. Below the code block, explain in 2 or 3 short sentences what those lines do.

If the file does not exist, or line $1 is past the end of the file, say so and stop.
