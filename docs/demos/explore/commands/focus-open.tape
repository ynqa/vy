# Open a subtree and return to the original view.
# Page: docs/src/content/docs/explore/commands/focus.md
Output docs/public/demos/explore/commands/focus-open.gif
Output docs/demos/.work/explore/commands/focus-open.txt
Source docs/demos/shared/common.tape

Type "vy sample.json"
Sleep 750ms
Enter
Wait+Screen /items/
Sleep 2s
Type ":"
Ctrl+U
Type "focus @0 .items[0]"
Sleep 750ms
Enter
Sleep 3s
Screenshot docs/public/demos/explore/commands/focus-open.png
Sleep 500ms
Escape
Sleep 3s

Hide
Type ":"
Ctrl+U
Type "quit"
Enter
Sleep 300ms
Type "exit"
Enter
