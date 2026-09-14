---
title: "jaq"
description: "Extract and transform data with the built-in jaq engine, then continue exploring the results."
sidebar:
  order: 4
---

Extract and transform data with expressions, then explore the results as a tree. Refine an expression while comparing the input and results side by side.

## Run a query

![Extract matching elements, then navigate, expand, and collapse the result tree](/demos/explore/commands/jaq-run.gif)

vy embeds *jaq*. It supports extraction and transformation with jq-like expressions without starting an external jq process. Full compatibility with jq is not guaranteed.

Use `:jaq <expr>` to query the documents you are browsing. For example, extract elements from the `items` array whose `status` is `failed`:

```text
:jaq .items[] | select(.status == "failed")
```

Running a query from the input or *focus* view opens the output in a new view for further exploration. `:j` is an alias. Expressions do not pass through a shell, so you do not need to wrap the entire expression in shell quotes.

Navigate the result tree with `j` / `k` and expand or collapse nodes with `Space`. You can also use `/` search, `:goto`, `:flatten`, `:focus`, `:preview`, `:copy`, `:print`, and `:write`.

### Extraction and transformation examples

Use expressions to select keys, sort values, and count records. These examples assume a document with an `items` array.

```text
:jaq .items[] | {name, status}
:jaq .items | sort_by(.duration_ms)
:jaq [.items[] | select(.status == "failed")] | length
```

Multiple output values are treated as separate result documents. For YAML input, results are also displayed as YAML.

## Compare input and results

![Display input and results side by side, switch panes with Tab, and explore each pane](/demos/explore/commands/jaq-compare.gif)

Press `s` to display input and results side by side. Compare the original data with the extracted or transformed output. Press `s` again to show only the results.

Switch the active pane with `Tab` or the mouse, then navigate and expand or collapse nodes in either pane. Search and the document indices and paths used by regular commands refer to the active pane.

## Edit an expression

![Edit the filter expression to extract keys from both records in the original input](/demos/explore/commands/jaq-reedit.gif)

Press `:` to open the command editor prefilled with the current `jaq <expr>`. Edit the expression and press `Enter` to rerun it, updating the results in the current view. Use `Shift+Enter` to split long expressions across multiple lines. Press `Esc` to close the editor.

Rerunning uses the input from when that *jaq* view was first opened. For example, after filtering down to one record, changing the expression to `.items[] | {name, status}` extracts keys from every element in the original array. Editing a query does not apply it to the previous results.

You can replace the entire command editor contents to run other commands, such as `focus` or `help`.

Rerunning uses the same input regardless of which pane is active, and switches to the result pane after updating. Press `Esc` while browsing to return to the view that opened *jaq*.

See [Combining focus and jaq](../focus-and-jaq.md) to transform a subtree or open part of an extracted result.

## Partial results and cancellation

![Follow an increasing sequence from zero, cancel with Ctrl+C, and browse the results already produced](/demos/explore/commands/jaq-cancel.gif)

This demo runs a query that keeps producing `0, 1, 2, …`. Press `G` to move to the current end, then `Ctrl+C` to cancel. Results already produced remain available for browsing.

```text
:jaq 0 | recurse(.+1)
```

Output values appear as they are produced, before the query finishes. While you edit a command or search, the displayed results are held steady; new results are applied after the interaction.

`Ctrl+C` requests cancellation of the running query. Cancellation is checked between output values, so it cannot immediately force-stop a computation that produces no output.

You can use `:help` and `:config` even with empty results. Commands such as `:focus` and output commands return an error when there is no selection.

See [history](hstr.md) to reuse queries, and [*preview*](preview.md), [*copy*](copy.md), [*print*](print.md), or [*write*](write.md) to extract results.
