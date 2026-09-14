# Follow the first-steps flow: move, toggle, goto, focus, search, return, quit.
# Page: docs/src/content/docs/getting-started.md
Output docs/public/demos/getting-started.gif
Output docs/demos/.work/getting-started.txt
Source docs/demos/shared/common.tape

Type "vy sample.json"
Sleep 750ms
Enter
Wait+Screen /items/
Sleep 2s
Type@500ms "jjjj"
Sleep 1s
Type@500ms "kkk"
Sleep 1s
Type@500ms "jjj"
Sleep 1s
Type@500ms "kkk"
Sleep 2s
Space
Sleep 2s
Space
Sleep 2s
Type ":"
Ctrl+U
Type "goto @0 .items[0]"
Sleep 750ms
Enter
Sleep 3s
Type ":"
Ctrl+U
Type "focus"
Sleep 750ms
Enter
Sleep 3s
Type "/"
Ctrl+U
Type "failed"
Sleep 750ms
Enter
Sleep 3s
Escape
Sleep 2s
Screenshot docs/public/demos/getting-started.png
Sleep 500ms
Type ":"
Ctrl+U
Type "quit"
Sleep 750ms
Enter
Sleep 2s
Hide
Type "exit"
Enter
