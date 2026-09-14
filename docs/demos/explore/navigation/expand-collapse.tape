# Demonstrate expand-collapse with keyboard operations only.
# Page: docs/src/content/docs/explore/navigation.md
Output docs/public/demos/explore/navigation/expand-collapse.gif
Output docs/demos/.work/explore/navigation/expand-collapse.txt
Source docs/demos/shared/common.tape

Type "vy sample.json"
Sleep 750ms
Enter
Wait+Screen /items/
Sleep 2s
Type "E"
Sleep 3s
Type "e"
Sleep 3s

Sleep 3s
Screenshot docs/public/demos/explore/navigation/expand-collapse.png
Sleep 500ms
Hide
Escape
Sleep 300ms
Type "exit"
Enter
