# Write a subtree to a new file and keep browsing.
# Page: docs/src/content/docs/explore/commands/write.md
Output docs/public/demos/explore/commands/write.gif
Output docs/demos/.work/explore/commands/write.txt
Source docs/demos/shared/common.tape

Type "vy sample.json"
Sleep 750ms
Enter
Wait+Screen /items/
Sleep 2s
Type ":"
Ctrl+U
Type "write api.json @0 .items[0]"
Sleep 750ms
Enter
Sleep 4s
Type "j"
Sleep 2s
Hide
Escape
Sleep 300ms
Show
Type "cat api.json"
Sleep 750ms
Enter
Sleep 3s

Sleep 3s
Screenshot docs/public/demos/explore/commands/write.png
Sleep 500ms
Hide
Type "exit"
Enter
