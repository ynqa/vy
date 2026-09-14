# Copy the selected subtree to the OS clipboard.
# Page: docs/src/content/docs/explore/commands/copy.md
Output docs/public/demos/explore/commands/copy-subtree.gif
Output docs/demos/.work/explore/commands/copy-subtree.txt
Source docs/demos/shared/common.tape

Type "vy sample.json"
Sleep 750ms
Enter
Wait+Screen /items/
Sleep 2s
Type@350ms "ll"
Sleep 2s
Type ":"
Ctrl+U
Type "copy subtree"
Sleep 750ms
Enter
Sleep 4s

Sleep 3s
Screenshot docs/public/demos/explore/commands/copy-subtree.png
Sleep 500ms
Hide
Escape
Sleep 300ms
Type "exit"
Enter
