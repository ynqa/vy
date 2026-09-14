# Demonstrate toggle with keyboard operations only.
# Page: docs/src/content/docs/explore/navigation.md
Output docs/public/demos/explore/navigation/toggle.gif
Output docs/demos/.work/explore/navigation/toggle.txt
Source docs/demos/shared/common.tape

Type "vy sample.json"
Sleep 750ms
Enter
Wait+Screen /items/
Sleep 2s
Type@350ms "ll"
Sleep 2s
Space
Sleep 2s
Enter
Sleep 2s

Sleep 3s
Screenshot docs/public/demos/explore/navigation/toggle.png
Sleep 500ms
Hide
Escape
Sleep 300ms
Type "exit"
Enter
