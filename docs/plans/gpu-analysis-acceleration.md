# GPU analysis acceleration

This maintenance scope adds a persisted GPU acceleration setting, enabled by default, to the
existing smart-album analysis. It does not start R7 semantic-search implementation.

## Delivery

1. Admit the measured DirectML image encoder behind the existing isolated worker. Preserve the
   pinned CPU encoder, individual text encoding, source guards and joined cancellation. Use FP32
   computation with the existing image weights; do not use the numerically divergent FP16 GPU path.
2. Select a supported GPU by default and retain an explicit CPU choice. Give GPU features their own
   pipeline identity and replace an active analysis setup through the existing transaction owner.
   Preserve legacy decisions and historical predictions. Do not relabel old CPU features as GPU output.
3. Connect the existing Settings controls and persisted preference to worker selection through the
   current configuration-retirement lifecycle. Report the selected/active device accurately.
4. Include the fixed runtime dependencies and notices in development, portable and MSI bundles.
   Verify native output agreement, switching/restart, unavailable hardware, cancellation, packaging
   and the actual settings interaction with focused checks.

## Boundaries

No original media changes or automatic cloud hydration. New dependency and precision decisions
belong to the runtime ADR; measurements belong to acceptance evidence. Retain the completed
368-image evaluation rather than repeating unaffected model comparisons.

The settings, runtime, packaging and necessary compatibility changes form one rollback unit.
The CPU setting supplies behavioral rollback. Binary rollback retains schema-3 compatibility or
restores the installation-settings backup, as specified in ADR 0039, without deleting analysis history.
