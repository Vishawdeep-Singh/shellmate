---
name: loc
description: Counts lines of code per file and lists the biggest files. Usage is /loc DIR EXT TOP, for example /loc src rs 5.
context: fork
---

The user ran: /loc $ARGUMENTS

Run this exact command with the Bash tool:

bash .claude/skills/loc/scripts/loc.sh "$ARGUMENTS[0]" "$ARGUMENTS[1]" "$ARGUMENTS[2]"

Missing arguments are fine. The script fills in defaults (DIR is the current directory, EXT is rs, TOP is 10).

Show the script's output in a code block. Then add one sentence naming the largest file and roughly what share of the total lines it has.

If the script prints an error, show the error and stop.
