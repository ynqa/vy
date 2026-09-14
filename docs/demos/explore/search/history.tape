# Recall and edit regex history, including after restarting.
# Page: docs/src/content/docs/explore/search.md
Output docs/public/demos/explore/search/history.gif
Output docs/demos/.work/explore/search/history.txt
Source docs/demos/shared/common.tape

Type "vy sample.json"
Sleep 750ms
Enter
Wait+Screen /items/
Sleep 2s
Type "/"
Ctrl+U
Type "failed"
Sleep 750ms
Enter
Sleep 2s
Type "/"
Ctrl+U
Type "duration_ms"
Sleep 750ms
Enter
Sleep 2s
Type "/"
Ctrl+U
Type "status"
Up
Sleep 2s
Up
Sleep 2s
Down
Sleep 2s
Down
Sleep 2s
Enter
Sleep 2s
Type "/"
Up
Sleep 1s
Type "|name"
Sleep 750ms
Enter
Sleep 2s
Escape
Sleep 1s
Type "vy sample.json"
Sleep 750ms
Enter
Wait+Screen /items/
Sleep 2s
Type "/"
Up
Sleep 2s
Enter
Sleep 3s

Sleep 3s
Screenshot docs/public/demos/explore/search/history.png
Sleep 500ms
Hide
Escape
Sleep 300ms
Type "exit"
Enter
