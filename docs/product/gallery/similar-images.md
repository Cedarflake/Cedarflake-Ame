# Similar images inside Duplicates

Status: confirmed interaction direction; detailed layout and comparison controls under review.

This scoped interface contract continues [gallery-ui](../gallery-ui.md). The
[R6 plan](../../plans/r6-similar-images.md) owns implementation slices; the
[roadmap](../../roadmap.md) owns activation and delivery status. Future controls stay absent from
the production application until their use cases are connected.

## Confirmed navigation and behavior

- Keep the existing `去重` primary destination. Its top-level modes are `重复图片` and `相似图片`.
- Each mode retains its own group results and `已忽略` category. Reuse the established group-card,
  member-list and image-viewer patterns; the type selector and result category have distinct roles.
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
- Switching modes retains their independent selection and browsing position. Navigation alone
  does not cancel analysis. Explicit task controls apply to the visible mode.

## Proposed arrangement for review

Place the two mode tabs above the current title/category/action row. The title and group count
describe the active mode. Keep the group/ignored category centered where width permits and
refresh/pause controls on the right. Similarity degree appears only in the similar-image mode.
At narrow widths, wrap the controls into clearly separated rows without horizontal clipping.

The sketch uses `严格 / 标准 / 宽松`, initially `标准`. These names and the default remain proposals;
sample membership changes illustrate the interaction and do not prescribe numerical thresholds.
Prefer a labeled degree selector over exposing hash sizes or algorithm parameters.

Group cards open full-width member rows. Each member exposes its preview, filename, dimensions,
file size and inspectable location. The proposed comparison entry selects two members and opens
a side-by-side view with optional synchronized zoom/pan. Returning preserves the member selection
and group position; no comparison action advances to another group.

Analysis activation, exact-copy representation within a similar group and additional decision
actions remain open in the execution plan. A populated sketch is not a claim that analysis ran.
