# Filter full paths and values with AND/OR, then jump back to the tree.
# Page: docs/src/content/docs/explore/commands/flatten.md
Output docs/public/demos/explore/commands/flatten.gif
Output docs/demos/.work/explore/commands/flatten.txt
Source docs/demos/shared/common.tape

Type "vy sample.json"
Sleep 750ms
Enter
Wait+Screen /items/
Sleep 2s
Type ":"
Ctrl+U
Type "flatten"
Sleep 750ms
Enter
Sleep 3s

# Both expressions must match the same flattened line.
Type ":"
Ctrl+U
Type "status&failed"
Sleep 750ms
Enter
Sleep 3s
Escape
Sleep 2s

# An OR expression also includes duration values.
Type ":"
Ctrl+U
Type "failed|duration_ms"
Sleep 750ms
Enter
Sleep 3s
Escape
Sleep 2s

# Restore the precise filter and navigate to its source node.
Type ":"
Ctrl+U
Type "status&failed"
Sleep 750ms
Enter
Sleep 3s
Escape
Sleep 2s
Screenshot docs/public/demos/explore/commands/flatten.png
Sleep 500ms
Enter
Sleep 3s

Sleep 4s
Hide
Type ":"
Ctrl+U
Type "quit"
Enter
Sleep 300ms
Type "exit"
Enter
