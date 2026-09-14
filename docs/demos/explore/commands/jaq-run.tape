# Execute a query and explore the result.
# Page: docs/src/content/docs/explore/commands/jaq.md
Output docs/public/demos/explore/commands/jaq-run.gif
Output docs/demos/.work/explore/commands/jaq-run.txt
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
Type@500ms "jjj"
Sleep 2s
Type@500ms "kk"
Sleep 2s
Type "g"
Space
Sleep 2s
Space
Sleep 2s

Sleep 3s
Screenshot docs/public/demos/explore/commands/jaq-run.png
Sleep 500ms
Hide
Type ":"
Ctrl+U
Type "quit"
Enter
Sleep 300ms
Type "exit"
Enter
