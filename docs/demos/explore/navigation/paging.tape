# Demonstrate paging with keyboard operations only.
# Page: docs/src/content/docs/explore/navigation.md
Output docs/public/demos/explore/navigation/paging.gif
Output docs/demos/.work/explore/navigation/paging.txt
Source docs/demos/shared/common.tape

Type "vy navigation.json"
Sleep 750ms
Enter
Wait+Screen /items/
Sleep 2s
PageDown
Sleep 2s
PageUp
Sleep 2s
Ctrl+D
Sleep 2s
Ctrl+U
Sleep 2s

Sleep 3s
Screenshot docs/public/demos/explore/navigation/paging.png
Sleep 500ms
Hide
Escape
Sleep 300ms
Type "exit"
Enter
