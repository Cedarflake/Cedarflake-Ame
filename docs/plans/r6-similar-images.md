# R6 - Similar image browsing and comparison

Status: interaction planning; implementation activation and acceptance belong to the roadmap.

The [roadmap](../roadmap.md#current-execution-queue) currently selects the interaction plan and
clickable sketch only. The [R6 scope](../product/organization-workflows.md#r6---perceptual-similarity-review)
owns the product outcome; the [similar-image interface](../product/gallery/similar-images.md)
owns the confirmed interaction direction and the remaining design choices.

## Delivery boundary

Add similar-image browsing inside the existing Duplicates workspace. Preserve the established
group list, member view, image viewer, source safety and ordinary Library browsing. Similarity
degree is selectable. User choices are saved in place; there is no group-completion workflow.

The current deliverable is a reviewable plan and interactive sketch. It does not introduce a
runtime dependency, change the schema, activate automatic similarity scanning or implement R6.
Sketch matches and image metadata are illustrative, not algorithm-quality evidence.

## R6-00: Confirm the interface

1. Preview the right-hand group-view menu and adjacent similarity-degree icon menu on the top-level
   list, group opening, comparison, preference and ignore/restore. Nested pages omit both menus.
   Primary-destination switches reset nested pages; ordinary viewer return keeps its parent group.
2. Check Library separately: its current duplicate-display choices remain, with no similarity
   filter or folding caused by similarity.
3. Confirm the proposed degree labels/default and comparison layout. Keep comparison and ignore
   actions together; omit top tabs, selection instructions and manual refresh/stop-search controls.
4. Incorporate review into the owning interface contract before product implementation.

Exit: the interaction choices are explicit, including anything deferred. A clickable sketch is
not implementation or acceptance evidence.

## R6-01: Admit an engine and comparison boundary

Depends on R6-00. Evaluate the MIT `czkawka_core` similarity capability against a narrow adapter
using the same ecosystem's perceptual-hash and candidate-search libraries. The
[R3 decision](../architecture/0028-versioned-exact-content-analysis.md) retained Czkawka as a future
candidate; it did not admit that dependency for R6.

- Start with reviewed Czkawka 12.0.2 source; recheck upstream, dependency licenses, Windows packaging
  and version pins at admission. Do not copy the GPL Krokiet UI or its assets.
- Verify that the selected boundary accepts Ame-admitted content without another source-directory
  scan, path reopening outside the held-source contract, or placeholder hydration.
- Compare recompression, resize, quarter-turn rotation, mirror, crop and watermark cases, plus
  visually similar but unrelated negatives. Record supported transformation limits and the
  precision/recall tradeoff of each degree. Do not label raw hash distance as a confidence percentage.
- Measure candidate/index memory, work growth, large-image decode limits and cancellation with a
  bounded fixture. Avoid unbounded all-pairs comparison and chains that obscure why members match.
- Resolve only uncertain integration questions with a focused prototype, then choose one engine
  boundary. Record the decision, parameters, compatibility and replacement strategy in an ADR.

Exit: one admitted approach, meaningful quality evidence and bounded integration cost.

## R6-02: Persist evidence and connect bounded analysis

Depends on the admitted R6-01 contract. Initial discovery and incremental updates are automatic;
the workspace does not require a manual start, refresh or stop-search action.

- Use Ame-owned domain values and ports under ADR 0037. Keep engine types and decoding/index
  implementation in adapters; application owns scheduling, source admission and result publication.
- Read bounded candidates from the published catalog. Revalidate source identity, revision and
  generation before publishing; changed or removed files invalidate derived results.
- Record immutable engine/version/parameter and run evidence. Keep preferred-image and ignored-group
  decisions independent of derived hashes and compatible reanalysis.
- Specify stable group/member identity and exact-duplicate representation before schema changes;
  use forward migrations if persistence changes. Similarity preference has no Library folding effect.
- Bound automatic scheduling, worker lifetime, memory and read concurrency. Retain cancellation,
  restart/retry and source-state revalidation at the application boundary; keep available results
  during updates. View and degree changes must not restart a full analysis or bypass source admission.

Exit: analysis and durable choices work through production contracts, including stale and cancelled
completion. Scheduling behavior is explicit rather than inferred from the sketch.

## R6-03: Connect the accepted browsing interface

Depends on R6-02 and the confirmed R6-00 layout.

Reuse existing workspace, group/member, preview, viewer, loading and feedback owners. Extract a
shared responsibility where needed instead of adding similarity flags throughout the exact-duplicate
controller. Keep analysis, query state and user choices separately owned for each mode.

Connect the three-choice group-view menu, degree changes, bounded queries, in-place decisions and
comparison. View or primary-destination switches reset nested navigation and temporary selection;
saved preferences and ignored records remain. Returning from comparison restores its member view.
Keyboard navigation, enlarged text and narrow windows retain the same available actions.

Exit: the real desktop path matches the accepted interface; no completion queue, next-group action,
similarity filter in Library or filesystem operation is introduced.

## R6-04: Verify the delivered result

Reuse established source, lifecycle, archive and UI fixtures where they cover the changed boundary.
Add only meaningful missing coverage under the [test-design standard](../development/testing.md).

- Exercise admitted transformations and representative false positives through the production engine.
- Verify source changes, unavailable files, placeholders, restart, cancellation and retained choices.
- Check large-library candidate bounds and responsiveness on the established constrained client.
- Browse groups, switch group views, change degree, compare images, set a preference
  and ignore/restore in the actual Windows application. Verify automatic updates and navigation
  resets while ordinary Library behavior, durable decisions and original media are preserved.
- Record results and remaining limits in acceptance; use normal hosted gates under the existing
  verification workflow. The roadmap owns the final exit decision.

## Decisions still open

| Decision | Current proposal |
| --- | --- |
| Degree labels and initial value | Sketch uses `严格 / 标准 / 宽松`, initially `标准`; confirm wording/default, then calibrate thresholds in R6-01. |
| Comparison interaction | Select two members, open side-by-side comparison, optionally synchronize zoom/pan, and return to the same group. |
| Exact copies within similar groups | Evaluate one logical representative with inspectable locations; do not decide this from mock data. |
| Additional decisions | Pair/member exclusion and other actions require a distinct user need; they are absent from the initial sketch. |
