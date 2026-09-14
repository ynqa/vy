# Query a focused record and return to the subtree.
# Page: docs/src/content/docs/explore/focus-and-jaq.md
Output docs/public/demos/explore/focus-and-jaq/focus-jaq.gif
Output docs/demos/.work/explore/focus-and-jaq/focus-jaq.txt
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
Type ":"
Ctrl+U
Type "jaq {name, seconds: (.duration_ms / 1000)}"
Sleep 750ms
Enter
Sleep 3s
Screenshot docs/public/demos/explore/focus-and-jaq/focus-jaq.png
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
