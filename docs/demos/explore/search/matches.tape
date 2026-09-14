# Search a regex and move between matches.
# Page: docs/src/content/docs/explore/search.md
Output docs/public/demos/explore/search/matches.gif
Output docs/demos/.work/explore/search/matches.txt
Source docs/demos/shared/common.tape

Type "vy sample.json"
Sleep 750ms
Enter
Wait+Screen /items/
Sleep 2s
Type "E"
Sleep 1s
Type "/"
Type "failed|scheduled"
Sleep 750ms
Enter
Sleep 3s
Type "n"
Sleep 2s
Type "N"
Sleep 2s

Sleep 3s
Screenshot docs/public/demos/explore/search/matches.png
Sleep 500ms
Hide
Escape
Sleep 300ms
Type "exit"
Enter
