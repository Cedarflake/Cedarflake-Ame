# Desktop internationalization

The [roadmap](../roadmap.md#desktop-internationalization) owns priority. The November 21 delivery
covers the Windows application in Simplified Chinese, English and Japanese on x64 and ARM64. Installer
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
3. Translate all application-managed smart-album names by stable category ID, including the broad
   purposes and supported curated topic/purpose pairs. Smart albums have no rename action.
   Preserve classification identities, model inputs, semantic revisions and existing results.
4. Generate and check all shipped catalogs in the existing static gate. Verify matching keys and arguments,
   complete supported languages and the absence of stray fixed copy in migrated application surfaces.
   Exercise English and Chinese through the real application entry points and normal Windows CI.
5. Add the Japanese catalog and the 日本語 option in Settings. Recognize Japanese Windows language
   preferences, keep English as the final fallback, and exercise switching with a populated library
   and an existing notification. Identify Simplified Chinese by script (`zh-Hans`) throughout the
   application and README links. Link the English, Simplified Chinese and Japanese READMEs
   from the centered language selector on each page.
6. Qualify translated layouts across the workbench, Settings, filters, metadata, dialogs and viewer.
   Inspect fixed text widths/heights and adjacent spacing, then reproduce suspect cases at the
   supported minimum window size and enlarged text in all three languages. Repair demonstrated
   overlap, clipping and alignment failures while retaining useful icon, target and spacing tokens.
   Check preview feedback alongside these surfaces: persistent conditions remain until handled or
   acknowledged, and ordinary outcomes use the shared transient lifetime.

## Exit checks

- With no saved language, the app follows the system preference. A manual choice survives restart.
  Changing it on a populated workbench preserves the selected image and open route.
- Existing notifications change language in place, and their actions still address the original task.
  Check singular and plural counts, localized dates, menus, accessibility and narrow English layouts.
- Translated labels and values have visible separation, common columns remain aligned, and actions
  stay reachable at 150% text. A preview notification without a recovery action can be acknowledged;
  transient outcomes disappear through their existing five-second owner.
- Automatic smart-album names follow the language by stable ID; an unknown ID displays its returned
  label. File paths and user album names retain their exact spelling. Switching languages performs
  no catalog writes or model analysis.
- Verify the desktop language workflow in all three languages with disposable data, inspect
  applicable exact-revision CI and record any outstanding evidence before accepting delivery.

The [verification record](../acceptance/desktop-internationalization.md) records client coverage
and the platforms exercised.
