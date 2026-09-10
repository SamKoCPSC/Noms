---
name: implement-review
description: "Use when implementing a code change, bug fix, refactor, migration, or feature in a repository. Requires an implementation–review loop: understand the request, make the smallest coherent change, validate it, inspect the diff as a fresh reviewer, fix findings, and repeat until the change is clean or a concrete blocker is reported."
---

# Implement–Review Loop

Treat every implementation task as a loop, not a single coding pass. Keep the loop proportional to the change: a small fix may need one review pass; a risky or broad change may need several.

## Workflow

1. **Understand the change.** The main agent inspects the relevant files, project conventions, tests, and current git diff. Identify acceptance criteria, boundaries, likely regressions, and the narrowest validation that can prove the change.

2. **Use an isolated worktree.** Unless the user explicitly requests another workflow, make all code changes in a dedicated git worktree and branch for the task. Before creating either one, inspect the existing branches and worktrees and reuse the suitable task-specific branch and worktree when they already exist. Create only the missing branch or worktree when no suitable one exists. Do not implement directly in the canonical repository checkout. Confirm the intended base and perform every implementation, validation, review, and commit from the selected worktree. Preserve unrelated worktrees and uncommitted user changes; never repurpose or clean them without explicit permission.

3. **Write tests first.** Before prompting or delegating to an implementer, the main agent writes or updates the tests that express the requested behavior and important edge cases. Run them once to establish that they fail for the expected reason when practical. Tests are part of the main agent’s specification for the implementation.

4. **Match the main agent's model.** Use the same model as the main agent for implementer, code-review, and validation agents. Do not select a faster, cheaper, or more capable model based on task complexity. Preserve the main agent's reasoning effort unless the user explicitly requests a different effort or the environment cannot provide it.

5. **Prompt the implementer.** Give the implementer the acceptance criteria, relevant context, and the tests to satisfy. Instruct it to minimize verbosity without sacrificing correctness: keep progress updates, explanations, and the final handoff concise and high-signal, and avoid narrating routine actions. The implementer must never create, modify, delete, weaken, or skip tests. It must keep implementing production code, running the relevant tests, and correcting failures until the tests pass. If the tests appear incorrect, incomplete, or impossible to satisfy, the implementer reports that concern to the main agent instead of changing the tests.

6. **Implement in reviewable commits.** The implementer makes the smallest complete production-code change that satisfies the request and the pre-written tests, iterating until the relevant tests pass. Preserve unrelated work in the working tree. Follow existing patterns unless there is a concrete reason to change them.

   Unless the user explicitly requests a single commit or another commit strategy, build the feature as multiple small, coherent commits that a human can review independently. Each commit must have one clear purpose, include its directly related tests or supporting changes, leave the branch in a valid state, and use a descriptive commit message. Separate concerns such as preparatory refactors, behavior changes, migrations, generated artifacts, and follow-up fixes when they can stand alone. Do not split changes mechanically by file or create noisy checkpoint commits; prefer the smallest semantic units that preserve correctness. Review the staged diff before every commit and review the complete commit series before completion.

   Use descriptive, self-documenting names for variables, constants, functions, methods, types, modules, and test fixtures. Names should communicate purpose and domain meaning without requiring a comment or surrounding code to decode them. Avoid unexplained abbreviations, overly generic names, and single-letter names except for conventional short-lived indexes or similarly obvious cases. Match the project’s existing naming conventions.

   Be generous with concise, durable comments that explain why code exists, which invariant or constraint it preserves, and how it connects to callers, consumers, persistence, lifecycle events, or other parts of the system. Functions and methods should normally have a doc comment or an immediately adjacent explanatory comment, especially when they implement domain behavior, participate in a cross-component flow, encode a non-obvious decision, or have important failure or lifecycle semantics. Within non-trivial functions and methods, add short comments at meaningful transitions so a reader can follow the natural logic flow—for example, when moving from validation to normalization, persistence, side effects, or response construction. Use these comments as signposts for logical stages rather than annotations for individual statements. Omit comments only for genuinely trivial functions whose purpose and relationships are fully obvious from their name, signature, and immediate context. Comments must add information that the code cannot express clearly on its own; do not merely translate syntax, repeat names, or narrate each line. Keep the explanation proportionate and update or remove comments when the corresponding behavior changes.

7. **Run initial checks.** The main agent runs the applicable automated checks below before code review. The implementer may run them for feedback but must not edit tests. Keep runtime, browser, screenshot, design-quality, and report-generation work in Phase 2 so Phase 1 remains focused. If a check cannot run, record the exact reason and use the strongest available alternative.

   **Checks for this repository:**

   - For a broad change or before completion, run `just ci`. This is the project’s CI gate and runs formatting, WASM and server compilation checks, Clippy with warnings denied, server-feature tests in SQLx offline mode, and schema-plan script tests.
   - For a focused Rust change, run at least `just fmt`, `just check`, and `just test`; run `just lint` when the change affects production Rust code. Prefer the individual targets when diagnosing failures, but do not omit a relevant target from the final evidence.
   - For changes to `migrations/`, database access, or SQLx queries, run `just migrate-plan` when the local database/tooling is available, inspect the generated plan for data loss or compatibility risks, and run `just sqlx-prepare` when the schema or compile-time SQL queries changed. Also run `just schema-check-test` for the destructive-change guard.
   - For changes to Tailwind input or frontend assets, run `npm run tailwind:build` when generated CSS is affected, then include the generated output in review only if the project convention requires it.
   Do not treat a passing test run as sufficient when formatting, compilation, lint, schema safety, generated assets, or manual behavior is relevant to the change.

8. **Phase 1 — Code review.** After implementation and the main agent's initial checks, prompt a distinct review-specific agent for a focused, read-only code review. Give it the original request, acceptance criteria, exact review target, complete diff, relevant repository guidance, and the surrounding code needed to understand affected call paths and contracts. Define the target precisely as the working-tree changes, a commit range, or the merge-base diff against the intended base branch. Do not give this phase implementation summaries, passing validation results, browser tasks, screenshots, or report-generation work that could bias or distract defect discovery.

   The reviewer must inspect every changed file and follow affected code paths into relevant callers, consumers, tests, configuration, persistence, and error handling. Continue until every qualifying finding has been reported; do not stop after the first issue. A finding qualifies only when all of the following hold:

   - the change introduced it;
   - it is discrete and actionable rather than a general concern;
   - concrete code paths, inputs, environments, or states demonstrate its impact;
   - it does not depend on unstated assumptions about intent or unrelated code;
   - it is not clearly an intentional behavior change;
   - the author would likely fix it if informed.

   Prioritize correctness defects, behavioral regressions, security or privacy failures, data loss, broken contracts, unsafe error handling, concurrency or lifecycle faults, and missing tests for material behavior. Ignore style, formatting, naming, speculative risks, and pre-existing defects unless they obscure correctness or violate explicit repository requirements. Prove cross-component effects by identifying the affected component or path; do not report that something “may” break without tracing how.

   Report findings first, ordered by severity, with one finding per distinct issue. Use priorities `P0` through `P3`, a confidence score, and the shortest useful changed-line range that overlaps the diff, normally no more than 5–10 lines. Each finding must explain the triggering scenario and why the behavior is incorrect in one concise paragraph. Use `P0` only for unconditional release-blocking or catastrophic issues; `P1` for urgent defects; `P2` for normal actionable defects; and `P3` only for non-blocking, low-impact observations that do not make the patch incorrect.

   End with an overall correctness verdict and a short explanation. The final line must be `PASS` only when the patch is correct and no `P0`–`P2` findings remain; otherwise it must be `FAIL`. If there are no findings, state that explicitly and identify any residual static-analysis uncertainty. Do not modify files, propose unrelated improvements, perform browser validation, or produce the HTML evidence report during this phase.

9. **Phase 2 — Validate.** Start only after Phase 1 returns `PASS`. Prompt a distinct validation-specific agent to prove that the accepted patch works. Give it the original request and acceptance criteria, the reviewed diff and commit series, the main agent's check results, and the Phase 1 verdict and residual risks. This phase owns execution evidence: inspect the tests, run or independently repeat the relevant checks, exercise failure paths and important edge cases, and map each acceptance criterion to concrete evidence. A passing test suite is not sufficient when runtime behavior, generated artifacts, schema safety, accessibility, or visual behavior also matters.

   For user-facing or runtime behavior, use this repository’s `just up` recipe—or an equivalent startup command—to deploy a fresh isolated application instance on its own unused port. Do not reuse an existing app instance. Wait for infrastructure, migrations/schema application, generated artifacts, and the application server to complete successfully. Open the fresh instance in Codex’s in-app Browser and manually exercise the relevant flows and states. If authentication is required, use the mock sign-in flow with username `testuser`. If startup or browser verification cannot run, record the exact blocker, use the strongest available alternative, and do not claim the behavior was verified.

   For user-facing or visual behavior, capture screenshots that demonstrate the implemented feature in its relevant states rather than merely showing that the page loads. Assess layout, hierarchy, spacing, typography, color, interaction feedback, accessibility, and responsiveness when applicable. Also exercise relevant pre-existing flows to look for regressions.

   Create a thorough, portable HTML validation report in a temporary directory such as `/tmp`; do not place the report, screenshots, or generated report assets in the repository. Embed screenshots as data URIs/base64 assets or self-contained inline SVG so the report has no broken environment-specific links. Include the request and acceptance criteria, reviewed implementation and environment, feature walkthrough, criterion-to-evidence mapping, captioned screenshots when applicable, design-quality assessment when applicable, relevant checks and outcomes, a distinct **Existing Features and Regression Check** section, limitations, unavailable checks, residual risks, and any validation findings. Verify that the HTML opens and renders its embedded evidence.

   The validation agent must report a concise summary, blocking and should-fix findings, the check outcomes, screenshot references, and the verified report path. Its final line must be `PASS` only when every material acceptance criterion has sufficient evidence and no blocking or should-fix validation finding remains; otherwise it must be `FAIL`. The main agent sends or attaches the report and applicable screenshots in the final response.

10. **Iterate from either verdict.** If either phase returns `FAIL`, the main agent triages each actionable finding. The main agent adds or strengthens tests needed to reproduce a reported defect, close a material coverage gap, or encode clarified acceptance criteria; run those tests to establish the strongest practical baseline. Send production-code findings and the main-agent-owned tests to the implementer, which fixes production code only and must not edit the tests. Repeat the main-agent checks and always rerun Phase 1 because the patch changed. Run Phase 2 again only after the renewed code review passes. Do not declare completion while either phase is failing or a known blocking or should-fix issue remains. If a test changes the requirement rather than merely proving it, make that requirement change explicit and restart the tests-first cycle.

11. **Finish.** Stop only when both the code-review and validation phases return `PASS`, the acceptance criteria are met, validation is appropriately green, and the commit series is clean and human-reviewable. Report the worktree path and branch, ordered commits, what changed, what was checked, both phase summaries and verdicts, the validation report and screenshots when applicable, and any residual risk or unavailable check.

## Guardrails

- Do not expand the task merely because review reveals unrelated improvements. Mention unrelated issues separately.
- Do not weaken, delete, or bypass a test just to make validation pass; determine whether the implementation or test is wrong.
- For destructive operations, schema changes, auth/security behavior, or external side effects, require explicit evidence and inspect failure paths carefully.
- If requirements are genuinely ambiguous and different interpretations would materially change the implementation, ask before proceeding. Otherwise make a narrow, stated assumption.
- If blocked, provide the concrete blocker, attempted alternatives, and the smallest user decision or external change needed to continue.

## Completion report

End with a concise summary containing:

 - implementation result;
 - validation commands and outcomes;
 - review findings fixed, if any;
- evidence for completion, including relevant test coverage and screenshots sent to the user when applicable;
 - remaining limitations or risks.
