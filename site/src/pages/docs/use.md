---
layout: ../../layouts/DocsLayout.astro
title: Commands and keys
description: What to type and what to press.
---

## Commands

```sh
ghdeck                                    # dashboard
ghdeck list                               # what needs attention
ghdeck list all                           # everything
ghdeck show JuliaGeometry/Meshes.jl#1428  # one conversation
ghdeck sync                               # refresh now
ghdeck --version
```

## Filters

`attention`, `open`, `yours`, `to-review` and `all`.

Attention means open items that are conflicting, have changes requested, have CI failing, or wait on your review.

## Keys

| key | does |
| --- | --- |
| up down | move |
| left right | switch filter |
| 1 to 5 | jump to a filter |
| enter | read the conversation |
| o | open in browser |
| y | copy url |
| r | sync |
| / | search |
| ? | help |
| q | quit |

In a conversation, up and down scroll, `b` expands bot messages, left or esc goes back.
