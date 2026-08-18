# Architecture Decisions

This directory records decisions that constrain Cedarflake Ame's implementation. Product delivery
order and current progress belong in the repository-owned `docs/roadmap.md`, not in these records.

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
- [ADR 0005: Govern storage and the bounded preview lifecycle](0005-storage-governance-and-budget.md)
- [ADR 0006: Parse EXIF evidence behind Ame-owned media contracts](0006-exif-capture-time-adapter.md)
- [ADR 0007: Reconcile Windows locations with versioned file identity evidence](0007-windows-file-identity-reconciliation.md)
- [ADR 0008: Capture-time keyset for the unified gallery](0008-capture-time-gallery-keyset.md)
- [ADR 0009: Accepted unified gallery UI contract](0009-accepted-unified-gallery-ui-contract.md)
- [ADR 0010: Dual-level gallery time navigation](0010-dual-level-time-navigation.md)
- [ADR 0011: Bidirectional gallery keyset around time anchors](0011-bidirectional-gallery-keyset.md)
- [ADR 0012: Draw the Windows title bar inside Ame](0012-app-drawn-windows-chrome.md)
- [ADR 0013: Persist stable UI preferences outside the catalog](0013-persist-ui-preferences.md)
- [ADR 0014: Query-wide gallery layout manifest and unified navigation](0014-query-wide-gallery-layout-manifest.md)
- [ADR 0015: Versioned Windows x64 portable distribution](0015-windows-release-distribution.md)
- [ADR 0016: Normalize continuous library changes before reconciliation](0016-continuous-library-synchronization-contracts.md)
- [ADR 0017: Validate notify 8.2.0 behind the Windows change-source adapter](0017-notify-windows-change-source.md)
- [ADR 0018: Persist normalized library changes in a leased SQLite queue](0018-durable-library-change-queue.md)
- [ADR 0019: Publish incremental catalog deltas atomically](0019-atomic-incremental-catalog-deltas.md)
- [ADR 0020: Run continuous library synchronization with the desktop lifecycle](0020-production-library-synchronization-lifecycle.md)
- [ADR 0021: Recover library freshness through bounded authoritative reconciliation](0021-authoritative-library-recovery-and-consistency.md)
- [ADR 0022: Catch up Windows downtime through the USN change journal](0022-windows-usn-downtime-catch-up.md)
