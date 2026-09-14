# Demonstrate move with keyboard operations only.
# Page: docs/src/content/docs/explore/navigation.md
Output docs/public/demos/explore/navigation/move.gif
Output docs/demos/.work/explore/navigation/move.txt
Source docs/demos/shared/common.tape

Type "vy sample.json"
Sleep 750ms
Enter
Wait+Screen /items/
Sleep 2s
Type@350ms "jjjj"
Sleep 2s
Type@350ms "kk"
Sleep 2s
Down
Sleep 1s
Up
Sleep 2s

Sleep 3s
Screenshot docs/public/demos/explore/navigation/move.png
Sleep 500ms
Hide
Escape
Sleep 300ms
Type "exit"
Enter
