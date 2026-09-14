# Reedit a query against the original input.
# Page: docs/src/content/docs/explore/commands/jaq.md
Output docs/public/demos/explore/commands/jaq-reedit.gif
Output docs/demos/.work/explore/commands/jaq-reedit.txt
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
Sleep 2s
Ctrl+U
Type "jaq .items[] | {name, status}"
Sleep 750ms
Enter
Sleep 3s

Sleep 3s
Screenshot docs/public/demos/explore/commands/jaq-reedit.png
Sleep 500ms
Hide
Type ":"
Ctrl+U
Type "quit"
Enter
Sleep 300ms
Type "exit"
Enter
