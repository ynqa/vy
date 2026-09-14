# Change indentation and key colors without leaving Browse.
# Page: docs/src/content/docs/explore/commands/config.md
Output docs/public/demos/explore/commands/config-set.gif
Output docs/demos/.work/explore/commands/config-set.txt
Source docs/demos/shared/common.tape

Type "vy sample.json"
Sleep 750ms
Enter
Wait+Screen /items/
Sleep 2s
Type ":"
Ctrl+U
Type "config set json.indent 4"
Sleep 750ms
Enter
Sleep 3s
Type ":"
Ctrl+U
Type "config set json.key_style fg=cyan,attr=bold"
Sleep 750ms
Enter
Sleep 3s

Sleep 3s
Screenshot docs/public/demos/explore/commands/config-set.png
Sleep 500ms
Hide
Escape
Sleep 300ms
Type "exit"
Enter
