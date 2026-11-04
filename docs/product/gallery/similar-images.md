# Similar images inside Duplicates

Status: confirmed interaction direction; detailed layout and comparison controls under review.

This scoped interface contract continues [gallery-ui](../gallery-ui.md). The
[R6 plan](../../plans/r6-similar-images.md) owns implementation slices; the
[roadmap](../../roadmap.md) owns activation and delivery status. Future controls stay absent from
the production application until their use cases are connected.

## Confirmed navigation and behavior

- Keep the existing `去重` primary destination, without top-level tabs. Put a `筛选` menu in the
  right-hand action area with mutually exclusive `重复组`, `已忽略` and `相似组` choices.
- `已忽略` collects both kinds of ignored group; identify the kind on each card. Keep exact and
  similarity evidence and saved decisions distinct. Reuse the group-card, member-list and viewer.
- Similarity degree is selectable inside the similar-image mode. A changed degree updates candidate
  results without erasing saved preferences or ignored decisions.
- Browse freely: opening a group, viewing images and returning are ordinary navigation. There is
  no `完成此组`, next-group step, review-completion queue or completion percentage.
- `设为首选`, `取消首选`, `忽略此组` and `恢复此组` save in place. Ignoring a currently open group
  updates its action/state without automatically navigating away. The list reflects it on return.
- A similarity preference is local to that similarity group. It does not merge or hide images in
  Library and does not establish byte identity or physical-operation authority.
- Library, Favorites and albums gain no similarity filter or similarity-driven folding. Existing
  exact-duplicate display behavior remains owned by the
  [current filter contract](header-and-controls.md#filter-and-exact-duplicate-behavior).
- Changing the result filter from a member page returns to the selected result list, clears temporary
  member selection and resets browsing position. Switching primary destinations also leaves the
  nested page; switching back opens the list. Saved preferences and ignored decisions remain intact.
- Discovery and result updates run automatically under bounded scheduling. Remove the workspace's
  refresh and pause/stop-search controls. Navigation and filtering do not cancel analysis or require
  a manual scan; source-read and placeholder protections still apply.

## Proposed arrangement for review

Keep the title/group count on the left and the filter menu on the right, with no centered category
switch or extra navigation row. The filter remains available on member pages. Similarity degree
appears only in the similar-group list. Narrow widths retain every action without horizontal clipping.

The sketch uses `严格 / 标准 / 宽松`, initially `标准`. These names and the default remain proposals;
sample membership changes illustrate the interaction and do not prescribe numerical thresholds.
Prefer a labeled degree selector over exposing hash sizes or algorithm parameters.

Group cards open full-width member rows. Each member exposes its preview, filename, dimensions,
file size and inspectable location. Put `对比图片` beside `忽略此组` / `恢复此组` in the same right-hand
action row, followed by the filter menu. Remove the separate selection toolbar and its left-hand
instruction/count text. Comparison enables when two members are selected and opens a side-by-side
view with optional synchronized zoom/pan. Ordinary return from comparison preserves that member
view; changing the result filter instead resets it. No comparison action advances to another group.

Exact-copy representation within a similar group and additional decision actions remain open in the
execution plan. A populated sketch is not a claim that automatic analysis ran.
