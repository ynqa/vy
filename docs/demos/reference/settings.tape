# Complete setting keys/values and inspect the resulting TOML.
# Page: docs/src/content/docs/reference/settings/display.md
Output docs/public/demos/reference/settings.gif
Output docs/demos/.work/reference/settings.txt
Source docs/demos/shared/common.tape

Type "vy sample.json"
Sleep 750ms
Enter
Wait+Screen /items/
Sleep 2s

# Complete a setting key.
Type ":config get json.ind"
Tab
Sleep 2s
Enter
Sleep 2s
Type ":"
Ctrl+U
Type "config set json.indent 4"
Sleep 750ms
Enter
Sleep 3s

# Complete a boolean value.
Type ":config set json.show_line_numbers f"
Tab
Sleep 2s
Enter
Sleep 3s
Type ":"
Ctrl+U
Type "config get json.show_line_numbers"
Sleep 750ms
Enter
Sleep 3s
Type ":"
Ctrl+U
Type "config view"
Sleep 750ms
Enter
Sleep 3s
Sleep 4s
Escape

Sleep 4s
Screenshot docs/public/demos/reference/settings.png
Sleep 500ms
Hide
Type ":"
Ctrl+U
Type "quit"
Enter
Sleep 300ms
Type "exit"
Enter
