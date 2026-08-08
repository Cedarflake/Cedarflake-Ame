# Architecture Decisions

This directory records decisions that constrain Cedarflake Ame's implementation. Product delivery
order and current progress belong in the external temporary roadmap, not in these records.

Decision statuses:

- **Proposed**: under discussion and not binding.
- **Accepted for validation**: authorized for an explicit technical gate but not yet proven.
- **Accepted**: validated and binding until replaced.
- **Superseded**: replaced by a later decision.
- **Rejected**: evaluated and not selected.

An accepted-for-validation decision must state its validation gates and replacement conditions.

Recorded decisions:

- [ADR 0001: Flutter Material 3 with a Rust application core](0001-flutter-rust-desktop-stack.md)
- [ADR 0002: Rust-owned catalog and recoverable media boundaries](0002-rust-owned-catalog-and-media-boundaries.md)
- [ADR 0003: One unified gallery with contextual actions](0003-unified-gallery-information-architecture.md)
- [ADR 0004: Admit narrow dependencies for the R0 vertical slice](0004-r0-dependency-admission.md)
- [ADR 0005: Freeze active storage and enforce a preview budget](0005-storage-governance-and-budget.md)
- [ADR 0006: Parse capture-time evidence behind an Ame metadata port](0006-exif-capture-time-adapter.md)
- [ADR 0007: Reconcile Windows locations with versioned file identity evidence](0007-windows-file-identity-reconciliation.md)
- [ADR 0008: Capture-time keyset for the unified gallery](0008-capture-time-gallery-keyset.md)
- [ADR 0009: Accepted unified gallery UI contract](0009-accepted-unified-gallery-ui-contract.md)
- [ADR 0010: Dual-level gallery time navigation](0010-dual-level-time-navigation.md)
- [ADR 0011: Bidirectional gallery keyset around time anchors](0011-bidirectional-gallery-keyset.md)
- [ADR 0012: Draw the Windows title bar inside Ame](0012-app-drawn-windows-chrome.md)
- [ADR 0013: Persist stable UI preferences outside the catalog](0013-persist-ui-preferences.md)
