# Copy the selected path to the OS clipboard.
# Page: docs/src/content/docs/explore/commands/copy.md
Output docs/public/demos/explore/commands/copy-path.gif
Output docs/demos/.work/explore/commands/copy-path.txt
Source docs/demos/shared/common.tape

Type "vy sample.json"
Sleep 750ms
Enter
Wait+Screen /items/
Sleep 2s
Type@350ms "lll"
Sleep 2s
Type ":"
Ctrl+U
Type "copy path"
Sleep 750ms
Enter
Sleep 4s

Sleep 3s
Screenshot docs/public/demos/explore/commands/copy-path.png
Sleep 500ms
Hide
Escape
Sleep 300ms
Type "exit"
Enter
