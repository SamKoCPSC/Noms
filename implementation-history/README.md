# Implementation History

This folder serves as a living knowledge base for the Noms project, documenting the "how" and "why" behind complex implementations, difficult bugs, and architectural decisions.

## Purpose

While the main documentation (README, code comments) describes *what* the code does, this folder records *how* we got there. It captures the trial-and-error process, dead ends, and hard-won insights that are often lost during refactoring or when developers leave the project.

## What to Record

- **Complex Bugs:** Issues that took significant time to debug, especially those with non-obvious root causes.
- **Architectural Decisions:** Explanations for why a specific pattern or library was chosen over alternatives.
- **Framework Quirks:** Workarounds for unexpected behavior in Dioxus, Rust, or other dependencies.
- **Performance Optimizations:** Before/after metrics and the specific changes that led to improvements.

## Format

Entries should be written in Markdown and follow this structure:

```markdown
# [Title of Issue/Feature]

**Date:** YYYY-MM-DD
**Issue:** [Ticket ID]
**Status:** Resolved / Ongoing

## Summary
Brief overview of the problem and the final solution.

## The Journey
Chronological account of attempts, failures, and discoveries.

## Final Solution
Code snippets and explanation of the working implementation.

## Lessons Learned
Key takeaways for future reference.
```

## Current Entries

- [dark-mode.md](./dark-mode.md) — Dioxus 0.7 theme toggle, SSR panics, and `document::eval` pitfalls.
