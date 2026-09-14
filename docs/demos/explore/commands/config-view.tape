# Read the configuration file.
# Page: docs/src/content/docs/explore/commands/config.md
Output docs/public/demos/explore/commands/config-view.gif
Output docs/demos/.work/explore/commands/config-view.txt
Source docs/demos/shared/common.tape

Type "vy sample.json"
Sleep 750ms
Enter
Wait+Screen /items/
Sleep 2s
Type ":"
Ctrl+U
Type "config view"
Sleep 750ms
Enter
Sleep 3s
PageDown
Sleep 3s
Escape
Sleep 1s

Sleep 3s
Screenshot docs/public/demos/explore/commands/config-view.png
Sleep 500ms
Hide
Escape
Sleep 300ms
Type "exit"
Enter
