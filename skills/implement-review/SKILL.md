---
name: implement-review
description: "Use when implementing a code change, bug fix, refactor, migration, or feature in a repository. Requires an implementation–review loop: understand the request, make the smallest coherent change, validate it, inspect the diff as a fresh reviewer, fix findings, and repeat until the change is clean or a concrete blocker is reported."
---

# Implement–Review Loop

Treat every implementation task as a loop, not a single coding pass. Keep the loop proportional to the change: a small fix may need one review pass; a risky or broad change may need several.

## Workflow

1. **Understand the change.** The main agent inspects the relevant files, project conventions, tests, and current git diff. Identify acceptance criteria, boundaries, likely regressions, and the narrowest validation that can prove the change.

2. **Write tests first.** Before prompting or delegating to an implementer, the main agent writes or updates the tests that express the requested behavior and important edge cases. Run them once to establish that they fail for the expected reason when practical. Tests are part of the main agent’s specification for the implementation.

3. **Prompt the implementer.** Give the implementer the acceptance criteria, relevant context, and the tests to satisfy. The implementer must never create, modify, delete, weaken, or skip tests. If the tests appear incorrect, incomplete, or impossible to satisfy, the implementer reports that concern to the main agent instead of changing the tests.

4. **Implement.** The implementer makes the smallest complete production-code change that satisfies the request and the pre-written tests. Preserve unrelated work in the working tree. Follow existing patterns unless there is a concrete reason to change them.

   Use descriptive, self-documenting names for variables, constants, functions, methods, types, modules, and test fixtures. Names should communicate purpose and domain meaning without requiring a comment or surrounding code to decode them. Avoid unexplained abbreviations, overly generic names, and single-letter names except for conventional short-lived indexes or similarly obvious cases. Match the project’s existing naming conventions.

5. **Validate.** The main agent runs the tests and checks below. The implementer may run them for feedback but must not edit tests. If a check cannot run, record the exact reason and use the strongest available alternative.

   **Checks for this repository:**

   - For a broad change or before completion, run `just ci`. This is the project’s CI gate and runs formatting, WASM and server compilation checks, Clippy with warnings denied, server-feature tests in SQLx offline mode, and schema-plan script tests.
   - For a focused Rust change, run at least `just fmt`, `just check`, and `just test`; run `just lint` when the change affects production Rust code. Prefer the individual targets when diagnosing failures, but do not omit a relevant target from the final evidence.
   - For changes to `migrations/`, database access, or SQLx queries, run `just migrate-plan` when the local database/tooling is available, inspect the generated plan for data loss or compatibility risks, and run `just sqlx-prepare` when the schema or compile-time SQL queries changed. Also run `just schema-check-test` for the destructive-change guard.
   - For changes to Tailwind input or frontend assets, run `npm run tailwind:build` when generated CSS is affected, then include the generated output in review only if the project convention requires it.
   Do not treat a passing test run as sufficient when formatting, compilation, lint, schema safety, generated assets, or manual behavior is relevant to the change.

6. **Review independently.** Stop thinking like the implementer. The main agent re-reads the request and inspects the complete diff, including the pre-written tests. Look for:

   - missed requirements or incomplete edge cases;
   - regressions, incorrect error handling, and unsafe assumptions;
   - security, privacy, authorization, data-loss, and migration risks;
   - broken tests, stale documentation, and unnecessary scope;
   - code that does not match the repository’s conventions.

   Prefer concrete evidence from the code and validation output. Classify findings as blocking, should-fix, or informational.

   Require evidence that the feature is actually complete. Explain which tests prove each important acceptance criterion and include their relevant outcomes. For user-facing or visual behavior, capture screenshots when applicable—showing the feature working as intended in the relevant state, not merely showing that the application starts. Send or attach those screenshots to the user in the final response, with a concise caption stating what each screenshot demonstrates. If screenshots are not applicable, explain why tests or another check are sufficient.

7. **Iterate.** Fix every blocking and should-fix finding in production code, then repeat validation and review. Do not declare completion while a known blocking or should-fix issue remains. If a test must change because the requirement itself changed, the main agent makes that change explicitly and re-runs the tests-first cycle; the implementer still does not edit tests. Re-review the areas changed by each correction.

8. **Finish.** Stop when the acceptance criteria are met, validation is appropriately green, and the final review finds no blocking or should-fix issues. Report what changed, what was checked, and any residual risk or unavailable check.

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
