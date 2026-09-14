# Print a subtree to stdout and inspect the redirected output.
# Page: docs/src/content/docs/explore/commands/print.md
Output docs/public/demos/explore/commands/print.gif
Output docs/demos/.work/explore/commands/print.txt
Source docs/demos/shared/common.tape

Type "vy sample.json > selected.json"
Sleep 750ms
Enter
Wait+Screen /items/
Sleep 2s
Type ":"
Ctrl+U
Type "print @0 .items[0]"
Sleep 750ms
Enter
Sleep 1s
Type "cat selected.json"
Sleep 750ms
Enter
Sleep 3s

Sleep 3s
Screenshot docs/public/demos/explore/commands/print.png
Sleep 500ms
Hide
Type "exit"
Enter
