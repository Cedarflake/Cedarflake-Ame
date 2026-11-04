# ADR 0038: Bounded perceptual similarity over admitted content

- Status: Accepted
- Date: 2026-11-04

## Context and alternatives

Perceptual comparison needs image pixels but cannot own directory discovery, source admission or
catalog policy. [Czkawka 12.0.2](https://github.com/qarmin/czkawka/releases/tag/12.0.2) uses the MIT /
Apache-2.0 `image_hasher` library behind a core pipeline that also traverses directories, opens paths
and manages caches. Its complete core would duplicate Ame's held-source and scheduling boundaries.
The GPL Krokiet interface and assets are outside this decision.

The [engine evaluation](../acceptance/r6-engine-evaluation.md) compares Gradient hashes at 64 and
256 bits, Mean/DCT at 256 bits, and BK-tree versus partitioned lookup. Gradient at 64 bits preserves
more lightly cropped candidates in that fixture. A broad BK-tree lookup approaches a full-library
walk and has no suitable interruption point inside traversal. Neither the complete core nor BK-tree
is selected for production.

## Engine and normalization

Admit `image_hasher =3.1.1`, default features disabled, behind an Ame image-analysis adapter. Pin the
existing `image` decoder to `=0.25.10` when connecting that adapter. Both are MIT OR Apache-2.0;
retain their license notices in distribution. The disabled optional fast-resize implementation is
not admitted. This combination needs no model download, new native runtime or external executable.
Pure Rust decoding retains the recoverable parser policy in [ADR 0002](0002-rust-owned-catalog-and-media-boundaries.md).

The immutable scheme `ame-perceptual-gradient64-v1` binds:

- Byte-detected image format, first-frame decoding and EXIF orientation before normalization.
- At most 256 MiB encoded input, 16,384 pixels per dimension, 16 Mi pixels and 128 MiB decoder allocation.
- Conversion to RGBA8; alpha composition on white with integer rounding; `image`'s RGB-to-luma
  conversion; aspect-preserving Triangle resampling to fit 256 by 256, including upscaling.
- `image_hasher` Gradient, 8 by 8 bits, Lanczos3 hash resampling, without DCT or optional preprocessing.
- Four quarter-turns of the grayscale sample and four of its horizontal mirror. Keep only bounded
  sample buffers during transforms. Equal variants may share one stored posting.
- No match evidence when every variant is all-zero or all-one; retain an insufficient-structure
  disposition so unchanged flat content does not enter an endless retry loop.

The comparison is the minimum Hamming distance across both images' variant sets. It is symmetric
and is never a confidence percentage or proof of exact content. The degree policy `ame-similarity-degree-v1`
uses maximum distances 2 / 6 / 10 for strict / standard / broad, initially standard. Changing degree
queries compatible evidence without reading sources again. Heavy crops, layout changes, distinct
low-detail pictures and color-only differences remain quality limits; user review retains authority.

Changing decoder behavior, normalization, hash parameters or variant semantics creates a new scheme.
Record engine/version, scheme, degree policy and immutable analysis-run identity with evidence.
Never overwrite an older run in place or reuse incompatible hashes. Third-party types stay in the
adapter; domain, persistence and bridge contracts contain Ame-owned evidence and identities.

## Held source and bounded work

The engine consumes a borrowed, seekable reader from Ame's admitted source lease. It cannot open a
path, discover siblings, hydrate a placeholder or write source bytes. Keep the file and namespace
lease through result-publication revalidation. Missing identity/revision evidence still requires
the existing exact-content check; decoding success cannot replace it.

Application owns cancellation, current root/catalog/source authority, retry and publication. Decode
checks cancellation on reads/seeks and between phases; normalization checks during pixel conversion
and between bounded transforms. A cancelled operation publishes no evidence. Background work uses
one source reader at a time and yields to the existing foreground-read admission owner. No database
transaction spans decoding. CPU-only codec/resample sections have bounded inputs but do not provide
a hard wall-clock cancellation guarantee.

Persist four disjoint 16-bit band values for each distinct 64-bit variant. For radius r, enumerate
each band's values within floor(r / 4) changed bits, then confirm the complete distance. If every
band differed by more than that bound, the full distance would exceed r. This gives complete
candidate retrieval with at most 548 indexed keys per query variant at the broad degree.

SQLite owns indexed posting lookup and keyset windows; do not retain a library-sized tree, posting
map or deduplication set in the application. Stream dense postings and distance checks in bounded,
cancellable batches. A work ceiling returns continuation or an explicit incomplete disposition,
never silently claims an exhausted search. The probe's in-memory index measures the algorithm;
it is not the production storage layout or a bound on total catalog size.

## Group and decision identity

A group has one fixed reference content and scheme. Its Ame identity derives from that scheme and
the reference's exact fingerprint scheme, digest and byte length; it never depends on location,
member order, degree or the currently displayed representative. Persist reference admission rather
than selecting a new seed on every run. Compatible reanalysis reuses existing references first.

Every member must match that fixed reference directly. Do not infer membership through another
member. Match new contents against existing references first; a content outside those groups can
establish a new reference when it has a qualifying peer. Only reference formation searches the full
content index. This avoids recording every pair inside a dense group. Groups may overlap; excluding
one member is scoped to that group.

An immutable reference can outlive its visible source location. Removed sources never appear or
count as current members. Show a group only when at least two distinct, currently published contents
remain eligible at the selected degree. Exact copies share one member key but retain their separate
assets and locations. The representative follows a valid saved preference, then a deterministic
current member; it does not replace the comparison reference.

Ignore, preference and exclusion records are durable data independent of hashes and group
projections. Bind them to the group key and, where applicable, the complete member content key.
Retain an unavailable preference without applying it to replacement bytes. Exclusion applies to all
exact copies of that member within the group. Its undo must identify the specific saved exclusion
so a stale undo cannot reverse a newer choice. Degree changes and compatible reanalysis keep these
identities. An incompatible scheme keeps prior decisions without silently transferring them to a
different group. Clearing derived data cannot delete choices; an unavailable reference may prevent
its group from being reconstructed until that content becomes available again.

Application and persistence own current-source checks and decision transactions. Hashes confer no
preference, ignore, exclusion, deletion or Library-folding authority.

## Validation and replacement

The retained probe establishes transformation behavior, candidate completeness, bounded input,
sample normalization and measured cancellation/resource cost. Production adapter integration must
retain those checks alongside held Windows source leases, stale publication, corrupt/unavailable
inputs and original-byte preservation. Persistent candidate paging needs dense and large-library
coverage; actual desktop and constrained-client behavior keep their acceptance owners.

Replace the implementation behind its analysis port. An incompatible engine creates new derived
evidence and leaves prior evidence and durable decisions intact. Disabling analysis retires workers
without deleting choices or source files. Remove the old dependency only after its last active
scheme consumer is retired; migration and cleanup cannot reinterpret old hashes as the new scheme.
