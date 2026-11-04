# Similar images inside Duplicates

Status: confirmed interaction requirements; production styling follows the existing design system.

This scoped interface contract continues [gallery-ui](../gallery-ui.md). The
[R6 plan](../../plans/r6-similar-images.md) owns implementation slices; the
[roadmap](../../roadmap.md) owns activation and delivery status. Future controls stay absent from
the production application until their use cases are connected. The completed sketch illustrates
navigation and actions only. Refine production layout against actual workflows and use the shared
[feedback templates](feedback.md#feedback-templates); the sketch's notifications are not a visual
or timing specification.

## Confirmed navigation and behavior

- Keep the existing `去重` primary destination, without top-level tabs. Put a `分组` view menu with
  a layers icon in the right-hand action area. Its mutually exclusive choices are `重复组`, `已忽略`
  and `相似组`; this changes the group view and is not labeled or depicted as a result filter.
- `已忽略` collects both kinds of ignored group; identify the kind on each card. Keep exact and
  similarity evidence and saved decisions distinct. Reuse the group-card, member-list and viewer.
- Similarity degree offers `严格 / 标准 / 宽松`, initially `标准`. A changed degree updates candidate
  results without erasing saved preferences, member exclusions or ignored decisions.
- Byte-identical copies occupy one representative slot in a similar group, with every underlying
  location inspectable. Reuse the existing exact-copy representative rules and retain distinct
  asset/location identities; this display projection does not merge catalog records.
- Browse freely: opening a group, viewing images and returning are ordinary navigation. There is
  no `完成此组`, next-group step, review-completion queue or completion percentage.
- `设为首选`, `取消首选`, `忽略此组` and `恢复此组` save in place. Ignoring a currently open group
  updates its action/state without automatically navigating away. The list reflects it on return.
- Each similar-group member exposes `移出此组` in its member row. It records a reversible exclusion
  from that group, including the exact copies represented by that row. Original files, Library
  visibility and membership in other groups are unchanged. Keep the exclusion across degree changes
  and compatible reanalysis; offer `撤销` through the shared feedback action.
- Removing a member updates the current selection and preferred representative consistently. If
  fewer than two distinct represented members remain, omit the group from results while keeping
  the open member page available for undo or return; never advance automatically to another group.
- A similarity preference is local to that similarity group. It does not merge or hide images in
  Library and does not establish byte identity or physical-operation authority.
- Library, Favorites and albums gain no similarity filter or similarity-driven folding. Existing
  exact-duplicate display behavior remains owned by the
  [current filter contract](header-and-controls.md#filter-and-exact-duplicate-behavior).
- View and similarity-degree menus belong only to the top-level list. Changing the view starts there
  with no retained member selection or nested page. Switching primary destinations also leaves the
  nested page; switching back opens the list. Saved preferences and ignored decisions remain intact.
- Discovery and result updates run automatically under bounded scheduling. Remove the workspace's
  refresh and pause/stop-search controls. Navigation and degree changes do not cancel analysis or require
  a manual scan; source-read and placeholder protections still apply.

## Controls and presentation

Keep the title/group count on the left and the group-view menu on the right, with no centered category
switch or extra navigation row. In the similar-group list, place an icon-only similarity-degree
menu with a sliders icon beside the group-view menu in the same action row. Its tooltip and accessible
name include the current degree; the menu identifies the selected option. Remove the left-hand degree
control and description. Member, viewer and comparison pages contain neither menu. Narrow widths
retain every action without horizontal clipping.

Calibrate the degree thresholds during engine evaluation. Use the product labels above rather
than hash sizes or algorithm parameters; illustrative sketch matches do not prescribe thresholds.

Group cards open full-width member rows. Each member exposes its preview, filename, dimensions,
file size and inspectable location. Put `对比图片` beside `忽略此组` / `恢复此组` in the same right-hand
action row. Remove the separate selection toolbar and its left-hand
instruction/count text. Comparison enables when two members are selected and opens a side-by-side
view with optional synchronized zoom/pan. Ordinary return from comparison preserves that member
view; changing primary destinations instead resets it. No comparison action advances to another group.

A populated sketch is not a claim that automatic analysis ran. The execution plan owns engine
evaluation, durable exclusion identity and verification of the real interaction.
