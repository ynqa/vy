# Demonstrate overflow with keyboard operations only.
# Page: docs/src/content/docs/explore/navigation.md
Output docs/public/demos/explore/navigation/overflow.gif
Output docs/demos/.work/explore/navigation/overflow.txt
Source docs/demos/shared/common.tape

Type "vy navigation.json"
Sleep 750ms
Enter
Wait+Screen /items/
Sleep 2s
Type "z"
Sleep 3s
Type "z"
Sleep 3s

Sleep 3s
Screenshot docs/public/demos/explore/navigation/overflow.png
Sleep 500ms
Hide
Escape
Sleep 300ms
Type "exit"
Enter
