# Prerelease compatibility cleanup

The [roadmap](../roadmap.md#prerelease-compatibility-cleanup) owns activation and delivery order.

## Outcome

Use the current Ame data and protocol contracts directly during Alpha development. Remove retired
application-version adaptations that no longer serve a supported workflow. Preserve current user
decisions, source safety, normal update retention and actionable failure handling.

The current workstation's data relocation is an explicitly authorized one-time operation outside
the product. Its script and private paths remain in ignored local storage.

## Sequence

1. Finish shared default storage and package lifecycle delivery in the Store MSIX change. Product
   startup has no legacy-directory import, persistent migration receipt or directory fallback.
2. Inventory database, background-recovery, preview, protocol and installation compatibility by
   actual caller and producer. Distinguish obsolete versions from current recovery and validation.
3. Establish the current catalog structure as a fresh-database baseline. Remove historical upgrade
   paths and fixtures that exist solely for retired Alpha schemas. Keep current-schema validation,
   transactions, durable decisions and future-version rejection. Align storage/configuration
   initialization with the same supported-version policy.
4. Remove other obsolete version adapters with their unused contracts and tests. Retain current
   source validation, renderer/platform support, file-format support and ordinary operational
   recovery. Record each retained version-related path's active purpose.
5. Update affected English, Simplified Chinese and Japanese messages and guidance together. Keep
   internal schema and migration details in diagnostics rather than ordinary workflow text.

## Evidence and boundaries

Compare fresh baseline schema, indexes, triggers and initial singleton rows against an independently
created current database. Exercise fresh creation, reopening current data, malformed/unsupported
data rejection and retained organization/file-operation state. Existing source-safe workflow tests
remain the execution owner; historical migration-only fixtures do not become a new compatibility
suite. Commit each independently reversible outcome after the Store change, then run applicable
existing hosted checks. This work does not authorize release publication or source-media changes.
