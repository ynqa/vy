# Open a query result in focus and return to the result view.
# Page: docs/src/content/docs/explore/focus-and-jaq.md
Output docs/public/demos/explore/focus-and-jaq/jaq-focus.gif
Output docs/demos/.work/explore/focus-and-jaq/jaq-focus.txt
Source docs/demos/shared/common.tape

Type "vy sample.json"
Sleep 750ms
Enter
Wait+Screen /items/
Sleep 2s
Type ":"
Ctrl+U
Type 'jaq .items[] | select(.status == "failed")'
Sleep 750ms
Enter
Sleep 3s
Type ":"
Ctrl+U
Type "focus"
Sleep 750ms
Enter
Sleep 3s
Type@500ms "jj"
Sleep 3s
Screenshot docs/public/demos/explore/focus-and-jaq/jaq-focus.png
Sleep 500ms
Escape
Sleep 3s
Hide
Type ":"
Ctrl+U
Type "quit"
Enter
Sleep 300ms
Type "exit"
Enter
