---
name: implement-review
description: "Use when implementing a code change, bug fix, refactor, migration, or feature in a repository. Requires an implementation–review loop: understand the request, make the smallest coherent change, validate it, inspect the diff as a fresh reviewer, fix findings, and repeat until the change is clean or a concrete blocker is reported."
---

# Implement–Review Loop

Treat every implementation task as a loop, not a single coding pass. Keep the loop proportional to the change: a small fix may need one review pass; a risky or broad change may need several.

## Workflow

1. **Understand the change.** The main agent inspects the relevant files, project conventions, tests, and current git diff. Identify acceptance criteria, boundaries, likely regressions, and the narrowest validation that can prove the change.

2. **Write tests first.** Before prompting or delegating to an implementer, the main agent writes or updates the tests that express the requested behavior and important edge cases. Run them once to establish that they fail for the expected reason when practical. Tests are part of the main agent’s specification for the implementation.

3. **Select proportionate models.** When model overrides are available, choose a model and reasoning effort appropriate to the task’s complexity, uncertainty, and risk. Use a faster or less capable model for small, well-scoped, low-risk changes; use a stronger model for broad, novel, security-sensitive, migration-heavy, or difficult debugging work. The review agent should be at least as capable as the implementer when the change is complex or high-risk. Do not request model names that are unavailable in the current Codex environment; use the closest available option and record the choice when relevant.

4. **Prompt the implementer.** Give the implementer the acceptance criteria, relevant context, and the tests to satisfy. The implementer must never create, modify, delete, weaken, or skip tests. It must keep implementing production code, running the relevant tests, and correcting failures until the tests pass. If the tests appear incorrect, incomplete, or impossible to satisfy, the implementer reports that concern to the main agent instead of changing the tests.

5. **Implement.** The implementer makes the smallest complete production-code change that satisfies the request and the pre-written tests, iterating until the relevant tests pass. Preserve unrelated work in the working tree. Follow existing patterns unless there is a concrete reason to change them.

   Use descriptive, self-documenting names for variables, constants, functions, methods, types, modules, and test fixtures. Names should communicate purpose and domain meaning without requiring a comment or surrounding code to decode them. Avoid unexplained abbreviations, overly generic names, and single-letter names except for conventional short-lived indexes or similarly obvious cases. Match the project’s existing naming conventions.

6. **Validate.** The main agent runs the tests and checks below. The implementer may run them for feedback but must not edit tests. If a check cannot run, record the exact reason and use the strongest available alternative.

   **Checks for this repository:**

   - For a broad change or before completion, run `just ci`. This is the project’s CI gate and runs formatting, WASM and server compilation checks, Clippy with warnings denied, server-feature tests in SQLx offline mode, and schema-plan script tests.
   - For a focused Rust change, run at least `just fmt`, `just check`, and `just test`; run `just lint` when the change affects production Rust code. Prefer the individual targets when diagnosing failures, but do not omit a relevant target from the final evidence.
   - For changes to `migrations/`, database access, or SQLx queries, run `just migrate-plan` when the local database/tooling is available, inspect the generated plan for data loss or compatibility risks, and run `just sqlx-prepare` when the schema or compile-time SQL queries changed. Also run `just schema-check-test` for the destructive-change guard.
   - For changes to Tailwind input or frontend assets, run `npm run tailwind:build` when generated CSS is affected, then include the generated output in review only if the project convention requires it.
   - Before using `just up` for UI verification or screenshots, first check whether the app is already running on port `8080` (for example, with `curl` or a port-listening check). Reuse the existing app when it is available; run `just up` only when port `8080` is not serving the app. Avoid starting a duplicate instance.
   Do not treat a passing test run as sufficient when formatting, compilation, lint, schema safety, generated assets, or manual behavior is relevant to the change.

7. **Prompt a separate review agent.** The main agent must prompt a distinct review-specific agent after implementation and validation. Give it the original request, acceptance criteria, pre-written tests, validation results, and complete implementation diff. The review agent must independently inspect the implementation and tests rather than relying only on the implementer’s explanation. Look for:

   - missed requirements or incomplete edge cases;
   - regressions, incorrect error handling, and unsafe assumptions;
   - security, privacy, authorization, data-loss, and migration risks;
   - broken tests, stale documentation, and unnecessary scope;
   - code that does not match the repository’s conventions.

   Prefer concrete evidence from the code and validation output. Classify findings as blocking, should-fix, or informational.

   Before manual review, check whether the application is already serving on port `8080`. If it is, reuse that running instance; otherwise use this repository’s `just up` recipe to deploy and start the application. Wait for infrastructure, migrations/schema application, generated artifacts, and the application server to complete successfully. Then open the local application in the browser and manually exercise the relevant user flows and states. If the port check, `just up`, or browser setup cannot run, record the exact blocker and use the strongest available alternative; do not claim browser-based behavior was verified.

   Require evidence that the feature is actually complete. Explain which tests prove each important acceptance criterion and include their relevant outcomes. For user-facing or visual behavior, capture screenshots when applicable—showing the feature working as intended in the relevant state, not merely showing that the application starts. The screenshots must also provide evidence that the implementation is visually coherent, polished, and appropriate for the product: inspect layout, hierarchy, spacing, typography, color, states, responsiveness, and any relevant interaction feedback. Do not treat a screenshot of a page merely loading as evidence of feature completion or design quality.

   During the review, create a thorough, portable HTML review report in a temporary directory such as `/tmp`; do not create the report, screenshots, or generated report assets in the repository working directory. The report must render its screenshots directly in the document rather than relying on broken or environment-specific links: embed each image as a data URI/base64 asset, or use self-contained inline SVG where appropriate. Include the original request and acceptance criteria, the implementation and environment reviewed, a concise feature walkthrough, one or more captioned screenshots showing explicit evidence of the implemented behavior, a separate design-quality assessment grounded in the screenshots, and a table of relevant passing tests/checks with commands and outcomes. After demonstrating that the implemented feature works, include a distinct **Existing Features and Regression Check** section. Exercise the relevant pre-existing user flows and checks, document which existing features were verified, include their passing test/manual evidence, and explicitly state whether any regressions were found. Include limitations, unavailable checks, residual risks, and any blocking or should-fix findings. Write the report to a clearly identified temporary artifact path that the main agent can provide to the user, and verify that the HTML opens and renders the embedded images successfully before reporting it.

   Send or attach the report and, when the interface supports it, the individual screenshots to the user in the final response. Include concise captions stating what each screenshot demonstrates. If screenshots are not applicable, explain why tests or another check are sufficient, but still create the HTML report when the review has meaningful evidence to present.

   The review agent must report back to the main agent with:

   - a concise review summary;
   - blocking, should-fix, and informational findings, if any;
   - evidence supporting completion, including check outcomes, screenshot references, and the verified path to the portable HTML report;
   - an explicit verdict on its own final line: `PASS` only when no blocking or should-fix issues remain, otherwise `FAIL`.

8. **Iterate from the verdict.** If the review agent returns `FAIL`, the main agent sends the implementation findings to the implementer, which fixes production code only. Then repeat validation and prompt the separate review agent again. Do not declare completion while the verdict is `FAIL` or a known blocking or should-fix issue remains. If a test must change because the requirement itself changed, the main agent makes that change explicitly and re-runs the tests-first cycle; the implementer still does not edit tests. Re-review the areas changed by each correction.

9. **Finish.** Stop only when the review agent returns `PASS`, the acceptance criteria are met, and validation is appropriately green. Report what changed, what was checked, the review agent’s summary and verdict, and any residual risk or unavailable check.

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
