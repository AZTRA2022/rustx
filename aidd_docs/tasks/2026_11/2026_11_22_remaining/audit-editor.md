# Editor and language server audit

Read from `crates/rustx-lsp/src/lib.rs` (capabilities it announces) and `editors/vscode` (what ships). Not measured against a live editor: a ranking, not a benchmark.

## What exists

| Area | State |
| ---- | ----- |
| Diagnostics from the compiler, on the right line | yes (maps), `system/*.rsx` now shadowed |
| Completion (+ resolve), hover, definition, type definition, implementation | yes, forwarded to rust-analyzer through the shadow |
| References, highlight, document symbols, folding, inlay hints, signature help | yes |
| Snippets | 27 (the `.rsx` ones; ES7+ ships roughly 100 for JS/React) |
| Grammar, language configuration | yes |

## Ranked gaps (against rust-analyzer and ES7+ React snippets)

| # | Gap | Why it is missing | Cost | Rank |
| - | --- | ----------------- | ---- | ---- |
| 1 | Rename | columns move on the lines the rewrite rebuilt; a wrong column rewrites the wrong text (comment in `lib.rs`) | needs a column map per line | high value, risky |
| 2 | Code actions (quick fixes) | same column problem | same map | high |
| 3 | Formatting of `.rsx` | the text formatted is generated Rust | a formatter for the markup subset | medium |
| 4 | Semantic tokens | grammar is TextMate only | rust-analyzer tokens through the shadow, same map | medium |
| 5 | More snippets (hooks-like `let mut` patterns, route files, `system/` pages) | cheap | JSON only | low, quick win |
| 6 | Workspace symbols, call hierarchy | not forwarded | forwarding | low |

## Decision

Items 1 and 2 share one prerequisite: a per-line column map from `.rsx` to the generated Rust. Until that exists, the server rightly refuses to answer. That map is the next piece of work in the language server; it is not started here because it needs its own design and an editor to prove it in.
