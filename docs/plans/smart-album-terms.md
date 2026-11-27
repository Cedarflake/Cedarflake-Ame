# Smart-album term editing

The [roadmap](../roadmap.md) owns delivery status. This scope adds term management through the
smart-album sidebar without restoring the retired classification settings or manual membership UI.

## Outcome and order

1. Add per-term application commands for create, edit, reset and custom-term deletion. Merge each
   reviewed term against current configuration in one transaction; unrelated changes remain usable.
   Preserve manual protection, stable IDs and compatible image features.
2. Add the sidebar edit action, an album-style tray and a separate short-form dialog. Retain the
   tray's scroll position. Localize unchanged built-in names, preserve user names, and recommend
   short English feature descriptions without restricting input languages.
3. Verify save/cancel/reset/delete, relevant conflicts, background maintenance and reusable features;
   inspect the actual client and compact/enlarged-text layout. Deliver one reversible change.

## Boundaries

Definitions may be edited before model installation. Only feature descriptions affect matching;
renaming does not replace an analysis run. Reset restores bundled content and retains protection.
Custom deletion removes the definition and its derived membership, never original images.
The fallback category stays internal. No migration of retired category-editing UI or settings is
included. Source media, model selection, per-image manual decisions and Beta publication are unchanged.

## Verification

Use the existing classification transaction, lifecycle and widget test owners for behavior checks.
Cover concurrent unrelated updates, edited-term conflict, rename-only changes, protected reset and
custom removal. Check retained rows and drafts across refresh, return position, cancellation, and
English/Chinese/Japanese at the supported compact window and enlarged text size. Avoid permanent
assertions on exact copy or spacing. Real model matching uses an isolated catalog and existing
licensed fixtures; record the observed outcome in the scoped acceptance record.
