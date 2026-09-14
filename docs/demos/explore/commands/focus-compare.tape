# Compare input and focused subtree in separate panes.
# Page: docs/src/content/docs/explore/commands/focus.md
Output docs/public/demos/explore/commands/focus-compare.gif
Output docs/demos/.work/explore/commands/focus-compare.txt
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
Type "s"
Sleep 3s
Tab
Sleep 2s
Type@500ms "jjj"
Sleep 2s
Tab
Sleep 2s
Type@500ms "jj"
Sleep 2s

Sleep 3s
Screenshot docs/public/demos/explore/commands/focus-compare.png
Sleep 500ms
Type "s"
Sleep 3s
Escape
Sleep 2s
Sleep 500ms
Hide
Type ":"
Ctrl+U
Type "quit"
Enter
Sleep 300ms
Type "exit"
Enter
