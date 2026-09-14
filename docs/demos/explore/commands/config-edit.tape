# Edit the configuration with vi and reload it.
# Page: docs/src/content/docs/explore/commands/config.md
Output docs/public/demos/explore/commands/config-edit.gif
Output docs/demos/.work/explore/commands/config-edit.txt
Source docs/demos/shared/common.tape

Hide
Type@0ms "export VISUAL='vi -u NONE -i NONE -n'"
Enter
Sleep 500ms
Ctrl+L
Show
Type "vy sample.json"
Sleep 750ms
Enter
Wait+Screen /items/
Sleep 2s
Type ":"
Ctrl+U
Type "config edit"
Sleep 750ms
Enter
Sleep 3s
Type "/^indent = 2$"
Enter
Sleep 2s
Type "0f2r4"
Escape
Sleep 2s
Type ":wq"
Sleep 750ms
Enter
Sleep 3s

Sleep 3s
Screenshot docs/public/demos/explore/commands/config-edit.png
Sleep 500ms
Hide
Escape
Sleep 300ms
Type "exit"
Enter
