# Filter command history and recall/edit a query.
# Page: docs/src/content/docs/explore/commands/hstr.md
Output docs/public/demos/explore/commands/hstr.gif
Output docs/demos/.work/explore/commands/hstr.txt
Source docs/demos/shared/common.tape

Hide
Type "vy sample.json"
Sleep 750ms
Enter
Wait+Screen /items/
Sleep 2s
Type ":"
Ctrl+U
Type "goto @0 .items[0]"
Sleep 750ms
Enter
Sleep 500ms
Type ":"
Ctrl+U
Type "jaq .items[] | {name, status}"
Sleep 750ms
Enter
Sleep 500ms
Escape
Sleep 300ms
Escape
Sleep 300ms
Ctrl+L
Show
Type "vy sample.json"
Sleep 750ms
Enter
Wait+Screen /items/
Sleep 2s
Type ":"
Ctrl+U
Type "hstr"
Sleep 750ms
Enter
Sleep 3s
Tab
Sleep 1s
Type "jaq"
Sleep 2s
Tab
Sleep 1s
Enter
Sleep 2s
Ctrl+U
Type "jaq .items[] | .name"
Sleep 750ms
Enter
Sleep 3s
Escape
Sleep 1s

Sleep 3s
Screenshot docs/public/demos/explore/commands/hstr.png
Sleep 500ms
Hide
Escape
Sleep 300ms
Type "exit"
Enter
