# Follow growing output, cancel the query, and browse the partial results.
# Page: docs/src/content/docs/explore/commands/jaq.md
Output docs/public/demos/explore/commands/jaq-cancel.gif
Output docs/demos/.work/explore/commands/jaq-cancel.txt
Source docs/demos/shared/common.tape

Type "vy sample.json"
Sleep 750ms
Enter
Wait+Screen /items/
Sleep 2s
Type ":"
Ctrl+U
Type "jaq 0 | recurse(.+1)"
Sleep 750ms
Enter
Wait+Screen /running/
Sleep 1s
Type "G"
Sleep 1s
Type "G"
Sleep 1s
Type "G"
Sleep 1s
Ctrl+C
Wait+Screen /cancelled/
Type "G"
Sleep 3s
Screenshot docs/public/demos/explore/commands/jaq-cancel.png
Sleep 500ms
Type "g"
Sleep 2s

Hide
Type ":"
Ctrl+U
Type "quit"
Enter
Sleep 300ms
Type "exit"
Enter
