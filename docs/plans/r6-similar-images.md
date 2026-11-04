# R6 - Similar image browsing and comparison

Status: working implementation plan, not a binding product specification.

This file holds R6's agreed planning direction, proposed implementation and unresolved details.
The [roadmap](../roadmap.md#current-execution-queue) owns activation and acceptance. These planning
choices do not change current product contracts; update those alongside reviewed implementation.

## Delivery boundary

Add similar-image browsing inside the existing Duplicates workspace. Preserve the established
group list, member view, image viewer, source safety and ordinary Library browsing. Similarity
degree is selectable. User choices are saved in place; there is no group-completion workflow.

Planning introduces no runtime dependency, schema change or automatic similarity scanning. Sketch
iteration has ended. Its navigation examples inform this plan, while production layout and especially
notifications will be refined in the actual application using the existing
[feedback templates](../product/gallery/feedback.md#feedback-templates). Sketch matches, metadata,
notification styling and timers are illustrative.

## R6-00: Interaction planning

The planning choices below guide implementation; detailed styling and notifications remain adjustable.

| Area | Planned direction |
| --- | --- |
| Entry | Keep `去重`; omit top tabs. A right-hand `分组` menu with a layers icon switches between `重复组`, `已忽略` and `相似组`. |
| Similarity degree | Offer `严格 / 标准 / 宽松`, initially `标准`, through a sliders-icon menu beside `分组` in the same action row. Show it only in the similar-group list, with the current degree in its tooltip and accessible name. |
| Nested pages | Member, viewer and comparison pages omit both menus. Primary-destination switches reset nested navigation and temporary selection; ordinary return from viewing/comparison preserves its parent group. |
| Browsing | Reuse group cards, member rows and image viewing. Browse freely, without group completion, a next-group step or review-progress tracking. |
| Comparison | Select two members; put `对比图片` beside `忽略此组` / `恢复此组`. Omit the separate selection toolbar and its left-hand instruction/count. Plan side-by-side viewing with optional synchronized zoom/pan. |
| Exact copies | Show one representative for byte-identical copies within a similar group and retain access to all locations. Reuse existing representative rules without merging asset/location identities. |
| Unrelated members | Add a member-row action to exclude that image from the current similar group; proposed label: `移出此组`. This does not change the original file, Library visibility or other groups. |
| Saved choices | Preserve preferences, ignored groups and member exclusions across degree changes and compatible reanalysis. A similarity preference does not fold or hide images in Library. |
| Ignored groups | List ignored exact and similar groups together, identify the kind on each card, and support restoration. Ignore/restore saves in place without automatically leaving the open group. |
| Automatic work | Discovery and result updates run automatically; remove manual refresh and pause/stop-search controls from this workspace. Navigation does not cancel analysis. |
| Library | Add no similarity filter or similarity-driven folding to Library, Favorites or albums; existing exact-duplicate display remains separate. |

Exit: planning choices and deferred details are recorded here. The sketch is not implementation or
product acceptance evidence, and no further sketch iteration is planned.

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

## R6-03: Implement the planned browsing interface

Depends on R6-02 and the R6-00 planning direction.

Reuse existing workspace, group/member, preview, viewer, loading and feedback owners. Extract a
shared responsibility where needed instead of adding similarity flags throughout the exact-duplicate
controller. Keep analysis, query state and user choices separately owned for each mode.

Connect the group-view menu, degree changes, representative/location inspection, in-place decisions,
member exclusion/undo and comparison. View or primary-destination switches reset nested navigation
and temporary selection; durable decisions remain. Returning from comparison restores its member view.
Keyboard navigation, enlarged text and supported desktop window sizes retain the same available actions.

Review the real desktop path against the plan, then document the implemented behavior in its product
owner. Introduce no completion queue, next-group action, Library similarity filter or filesystem operation.

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
stable group/member identity, exclusion matching and bounded automatic scheduling.

Resolve member-exclusion details during implementation: scope when one row represents exact copies,
undo through shared feedback, selected/preferred-member removal, and groups with fewer than two
members. Preserve original files and durable intent while keeping ordinary return navigation available.
Production styling and notification behavior will be reviewed in the real application, without
treating the sketch as a final specification.
