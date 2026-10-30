# R5 inference runtime evidence

Date: 2026-10-30. This record covers isolated runtime probes for the selected initial-screening
candidate. The [roadmap](../roadmap.md#r5-implementation-sequence) owns delivery status; the
[exploratory comparison](r5-model-evaluation.md) retains the separate model-quality observations.
No model or runtime dependency has entered the production application through these probes. The
Ame-owned category/matching contracts below are separate from the native inference adapter.

## Category and feature contracts

[ADR 0034](../architecture/0034-versioned-classification-contracts.md) separates display labels,
canonical category rules, the shared embedding space and independent encoder revisions. The
[native classification interface](../../rust/src/classification.rs) exposes validated Ame-owned
definitions/features and bounded matching without vendor types, file reads or database access.
It is not yet connected to desktop classification or durable R5 decisions.

The 18 new focused tests pass: ten domain cases and eight application matching cases. They cover
label/ordering reuse, semantic edits, added/retired active definitions, custom category counts,
malformed and duplicate definitions, extreme finite vectors, mixed/missing text evidence, stale
image pipelines, dimensional mismatch, max-description ranking, ties and single-category margins.
Two existing cases also match the local test filter and pass. These checks establish contract
behavior, not model accuracy or complete classification delivery.

Physical owners contain 179 taxonomy lines, 108 feature lines, 31 error/export lines plus three
test-module/spacing lines, 124 matching lines plus three test-module/spacing lines and a seven-line
public facade. Dedicated domain/application tests contain 181 and 202 lines; no inline test body,
schema, bridge generation, dependency or presentation change is included.

## Artifacts and execution boundary

The SigLIP 2 Base/16-224 ONNX snapshot remains
`ba1f3b0843f24bc5417d38e19c37b287d719b2f4`. The two encoders and tokenizer total 503,840,830 bytes.
The image graph has FP16 weights but accepts F32 NCHW pixels; its pooled output is F32 with 768
dimensions. The INT8 text graph accepts 64 I64 token IDs and returns the same pooled dimension.
The model artifacts, prepared pixels, raw features and diagnostic executables remain ignored.

| Artifact | Bytes | SHA-256 |
| --- | ---: | --- |
| Image encoder | 186,039,516 | `a1959f7bd3993a607e48839f6d01e25b876fe76afda301b028b78eef68aabd95` |
| Text encoder | 283,438,275 | `3a0603d3a00c05a80a6ded4743c16aaac7b1e62cdcc7e362e7ce418659b96400` |
| Tokenizer | 34,363,039 | `cb9140fae3ac5122c972d37adf83e1248471a38147ad76f8215c8872c6fd8322` |
| ONNX Runtime Windows x64 CPU archive | 82,929,944 | `5869e26bb25314375f2d5ef1877e3ea06b1543d6c710e6e4a1ec2f7177ab4b2c` |
| Native CPU runtime DLL | 17,253,728 | `cb2fa518b18356617f6f9f460c012501da2819ec7e5c1d80bd8f5de20492db0c` |

The native probe uses [ONNX Runtime 1.31.1](https://github.com/microsoft/onnxruntime/releases/tag/v1.31.1),
[`ort` 2.0.0-rc.15](https://crates.io/crates/ort/2.0.0-rc.15), and
[`tokenizers` 0.23.2](https://crates.io/crates/tokenizers/0.23.2). Their license declarations are
MIT, MIT/Apache-2.0, and Apache-2.0 respectively. Runtime notices and privacy documentation remain
with the extracted DLLs. The upstream Google model card identifies Apache-2.0; the ONNX conversion
card links that base model without its own license field. Distribution admission must retain that
provenance rather than infer a separate license declaration from the conversion card.

The Rust adapter disables default features and enables only `std`, `load-dynamic` and `api-31`.
Tokenizer HTTP support and parallelism are disabled. The parent sets `ORT_DISABLE_TELEMETRY=1`
before starting the worker; the worker also disables telemetry through the API. The official
[privacy document](https://github.com/microsoft/onnxruntime/blob/v1.31.1/docs/Privacy.md) distinguishes
this startup control from API-only suppression, which can follow an initialization event.

Each case runs in a separate process, uses sequential execution, one inter-op thread and disabled
thread spinning. Image graph optimization is basic; text optimization enables all graph passes.
Encoder output dimensions and finite values are checked. Source media is not opened: local image
probes reuse 32 previously prepared pixels after manifest validation. Timings exclude source
read/hash/decode and the application's foreground scheduling.

## Local native measurements

Two passes encode the same 32 images. RSS is sampled every 5 ms by the owning parent.
These are single local-primary observations, with possible workstation contention; they do not
establish cloud-primary or integrated desktop performance.

| Threads / image batch | Peak RSS MiB | Load ms | First batch ms | Warm median / P95 ms | Images/s |
| --- | ---: | ---: | ---: | ---: | ---: |
| 1 / 1 | 848.53 | 241.84 | 408.60 | 371.17 / 419.86 | 2.66 |
| 2 / 1 | 848.53 | 236.56 | 234.08 | 244.71 / 571.10 | 3.55 |
| 1 / 2 | 873.39 | 299.62 | 883.01 | 834.49 / 1,846.21 | 1.70 |
| 2 / 2 | 872.96 | 253.08 | 430.70 | 412.61 / 481.31 | 4.75 |

All 32 images retain their highest-scoring description against the retained image-feature baseline.
Minimum feature cosine is 0.9999976. This measures embedding compatibility, not category accuracy.

With single-description batches, the 24 text descriptions use 674.13 MiB peak / 567.11 ms load /
50.70 ms warm median on one thread, and 671.75 MiB / 477.04 ms / 29.22 ms on two threads.
All 24 native token sequences match the retained reference, including EOS, truncation and padding.
The text encoder is retired separately from the image encoder.

The parent terminated an image worker 20 ms after readiness. Process termination and wait took
24.44 ms; no result file was published. Product cancellation still needs the application generation
fence, retry ownership, process containment and integrated foreground checks in R5-04.

## Cloud runtime measurements

cloud-primary reports two logical processors and 8,536,289,280 usable physical bytes. Its isolated
probe reuses the installed Python 3.12.14 and NumPy with ONNX Runtime 1.31.1, psutil 7.2.2 and
FlatBuffers 25.9.23 in a separate dependency directory. The Python wheel's SHA-256 is
`ae8a9a351efda685cc59a66226cdefaa37a126c1d66872ca23a2de9425326130`; its runtime DLL hash is
`e45f61e50d8650885452be538a00bb14a715e46a1bc3691000ce947e89ed0306`.
This is a different binary from the standalone native DLL. It measures the selected model and
version's CPU behavior through Python, not the eventual Rust worker's integrated cloud workflow.

The image probe generates eight bounded gradient tensors and performs two passes; the text probe
reuses the 24 verified token sequences. No original images are transferred or read. Each child has
a 180-second / 2-GiB diagnostic ceiling and sampled RSS every 10 ms; all cases finish successfully.

| Threads / image batch | Peak RSS MiB | Load ms | First batch ms | Warm median / P95 ms | Images/s |
| --- | ---: | ---: | ---: | ---: | ---: |
| 1 / 1 | 858.26 | 559.49 | 686.83 | 566.77 / 581.62 | 1.72 |
| 2 / 1 | 858.65 | 1,081.08 | 544.97 | 562.88 / 622.06 | 1.78 |
| 1 / 2 | 882.93 | 753.53 | 1,161.98 | 1,051.00 / 1,061.07 | 1.85 |
| 2 / 2 | 883.71 | 594.74 | 1,083.86 | 1,032.62 / 1,056.95 | 1.88 |

Single-description text encoding takes 81.37 ms warm median / 496.14 MiB peak on one thread and
82.88 ms / 511.73 MiB on two. These cases consume prepared tokens without loading Rust's tokenizer;
their RSS is not a replacement for the higher native text-process observation. Forced image-worker
termination after readiness takes 26.73 ms. All child and supervisor processes exit.

The measurements favor one image per batch and one inference thread for the initial constrained
device configuration: a second thread changes throughput little, while a two-image batch roughly
doubles the uninterrupted inference interval. Long-run contention, actual source decode, the
supervised native worker and foreground gallery/viewer interaction remain integrated checks.
A follow-up of the one-thread / one-image case at BelowNormal process priority completes in
10.64 seconds, using 8.84 process-CPU seconds and 858.13 MiB peak RSS. Its mean CPU occupancy is
0.832 logical cores, about 41.6 percent of this two-logical-processor device. This short probe includes
startup and preparation; it does not measure foreground frame latency or sustained contention.
The CPU/model archives are verified before extraction, and the cloud archive is removed immediately;
the unused local archive, debug symbols and half-precision input fixture are also removed.
Readback after the cloud probe confirms no owned process or temporary Python dependency/transfer
part remains. The verified model files are retained for integration, and free storage is
11,461,218,304 bytes at that observation.

## Text batch compatibility

Changing only INT8 text batch size from eight to one under the retained ONNX Runtime 1.24.4 changes
the leading category of six of the 240 comparison images. Minimum text-feature cosine is 0.9982548;
recomputing batch eight matches the old features at 0.9999999. The new runtime with batch one versus
the old runtime with batch one gives minimum cosine 0.9992977. Token sequences remain identical.

Text batching and runtime configuration therefore belong in feature compatibility. Encode each
description separately so adding unrelated descriptions cannot change its quantization context.
Runtime, tokenizer, graph, preprocessing and batch-policy changes invalidate the affected feature
version. The retained 87.43-percent provisional agreement is not a quality result for this new
runtime/policy combination. Purpose-aware calibration and independent held-out evaluation retain
their requirements; no new high-confidence threshold is established by these process probes.

A separate cached-feature diagnostic compares description aggregation under the new single-text
encoding policy. Maximum-description similarity agrees with 163 of the 191 definite provisional
labels, mean similarity with 159, and a normalized mean prototype with 162. These reuse the old
image features and purpose rubric and are not independent validation. They provide no reason to
replace the existing maximum-description policy with averaging during runtime integration.

The complete 240-image retained cohort was then checked with native single-image/single-description
features. The first 32 compatible native image features were reused; the remaining 208 were encoded
from the verified cached pixels in seven bounded 32-image probe windows, padding the last window
with repeated inputs and discarding those repeated outputs. No source file was opened. Native image
features have minimum cosine 0.9999924 against the old image features, and replacing those old image
features changes none of the 240 category predictions when the current text features are held fixed.
Agreement remains 163/191 definite provisional labels, with 49 uncertain labels outside that ratio.
This confirms the runtime/policy comparison; it does not validate the later purpose-aware rubric or
establish independent human accuracy. The temporary tensor is removed after process retirement.
