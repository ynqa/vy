# Use tree keys, pane switching, editor cancellation, and help.
# Page: docs/src/content/docs/reference/settings/keybindings.md
Output docs/public/demos/reference/keybindings.gif
Output docs/demos/.work/reference/keybindings.txt
Source docs/demos/shared/common.tape

Type "vy sample.json"
Sleep 750ms
Enter
Wait+Screen /items/
Sleep 2s
Type@350ms "jj"
Space
Sleep 2s
Space
Sleep 2s
Type@350ms "G"
Sleep 2s
Type@350ms "g"
Sleep 2s
Type ":"
Ctrl+U
Type "focus @0 .items[0]"
Sleep 750ms
Enter
Sleep 3s
Type@350ms "s"
Sleep 2s
Tab
Sleep 2s
Tab
Sleep 1s
Type "/draft"
Sleep 2s
Escape
Sleep 2s
Escape
Sleep 1s
Type ":"
Ctrl+U
Type "help"
Sleep 750ms
Enter
Sleep 3s
PageDown
Sleep 4s
Screenshot docs/public/demos/reference/keybindings.png
Sleep 500ms
Escape

Sleep 4s
Hide
Type ":"
Ctrl+U
Type "quit"
Enter
Sleep 300ms
Type "exit"
Enter
