# ADR 0006: Parse EXIF evidence behind Ame-owned media contracts

- Status: Accepted
- Date: 2026-08-07
- Amended: 2026-08-10

## Context

Ame needs trustworthy capture-time evidence before a date-grouped gallery or time rail can be
implemented. EXIF timestamps are not ordinary instants: `DateTimeOriginal` often contains a local
wall time without an offset, while subsecond and offset data are stored in separate fields. Treating
every value as UTC or applying the workstation's current timezone would invent information and make
the catalog unstable.

The equal-height gallery also requires display-oriented dimensions before any preview is decoded.
Many cameras store landscape pixel buffers plus EXIF Orientation rather than rewriting pixels. If
the catalog keeps encoded dimensions while the viewer applies Orientation, gallery geometry and the
opened image disagree.

Writing an EXIF/TIFF parser inside Ame would duplicate a specialized parser and expand the hostile
input surface. A parser must remain replaceable, must not leak its types into Ame's domain or schema,
and must allow an otherwise valid image to remain browsable when metadata is absent or malformed.

## Considered options

### `kamadak-exif` 0.6.1

`kamadak-exif` is a pure-Rust, BSD-2-Clause parser with a small dependency surface. The project has
existed since 2016, its repository records more than 240 commits and 250 stars, and version 0.6.1
was released on 2024-11-07. It supports JPEG, TIFF-derived formats, HEIF, PNG, and WebP containers.
The already admitted `image` 0.25 decoder API explicitly identifies it as a parser for the raw EXIF
chunks returned by `ImageDecoder::exif_metadata`.

Its release cadence is slower than newer alternatives, and its `DateTime::from_ascii` parser does
not validate calendar ranges. Ame must therefore validate every normalized calendar value at its
own boundary and retain the raw evidence.

### `nom-exif` 3.6.1

`nom-exif` is MIT-licensed, actively released, fuzz-tested, and supports a broader image, RAW,
video, and audio surface. At evaluation time its repository reports version 3.6.1, more than 490
commits, and about 117 stars. Its API is evolving quickly and most of its track and motion-photo
surface is outside the current still-image capture-time requirement. It remains the preferred
comparison candidate when R4 evaluates broader metadata and video support.

### ExifTool process adapter

ExifTool is mature and broad, but it introduces a separately packaged executable, process protocol,
and materially higher distribution surface. It remains a candidate for later comprehensive metadata
support and a reference oracle for fixed fixtures.

### Parse raw EXIF inside Ame

Rejected. The `image` crate exposes raw EXIF bytes but deliberately does not parse fields. Owning a
TIFF/EXIF parser would violate the dependency policy and add avoidable parser risk.

## Decision

Admit `kamadak-exif` 0.6.1 behind an Ame-owned `MetadataExtractor` port for the first capture-time
slice. An Ame media-inspection adapter obtains dimensions and a bounded raw EXIF chunk from the
already admitted `image` decoder without decoding pixels, then passes only those bytes to the
metadata extractor.

Image decoder allocation remains capped by the existing 256 MiB media-inspection limit. The
metadata extractor independently refuses to parse a raw EXIF block larger than 4 MiB and limits
each retained capture-time field to 64 bytes. Oversized metadata becomes a structured issue; it
does not reject the image or increase the durable evidence size without bound.

The stable Ame result records:

- the normalized local wall time at nanosecond precision;
- an optional offset in minutes, without inventing one when absent;
- whether the evidence came from `DateTimeOriginal`, `DateTimeDigitized`, or `DateTime`;
- the bounded raw date, subsecond, and offset values;
- metadata engine identity and version.

Tag priority is original capture time, digitized time, then generic image time. A missing EXIF block
is normal and produces no issue. A malformed EXIF block or chosen timestamp produces a structured
metadata issue while the image remains indexed. Source modification time stays separate and is only
a future deterministic fallback; it is never relabeled as EXIF capture time.

Third-party fields, tags, errors, and date types cannot cross the adapter. Rescans may reuse metadata
only when source size and modification time match and the stored engine identity and version match
the active adapter.

For display orientation, the admitted `image` 0.25 decoder reads EXIF Orientation and maps values
1 through 8 immediately into an Ame-owned orientation value. Catalog width and height always mean
post-orientation display dimensions; orientations 5 through 8 exchange the encoded width and
height. Missing, malformed, or unsupported orientation evidence falls back to no transform and does
not prevent the image from being cataloged.

Preview generation applies the same Ame orientation to decoded pixels before resizing and writing
the derived JPEG. The preview algorithm identity includes an orientation-aware version, so an old
sideways artifact cannot satisfy the new cache key. The metadata evidence version also includes the
Ame orientation-contract version. A complete explicit root rescan is the recovery boundary for an
existing catalog: it reinspects bounded headers, publishes corrected dimensions atomically, and
marks incompatible previews pending without deleting or modifying source media. Correcting only a
visible preview is rejected because it would leave the query-wide layout manifest on stale durable
dimensions.

## Validation gates

- fixed JPEG fixtures cover original, digitized, generic, subsecond, offset, absent, and malformed
  timestamp cases;
- fixed JPEG fixtures cover Orientation 1, 3, 6, 8, at least one mirrored orientation, invalid
  orientation fallback, output corner placement, and post-orientation dimensions;
- invalid month, day, leap-day, time, subsecond, and offset values cannot become trusted evidence;
- an oversized raw EXIF block is rejected before the parser copies or traverses it;
- metadata failure does not reject an otherwise valid image;
- schema migration preserves older locations and marks their metadata engine as unknown;
- an unchanged rescan reuses compatible evidence, while an old engine identity is reanalyzed;
- Rust format, Clippy with warnings denied, tests, Flutter analysis and tests, Windows integration,
  and a Windows Release build pass;
- controlled fixtures prove that source bytes remain unchanged.
- an existing raw-dimension catalog fixture is recovered by a complete rescan, publishes a matching
  orientation-corrected manifest ratio, and invalidates the old preview algorithm identity.

## Validation evidence

- EXIF fixtures cover original, digitized, and generic image timestamps, priority fallback,
  subseconds, offsets, missing metadata, malformed metadata, invalid calendar and clock values, and
  the raw-block size limit.
- A real JPEG adapter fixture reads dimensions and capture evidence through the admitted image
  decoder without pixel decoding and verifies byte-for-byte source preservation.
- Schema v8 creation, v7 migration, and evidence round-tripping are covered by SQLite tests. Older
  rows remain explicitly unanalyzed rather than receiving invented evidence.
- Unchanged-file scan tests prove that compatible engine evidence is reused and an old engine
  identity is reinspected before publication.
- Generated Rust and Dart bridge hashes match. Flutter mapping tests preserve engine identity,
  provenance, offset, raw evidence, and an explicitly unknown timestamp.
- Rust formatting, Clippy with warnings denied, 44 Rust tests, Flutter analysis, 17 Flutter tests,
  two Windows integration scenarios, and the Windows Release build pass.
- The Windows workflow confirms the active metadata engine through the real bridge and leaves every
  controlled source byte and directory entry unchanged.

## Consequences and risks

- R1 reads image headers and metadata but still defers pixel decoding to visible preview requests.
- Some containers may expose no raw EXIF through the admitted `image` decoder even when a different
  parser could find metadata; those items remain explicitly unknown rather than guessed.
- `kamadak-exif` runs in process because it is pure Rust and returns recoverable errors. Hostile
  fixture coverage remains mandatory; any observed process termination triggers worker isolation or
  replacement.
- This decision covers capture-time and display-orientation evidence only. It does not admit
  metadata writing, XMP, GPS, camera details, RAW decoding, or video metadata.

## Replacement strategy

Replace `kamadak-exif` behind `MetadataExtractor` and bump the engine identity or version. Existing
evidence remains traceable until reanalysis. Catalog and presentation types do not change merely
because the parser changes.
