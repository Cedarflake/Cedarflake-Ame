# R6 - Similar image browsing and comparison

Status: scoped execution plan; activation and acceptance belong to the roadmap.

The [roadmap](../roadmap.md#current-execution-queue) owns the current execution boundary.
The [R6 scope](../product/organization-workflows.md#r6---perceptual-similarity-review)
owns the product outcome; the [similar-image interface](../product/gallery/similar-images.md)
owns the confirmed interaction requirements.

## Delivery boundary

Add similar-image browsing inside the existing Duplicates workspace. Preserve the established
group list, member view, image viewer, source safety and ordinary Library browsing. Similarity
degree is selectable. User choices are saved in place; there is no group-completion workflow.

Planning introduces no runtime dependency, schema change or automatic similarity scanning. The
sketch is frozen; later work follows confirmed behavior and the existing design/feedback system,
with layout refinements for real workflows. Sketch matches and metadata are illustrative.

## R6-00: Confirm the interface

1. Review the right-hand group-view menu and adjacent similarity-degree icon menu on the top-level
   list, group opening, comparison, preference, member exclusion and ignore/restore. Nested pages
   omit both menus.
   Primary-destination switches reset nested pages; ordinary viewer return keeps its parent group.
2. Check Library separately: its current duplicate-display choices remain, with no similarity
   filter or folding caused by similarity.
3. Apply the confirmed degree labels/default and representative rules. Keep comparison and ignore
   actions together; omit top tabs, selection instructions and manual refresh/stop-search controls.
4. Retain the decisions in the interface contract; review production feedback against its shared
   templates without another sketch iteration.

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
- Record immutable engine/version/parameter and run evidence. Keep preferred-image, member-exclusion
  and ignored-group decisions independent of derived hashes and compatible reanalysis.
- Specify stable group/member identity and exclusion matching across regenerated groups before
  schema changes. Implement the confirmed single-representative projection without merging asset
  or location identities; use forward migrations if persistence changes.
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

Connect the group-view menu, degree changes, representative/location inspection, in-place decisions,
member exclusion/undo and comparison. View or primary-destination switches reset nested navigation
and temporary selection; durable decisions remain. Returning from comparison restores its member view.
Keyboard navigation, enlarged text and narrow windows retain the same available actions.

Exit: the real desktop path matches the accepted interface; no completion queue, next-group action,
similarity filter in Library or filesystem operation is introduced.

## R6-04: Verify the delivered result

Reuse established source, lifecycle, archive and UI fixtures where they cover the changed boundary.
Add only meaningful missing coverage under the [test-design standard](../development/testing.md).

- Exercise admitted transformations and representative false positives through the production engine.
- Verify source changes, unavailable files, placeholders, restart, cancellation and retained choices,
  including exclusions after compatible reanalysis and changes of similarity degree.
- Check large-library candidate bounds and responsiveness on the established constrained client.
- Browse groups, inspect exact-copy locations, switch views, change degree, compare images, set a
  preference, exclude/undo a member and ignore/restore in the actual Windows application. Cover
  removal of a selected/preferred member and a group falling below two members. Verify automatic
  updates and navigation resets while Library behavior, durable decisions and original media remain intact.
- Record results and remaining limits in acceptance; use normal hosted gates under the existing
  verification workflow. The roadmap owns the final exit decision.

## Remaining engineering decisions

R6-01 owns engine admission, measured transformation coverage and degree thresholds. R6-02 owns
stable group/member identity, exclusion matching and bounded automatic scheduling. Production layout
and notification details follow the interface and shared feedback contracts; none requires another sketch.
