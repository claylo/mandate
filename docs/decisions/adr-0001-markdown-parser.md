---
status: accepted
date: 2025-12-22
decision-makers: clay, codex
---

# Select pulldown-cmark for Markdown parsing

## Context and Problem Statement

Mandate needs to parse CommonMark input and render roff manpages while closely
mirroring jq's `build_manpage.py` behavior. We need a parser that is correct,
lightweight, and easy to integrate with a custom renderer that depends on
some sibling-aware list/paragraph handling.

## Decision Drivers

* CommonMark correctness
* Ease of custom rendering to roff
* Minimal dependencies and runtime overhead
* Ability to control HTML handling (disable or ignore raw HTML)

## Considered Options

* pulldown-cmark
* markdown-it
* markdown (markdown-rs)
* comrak

## Decision Outcome

Chosen option: "pulldown-cmark", because it provides a fast, CommonMark-correct
event stream that we can map into a minimal internal model tailored for jq-style
roff rendering, without taking on a heavier AST or extra renderer features.

### Consequences

* Good: Simple event-driven integration and minimal dependency surface.
* Good: Full control over rendering rules and escaping behavior.
* Bad: We must build a small block model to handle jq's list/paragraph adjacency.

### Confirmation

* Unit tests for Markdown-to-roff edge cases (headings, lists, code).
* Fixture-based tests using jq manual inputs.

## Pros and Cons of the Options

### pulldown-cmark

* Good: Fast, CommonMark-correct, event stream fits a custom renderer.
* Good: Easy to disable or ignore raw HTML.
* Neutral: No AST by default; we can build the minimal structure we need.

### markdown-it

* Good: AST available, CommonMark-compliant.
* Bad: Heavier API surface and optional features we do not need.

### markdown (markdown-rs)

* Good: Strong compliance, mdast available, supports GFM/MDX.
* Bad: Broader scope and AST shape not aligned with our minimal rendering needs.

### comrak

* Good: Robust CommonMark/GFM compliance and AST.
* Bad: Heavier dependency stack and more features than needed for roff output.
