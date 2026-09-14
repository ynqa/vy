# Read help and return to Browse.
# Page: docs/src/content/docs/explore/commands/help.md
Output docs/public/demos/explore/commands/help.gif
Output docs/demos/.work/explore/commands/help.txt
Source docs/demos/shared/common.tape

Type "vy sample.json"
Sleep 750ms
Enter
Wait+Screen /items/
Sleep 2s
Type ":"
Ctrl+U
Type "help"
Sleep 750ms
Enter
Sleep 3s
PageDown
Sleep 3s
Escape
Sleep 1s

Sleep 3s
Screenshot docs/public/demos/explore/commands/help.png
Sleep 500ms
Hide
Escape
Sleep 300ms
Type "exit"
Enter
