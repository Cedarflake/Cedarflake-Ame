# ADR 0041: Resource-aware image and text scheduling

- Status: Accepted
- Date: 2026-11-07

## Context and decision

Interactive semantic search and background library analysis consume different encoders from the same
admitted SigLIP 2 package. Keeping image analysis running during a search avoids repeated image-worker
initialization and preserves index progress. The [paired-worker evidence](../acceptance/r7-semantic-discovery.md#parallel-worker-evaluation--2026-11-07)
establishes concurrent native execution with unchanged vectors. It also establishes additional memory
cost and CPU contention; concurrency is not a universal throughput improvement.

One application owner manages at most one image worker and one text worker across classification,
indexing and foreground search. Individual workers retain one encoder, one inference thread, bounded
messages, pinned artifacts and joined retirement. Image encoding follows the existing GPU preference;
text retains CPU execution. This replaces ADR 0034's application-wide single-worker admission while
preserving its per-worker protocol and source-publication contracts.

Domain owns the capacity decision and serial/parallel state. A narrow platform port reports usable
CPU parallelism, total and available physical memory, and remaining commit capacity. Windows adapters
read that state without changing system configuration. Application owns admission, priorities,
residency and retirement; model adapters neither choose resource policy nor access catalogs.

The Windows capacity adapter uses `GlobalMemoryStatusEx` with a correctly sized, initialized
`MEMORYSTATUSEX` output that lives for the synchronous call. This narrow unsafe boundary admits no
retained pointer, allocation ownership transfer or system mutation. A failed call returns unknown
capacity and serial admission; Rust's usable parallelism query supplies the processor count.

## Initial admission policy

These thresholds are conservative runtime policy for the admitted model package, not promises of
equal performance across devices. Revisit them using affected-device evidence when the package or
runtime memory contract changes.

| Condition | Policy |
| --- | --- |
| GPU image execution, at least 4 usable logical processors | Eligible for image/text overlap |
| CPU image execution, at least 8 usable logical processors | Eligible for image/text overlap |
| Less than 12 GiB usable physical RAM, or unavailable capacity evidence | Serial |
| Eligible device with at least 3 GiB available physical memory and commit headroom | Parallel admission |
| Available physical memory or commit headroom below 1.5 GiB, or explicit memory pressure | Return to serial admission |

Sample capacity at most once every two seconds during ordinary work. An explicit pressure signal
immediately closes parallel admission. After a downgrade, require the entry headroom continuously
for 30 seconds before restoring parallel mode. Hardware capability alone cannot override current
memory pressure. Do not add processor-frequency tiers, a user-maintained device profile or CPU-load
prediction in this stage.

The image job retains its 2-GiB committed-memory limit; the text job is limited to 1 GiB. At most
three GiB of worker commit is therefore admitted across the two encoders. The measured text cold
peak is approximately 702 MiB; acceptance must exercise the lower text limit with the native worker.
These job limits do not include Ame's catalog, preview cache or GPU allocations. The host headroom
test and existing bounded caches remain necessary.

## Ownership and interaction

Foreground requests take priority at bounded admission boundaries. Each encoder runs one request at
a time; parallel mode permits different encoders to overlap. Background source reads and decoding
retain their existing shared admission. Text inference consumes a bounded description and may overlap
image work only through the application scheduler. Classification taxonomy text and search text share
the same text owner; they cannot create a third model process.

In serial mode, finish or cooperatively yield the current bounded operation, retire the other encoder,
and then admit the foreground request. Preserve completed features and background checkpoints. Mode
changes cannot publish an incomplete source read or reset analysis progress. Keep a bounded text
session for continued input; resume pending background work when foreground demand subsides.

Keep only the latest pending foreground request. Supersession fences its publication and releases its
own work; it cannot cancel an unrelated image request. Query results retain their fixed manifest while
indexing continues. A cached query feature does not require loading a model again.

Admission state is never held across model initialization, inference, database access or process
retirement. Each admitted operation owns its cancellation identity and encoder lease. Serial fallback
waits for the other active lease to retire before replacing its process; failed retirement retains
ownership and blocks replacement. Closing the runtime or changing the model/GPU preference cancels
and joins both encoders before configuration changes commit. Idle text residency expires independently.

## Validation and replacement

Exercise capacity thresholds, downgrade/recovery hysteresis, foreground precedence, stale replies and
joined shutdown at the owning boundary. Use real concurrent workers to verify numerical agreement,
memory limits and failure isolation. Desktop acceptance compares cold/warm search latency, continuing
background progress, serial fallback and mode changes with source hashes unchanged. The existing
paired-worker experiment cannot establish those application outcomes.

Serial scheduling remains the fallback when capacity cannot be established. Removing parallel mode
must preserve queued work, semantic manifests, user decisions and the GPU preference; no database
migration or source-media operation is required. Replacing either encoder preserves model identity,
the admission interface and independent cancellation/retirement evidence.
