# MSI localization

The [roadmap](../roadmap.md) owns disposition. Extend the native installer under
[ADR 0027](../architecture/0027-windows-installer-lifecycle.md) with Simplified Chinese,
English and Japanese, selected from the invoking user's Windows display-language preferences.
Unsupported preferences fall back to English. Preserve the same single MSI and Setup entrypoints,
installation choices, upgrade identities, repair and optional uninstall data removal.

1. Author product strings in WiX localization catalogs and retain WiX's stock dialog, error and
   progress translations. Build all three catalogs from identical installation inputs.
2. Package a bounded presentation-only catalog in a language-neutral MSI. An immediate native
   owner populates temporary presentation rows before either UI or execute sequences consume them.
   Preserve action/event semantics and never apply runtime transforms or modify the cached package.
3. Exercise direct MSI and cached maintenance, language fallback, errors and selected cleanup
   options. Verify native layout in all three languages with disposable installation data.
4. Commit and push the change with its affected packaging checks and acceptance evidence.

The verification owner is [installer acceptance](../acceptance/unsigned-windows-release.md).
This work does not authorize a release or change the separate Beta candidate gate.
