# Foreground image scheduling

Scope: Windows engine image I/O priority, requested on 2026-10-22. The separate
five-minute Release observation is deferred. No source mutation, hydration,
catalog reset, decoder replacement or import optimization is included.

## Diagnosis

On the two-core, 8 GiB cloud-primary device, a slow Debug open spent 0.533 seconds
decoding, then 5.464 seconds waiting before a 0.028-second graphics upload.
Switching the same Debug bundle from Impeller to Skia did not remove the delay.

A same-process Skia comparison opened the same JPEG and PNG through the Gallery
handler, recording image-ready time and sampling the image I/O thread:

| I/O priority | JPEG | PNG | Observed waiting |
| --- | ---: | ---: | --- |
| Below normal | 12.186 s | 12.345 s | 24 ready-to-run samples |
| Normal | 1.179 s | 0.461 s | No ready-to-run samples |
| Restored below normal | 16.671 s | 11.143 s | 28 ready-to-run samples |

The adjustment was restored after the comparison. These are diagnostic samples,
not a percentile guarantee or validation of the deferred Release observation.

## Implementation and verification

The Windows runner owns one synchronous startup adjustment under
[ADR 0001](../architecture/0001-flutter-rust-desktop-stack.md#windows-image-io-scheduling).
The adapter has 102 implementation lines and a 29-line header; its dedicated
policy fixture has 88 lines. The 158-line window facade only composes the
observation, engine construction and result diagnostic. The real-engine fixture
has 179 lines, including its independent thread-identity check.

Local native verification passed with the installed Flutter 3.44.9 engine
`5a2a6a42cce67f965cf540fcecf616faca624aa1`. Both actual-engine lifecycle cases
independently identify `io.flutter.io` and require normal priority. Their boundary
fixtures preserve existing and above-normal workers, refuse two ambiguous
background candidates, ignore retired workers, and avoid repeated elevation.
Explicit destruction and scope-exit retirement both pass without access violations.

The result parser also requires the adjusted thread ID to equal the independent
engine-name observation. Its guardrails reject a mismatched ID, a missing
adjustment and a non-normal priority. The parser passed against the actual local
engine report and its positive/negative fixtures.

## Cloud application verification

The four production runner files were applied to the existing cloud-primary
checkout after patch validation, with the old source and executable retained in
ignored derived storage. The existing unrelated `analysis_options.yaml` and
`pubspec.lock` changes were preserved. The native runner was rebuilt and the
existing Debug application restarted with its default Impeller renderer.
Flutter was 3.47.7 on this device; the local/hosted pin remains 3.44.9.

The application startup diagnostic and native process inspection both reported
normal priority without a manual adjustment. Through the production Gallery
open handler, each image was closed and reopened three times:

| Image | Image-ready times | Following-frame times |
| --- | --- | --- |
| JPEG, 4000 x 2250 | 1.627, 1.567, 1.308 s | 1.657, 1.577, 1.321 s |
| PNG, 2250 x 3000 | 0.623, 0.875, 0.528 s | 0.646, 0.893, 0.539 s |

The rebuilt runner SHA-256 is
`AC87CAABAD2389FA2A649BF8DA7F5C1D02076666684DBAC3D6B5FDDEA1EBB70B`.
The engine trace reports Impeller decompression and texture upload. Source access
continues through the existing guarded provider; the test used two already-local
images and preserved the existing catalog. These six samples establish the scoped
Debug improvement, not a universal latency guarantee.

Exact-commit hosted checks are pending publication.
