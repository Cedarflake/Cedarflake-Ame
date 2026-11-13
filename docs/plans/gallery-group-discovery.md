# Gallery group discovery follow-up

Status: scoped execution plan; disposition is owned by the [roadmap](../roadmap.md#gallery-group-discovery).

## Delivery units

1. Restore scope-consistent semantic results. Retain all qualifying matches with fixed relevance
   admission, stable order and paged photo details. Verify the reported `猫咪` query in the complete
   library and a narrower folder, including access beyond the previous 500-result limit.
2. Add ordinary-gallery group filters: unrestricted, exact groups only, or similar groups only.
   They are mutually exclusive. An active group filter forces grouped display and disables the
   separate merge toggle; clearing it restores the previously selected display mode.
   Apply scope and group membership before counts, paging, timeline and full-selection resolution.
3. Connect gallery group cleanup and member actions. Add explicit duplicate/similar cleanup actions
   alongside ordinary deletion and expose preferred/member selection in the existing bottom tray.
   Reuse exact and similar keep rules, frozen file selection, confirmation and Windows execution.
   Ordinary merged-card deletion retains its existing whole-group meaning.
4. Label the duplicate workspace's group-type menu with its current type. Reuse its menu motion,
   icon family and action placement; keep unrelated layout and navigation unchanged.

Each unit includes implementation, relevant owner documentation, focused checks, client interaction,
and its own commit/push. Search can be delivered independently of the gallery group controls.

## Boundaries and verification

Keep relevance, exact byte identity and perceptual group membership distinct. Semantic search does
not authorize cleanup. Reuse the existing analysis engines, schema identities and file-operation
authority; no source mutation is needed to verify search or filtering.

The result manifest retains compact source identities; embedding reads and displayed photo details
remain windowed. Measure the actual result count and query cost rather than replacing the old count
cap with a larger cap. Check broader/narrower scopes, empty results, cancellation and existing
duplicate-folding behavior through their current owners.

For group UI, verify both group types, return to unrestricted display, selection after paging and
tray close/reopen. Destructive execution uses disposable fixtures; real-library verification stops
before confirmation. Keep large controllers as composition facades and place query/group policy in
typed application/domain owners under ADR 0025 and ADR 0037.

Evidence belongs in the relevant R7 search and R8 cleanup acceptance records. This follow-up does
not start the separately planned R10 Beta release work.
