# Read a decoded multiline string.
# Page: docs/src/content/docs/explore/commands/preview.md
Output docs/public/demos/explore/commands/preview.gif
Output docs/demos/.work/explore/commands/preview.txt
Source docs/demos/shared/common.tape

Type "vy sample.json"
Sleep 750ms
Enter
Wait+Screen /items/
Sleep 2s
Type ":"
Ctrl+U
Type "preview @0 .items[0].message"
Sleep 750ms
Enter
Sleep 4s
Escape
Sleep 1s

Sleep 3s
Screenshot docs/public/demos/explore/commands/preview.png
Sleep 500ms
Hide
Escape
Sleep 300ms
Type "exit"
Enter
