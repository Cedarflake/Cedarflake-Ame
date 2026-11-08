# GPU analysis acceleration evidence

## Native evaluation — 2026-11-07

The retained local-primary evaluation uses 368 existing JPEG previews, identical between encoders;
it does not read or modify originals. SigLIP 2 GPU image encoding took 5.50 seconds, compared with
27.92 seconds for Gemma GPU and 128.12 seconds for the existing SigLIP 2 CPU path. These are corpus
encoding measurements, not cold application startup or full-library indexing times.

FP16 DirectML output failed numerical comparison (minimum cosine about 0.52, median about 0.73).
Promoting the same FP16 weight values to FP32 computation produced minimum CPU/GPU cosine
0.9999313 and median 0.9999995. Fixed retrieval results were unchanged: Chinese 15/24 and 20/24,
English 19/24 and 23/24 for top-one and top-five retrieval respectively. These results do not establish
purpose-aware classification accuracy or GPU screening calibration.

The production Rust worker then encoded the existing 32-image admission tensors and 24 individual
descriptions under its 2 GiB Windows job. GPU minimum image cosine was 0.999998329; CPU text output
retained cosine 0.999999999999998. GPU initialization, including artifact verification and graph
promotion, took 4746 ms; explicit retirement took 144 ms; cancellation plus retirement took 116 ms.
The unchanged CPU path retained image cosine 0.999999999999997, with 549 ms initialization,
93 ms retirement and 42 ms cancellation/retirement. Both probes reported zero source files opened.

The runtime uses ORT API 24 for both CPU 1.31.1 and DirectML 1.24.4. The successful CPU probe verifies
that lowering the binding API did not change the retained CPU vectors. GPU-session CPU fallback
was disabled. The comparison used the available NVIDIA adapter; other vendors remain unmeasured.

## Integration checks

The regenerated typed bridge, native Release build, all-target/all-feature Clippy and Dart analyzer
passed. Focused regressions passed for the default-enabled schema migration, preservation of
installation state, CPU/GPU setup replacement, renamed categories, paused dispatch, retained runs,
stale setup rejection, tensor conversion and backend-bound worker readiness. The existing Rust and
Dart dependency-boundary checks passed, including the exact production-composition registrations.

Settings checks passed for the default-enabled switch, disabled input while replacement is pending,
saved selection after page reentry, and a committed preference followed by replacement failure.
The existing configuration-lifecycle suite and Settings-page suite passed. The ONNX schema generator
reproduced the checked-in types from its pinned descriptor without model artifacts.

The actual accelerator archives passed selected-file extraction and digest checks. CPU package
guardrails, portable archive checks, MSI payload/component checks, and unsigned-payload validation
passed, including rejection cases for absent runtime payloads. Both packages require all three GPU
DLLs and six supplied notices beside the separate CPU runtime.

The Profile desktop was exercised with an isolated native profile, the existing three-image
metadata fixture and an optimized DLL built with test-storage support. The loaded DLL matched its
build receipt. The settings switch initially selected the NVIDIA adapter, showed replacement in
progress while disabled, switched to CPU, retained that choice after normal close/restart, and
returned to GPU when enabled again. Both clients exited with code zero. Source-file hashes were
unchanged and the scratch profile was removed after all owned processes retired.

Read-only catalog inspection found separate CPU/GPU pipeline identities, two retained predictions
for each pipeline, and ten completed work items with no failed/waiting items. The two predictions
per pipeline correspond to two distinct contents in the three-file fixture. The final persisted
preference was enabled. This verifies the actual setting, runtime and persistence path; it does not
extend the retained retrieval experiment into a general classification-accuracy claim. The subsequent
copy adjustment adds `当前` to the selected-device and CPU descriptions without changing behavior.

Physical adapters from other vendors and a device-removal event have not been exercised on this
workstation; runtime errors retain the existing explicit failure path.

## Integration disposition — 2026-11-07

[PR 52](https://github.com/Cedarflake/Cedarflake-Ame/pull/52) integrated implementation `e93e9fdd`
into `main` at `8125b804`. Source review covered device/configuration lifetimes, preference migration,
pipeline/cache separation, precision promotion, artifact admission and package composition; no
blocking defect was found in that scope. The local checks above were reused without another broad run.

The user authorized merging after review without waiting for hosted CI. The
[exact-head run](https://github.com/Cedarflake/Cedarflake-Ame/actions/runs/42341238484) was still
running at merge; this disposition does not record a hosted pass. The local and remote feature
branches were deleted after confirming that `main` contained the reviewed implementation.

## Physical ownership

### Concurrent startup migration — 2026-11-08

The running Debug client's installation recovery reported `classification_installation_unavailable`
with `database is locked`. Its selected model directory already contained all three manifest-verified
artifacts. The error came from competing deferred schema-upgrade transactions, not missing models.
An eight-caller legacy-schema fixture reproduces the same error against `97279e23`. Acquiring an
immediate migration transaction and rereading its version preserves the saved installation, pending
download and default GPU preference; current-schema reads also succeed beside an uncommitted writer.

The nine installation-store tests pass in a small harness compiling copies of the production adapter,
ports and their tests against real SQLite. Only the unrelated error transport type is a local facade.
The full-crate local test compilation was stopped at low memory headroom; hosted checks own that run.
Eight Flutter installation/notification tests pass, including the configuration-specific recovery
message. Existing model files were only read and hashed; the running native DLL was not replaced.

The affected handwritten owners retain separate responsibilities: encoder factory 193 lines,
session encoder 177, graph promotion 121, application runtime facade 120, model transition 46,
Settings section 119, package preparation 82 and bounded download 40. These owners contain no
inline tests. Dedicated promotion, transition and Settings regressions contain 33, 127 and 77 lines
respectively. The 666-line ONNX schema module is reproducible generated output, not handwritten
runtime policy. The preference-storage owner has 32 production lines and 53 inline migration-test
lines, with no dedicated test file.
