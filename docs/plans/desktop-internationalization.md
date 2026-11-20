# Desktop internationalization

The [roadmap](../roadmap.md#desktop-internationalization) owns priority. The November 21 delivery
covers the Windows application in both Simplified Chinese and English on x64 and ARM64. Installer
dialog translation and an external translation service have separate owners.

## Delivery order

1. Use Flutter 3.50's ARB generator with the existing Chinese wording as the template and English as
   the second catalog. Add a persisted System / 简体中文 / English choice in Settings. Apply it to
   both normal startup and bootstrap recovery, alongside the existing standalone Material delegates.
   Resolve system language preferences in order, with English as the final fallback.
2. Migrate the application shell, settings, gallery, viewer, search, duplicates, albums, organization,
   file operations, confirmations, notifications, tooltips and accessibility labels. Format quantities
   and dates for the active language. Preserve navigation, selection, ongoing tasks, file paths,
   source and user-entered names. Render recorded events and their actions in the selected language.
3. Translate built-in category display names while retaining user renames and discovered names.
   Preserve classification identities, model inputs, semantic revisions and existing results.
4. Generate and check both catalogs in the existing static gate. Verify matching keys and arguments,
   complete supported languages and the absence of stray fixed copy in migrated application surfaces.
   Exercise English and Chinese through the real application entry points and normal Windows CI.

## Exit checks

- An old preferences store starts in System mode. A manual choice survives restart, and changing it
  on a populated workbench preserves the selected image and open route.
- Existing notifications change language in place, and their actions still address the original task.
  Check singular and plural counts, localized dates, menus, accessibility and narrow English layouts.
- Default category names follow the language. A saved custom name, a file path and a user album name
  retain their exact spelling. Switching languages performs no catalog writes or model analysis.
- Verify the bilingual desktop workflow with disposable data, inspect applicable exact-revision CI
  and record any outstanding evidence before accepting delivery.

The [verification record](../acceptance/desktop-internationalization.md) records client coverage
and the platforms exercised.
