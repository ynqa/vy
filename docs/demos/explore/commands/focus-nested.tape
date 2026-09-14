# Open nested subtrees and return one level at a time.
# Page: docs/src/content/docs/explore/commands/focus.md
Output docs/public/demos/explore/commands/focus-nested.gif
Output docs/demos/.work/explore/commands/focus-nested.txt
Source docs/demos/shared/common.tape

Type "vy sample.json"
Sleep 750ms
Enter
Wait+Screen /items/
Sleep 2s
Type ":"
Ctrl+U
Type "focus @0 .items"
Sleep 750ms
Enter
Sleep 3s
Type ":"
Ctrl+U
Type "goto @0 [0]"
Sleep 750ms
Enter
Sleep 2s
Type ":"
Ctrl+U
Type "focus"
Sleep 750ms
Enter
Sleep 3s
Type@500ms "jj"
Sleep 2s
Screenshot docs/public/demos/explore/commands/focus-nested.png
Sleep 500ms
Escape
Sleep 3s
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
