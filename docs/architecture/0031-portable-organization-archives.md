# ADR 0031: Stream portable organization archives from a committed snapshot

Status: Accepted for validation.

Date: 2026-10-24.

## Context

R4 requires portable recovery of durable decisions independently of source media and rebuildable
previews. Export must remain coherent while catalog publication and user commands continue. A
catalog revision or local admission ordinal has meaning only within its original catalog; neither
is a portable identity or authority to overwrite a newer decision.

## Decision

Use an Ame-owned UTF-8 JSON Lines format, identified by `ame-organization-archive` and an integer
format version. Version 1 contains a header, ordered records and a completion footer. Record kinds,
field order and scalar types are explicit adapter contracts, independent of SQLite schema discovery.
The header records an opaque archive identifier, creation time and source schema version for
provenance. The footer contains record counts and a BLAKE3 digest of all preceding bytes, including
line endings. This detects incomplete or accidentally altered files; it is not authentication.

Read and write one record at a time with a 1 MiB encoded-line limit. Reject unsupported versions,
unknown kinds, incompatible fields, oversized lines, trailing data, missing completion and digest
or count mismatches. Format validation does not authorize restoration: relational integrity,
association evidence, revision reconciliation and atomic application remain separate restore checks.
Never accept SQL, scripts or serialized engine objects from an archive.

Export includes album definitions and retirements, lineage decisions, group mappings, bindings,
source ownership, conflicts, operation/undo provenance, the committed bookmark and its scope,
publication-order admissions and R3 representative/ignore decisions. Catalog-local source generations,
admission ordinals and location identifiers are namespaced hints within this archive. Include their
stored association evidence for subsequent reconciliation; they cannot identify a target-catalog row
by themselves. Active review sessions and their save authority are not restored. Source files,
previews, models and unrelated derived catalog records are excluded.

Take a committed SQLite online snapshot in owned derived storage using bounded page steps, then
release the live catalog copy before streaming records. Export reads no source media. The temporary
snapshot can be larger than the resulting archive; expose copy progress, support cancellation and
retain a bounded copy deadline. This cost occurs only for an explicit export, never on startup.
Write into an exclusively owned temporary file beside the selected destination, flush and validate
it, then publish without replacing an existing file. Cancellation and failures retire owned scratch
files and leave current decisions and existing destination files intact.
One application export owns admission at a time; failure or cancellation releases that admission.
The destination must be outside configured source roots, catalog/settings directories and preview
storage, including their resolved aliases.

The application owns destination admission and use-case sequencing. A narrow repository port owns
the export request and progress; the SQLite adapter owns snapshot extraction, while its archive
codec owns framing and format validation. Only typed progress and a scalar receipt cross the bridge.

## Dependencies and alternatives

Use serde 1.0.229 and serde_json 1.0.154 (MIT OR Apache-2.0), behind the archive adapter. Their typed
serialization avoids an application JSON parser and provides bounded per-record decoding. Retain
the already locked tempfile 3.27.0 (MIT OR Apache-2.0) as a production dependency for exclusive
scratch ownership and no-clobber publication. SQLite backup and BLAKE3 are existing dependencies.
Official provenance: [serde](https://crates.io/crates/serde/1.0.229),
[serde_json](https://crates.io/crates/serde_json/1.0.154), and
[tempfile](https://docs.rs/tempfile/3.27.0/tempfile/struct.NamedTempFile.html).

A whole-catalog backup preserves unrelated derived data and binds portable recovery to internal
schema. A single JSON array requires either whole-library buffering or a more involved streaming
parser. Copying live tables independently can mix different committed decisions. These alternatives
do not meet the selected boundary as directly.

## Validation and replacement

Cover populated and empty exports, Unicode, stored R3 and R4 evidence, concurrent live writes,
bounded records, cancellation, malformed/truncated files and preservation of an existing destination.
Verify the actual settings workflow before claiming user-facing export delivery. Integrated R4
verification measures temporary storage, elapsed time and memory at retained-library scale.

A later format uses another explicit version and converter; never reinterpret version 1 fields.
Replacing serialization or snapshot engines preserves the port and archived decisions. Restoration
must stage and validate before atomic publication, preserve unmatched intent and newer local choices,
and keep explicit backup application separate from automatic reindex/reassociation authority.
