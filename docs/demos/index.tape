# A short tour: jump to a record, focus it, and query the original input.
# Page: docs/src/content/docs/index.md
Output docs/public/demos/index.gif
Output docs/demos/.work/index.txt
Source docs/demos/shared/common.tape

Type "vy sample.json"
Sleep 750ms
Enter
Wait+Screen /items/
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
Type@350ms "s"
Sleep 4s
Escape
Sleep 1s
Type ":"
Ctrl+U
Type 'jaq .items[] | select(.status == "failed")'
Sleep 750ms
Enter
Sleep 3s
Type@350ms "s"
Sleep 4s

Sleep 4s
Screenshot docs/public/demos/index.png
Sleep 500ms
Hide
Type ":"
Ctrl+U
Type "quit"
Enter
Sleep 300ms
Type "exit"
Enter
