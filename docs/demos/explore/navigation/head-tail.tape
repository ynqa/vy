# Demonstrate head-tail with keyboard operations only.
# Page: docs/src/content/docs/explore/navigation.md
Output docs/public/demos/explore/navigation/head-tail.gif
Output docs/demos/.work/explore/navigation/head-tail.txt
Source docs/demos/shared/common.tape

Type "vy navigation.json"
Sleep 750ms
Enter
Wait+Screen /items/
Sleep 2s
Type "G"
Sleep 3s
Type "g"
Sleep 3s

Sleep 3s
Screenshot docs/public/demos/explore/navigation/head-tail.png
Sleep 500ms
Hide
Escape
Sleep 300ms
Type "exit"
Enter
