# Complete a path, then move to a different YAML document.
# Page: docs/src/content/docs/explore/commands/goto.md
Output docs/public/demos/explore/commands/goto.gif
Output docs/demos/.work/explore/commands/goto.txt
Source docs/demos/shared/common.tape

Type "vy sample.json"
Sleep 750ms
Enter
Wait+Screen /items/
Sleep 2s
Type ":goto @0 .items[0].na"
Tab
Sleep 2s
Enter
Sleep 3s
Type ":"
Ctrl+U
Type "goto @0 .items[1].status"
Sleep 750ms
Enter
Sleep 3s
Hide
Escape
Sleep 300ms
Type@0ms "vy documents.yaml"
Enter
Wait+Screen /metadata/
Show
Sleep 2s
Type ":"
Ctrl+U
Type "goto @1 .metadata.name"
Sleep 750ms
Enter
Sleep 3s

Sleep 3s
Screenshot docs/public/demos/explore/commands/goto.png
Sleep 500ms
Hide
Escape
Sleep 300ms
Type "exit"
Enter
