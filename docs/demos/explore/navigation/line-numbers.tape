# Demonstrate line-numbers with keyboard operations only.
# Page: docs/src/content/docs/explore/navigation.md
Output docs/public/demos/explore/navigation/line-numbers.gif
Output docs/demos/.work/explore/navigation/line-numbers.txt
Source docs/demos/shared/common.tape

Type "vy sample.json"
Sleep 750ms
Enter
Wait+Screen /items/
Sleep 2s
Alt+n
Sleep 3s
Alt+n
Sleep 3s

Sleep 3s
Screenshot docs/public/demos/explore/navigation/line-numbers.png
Sleep 500ms
Hide
Escape
Sleep 300ms
Type "exit"
Enter
