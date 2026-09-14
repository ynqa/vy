# Complete a configuration key and inspect its value.
# Page: docs/src/content/docs/explore/commands/config.md
Output docs/public/demos/explore/commands/config-get.gif
Output docs/demos/.work/explore/commands/config-get.txt
Source docs/demos/shared/common.tape

Type "vy sample.json"
Sleep 750ms
Enter
Wait+Screen /items/
Sleep 2s
Type ":config get json.inden"
Tab
Sleep 2s
Enter
Sleep 3s
Type ":"
Ctrl+U
Type "config get json.overflow_mode"
Sleep 750ms
Enter
Sleep 3s

Sleep 3s
Screenshot docs/public/demos/explore/commands/config-get.png
Sleep 500ms
Hide
Escape
Sleep 300ms
Type "exit"
Enter
