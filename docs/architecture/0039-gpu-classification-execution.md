# ADR 0039: Default-enabled GPU image classification

Status: Accepted

Date: 2026-11-07

## Context and decision

The existing SigLIP 2 image encoder benefits from local GPU inference. Settings exposes one
persisted `GPU 加速` switch, initially enabled. This affects smart-album image analysis; text
encoding retains the admitted CPU path. Manual classification controls remain withdrawn.

Use ONNX Runtime DirectML 1.24.4 with DirectML 1.15.4 in the existing isolated worker. The CPU
runtime remains ONNX Runtime 1.31.1. The pinned `ort` 2.0.0-rc.15 binding uses API 24, supported by
both runtime versions. Runtime selection is immutable within one worker process.

DXGI enumeration selects a hardware D3D12 adapter with the largest dedicated memory, retaining
the first enumerated adapter on a tie. Disabled acceleration, absent GPU payload or no supported
adapter selects CPU. Initialization/inference failures remain structured worker failures; they do
not silently emit CPU output under a GPU feature identity. The setting remains available to select
CPU explicitly. A selected device is distinct from an initialized worker.

The adapter's `device.rs` owns the narrow unsafe Windows API calls. `CreateDXGIFactory1` returns an
owned COM interface; enumeration returns owned adapter references; `GetDesc1` supplies a copied
description; `D3D12CreateDevice` writes into a typed `Option<ID3D12Device>`, which is dropped before
selection returns. No raw pointer or COM interface crosses the adapter. Failed HRESULTs omit that
adapter; enumeration is bounded to 32. No GPU model or inference is created in this probe.

## Precision and compatibility

DirectML FP16 inference diverged on the retained corpus. Use FP32 computation from the exact
values of the existing pinned FP16 image weights. `precision.rs` decodes the official ONNX schema,
promotes the 218 FP16 tensors and associated tensor/Cast types, and serializes the graph in worker
memory. No replacement model is downloaded or persisted. Unsupported external data, sparse data,
subgraphs, training graphs, inconsistent tensor sizes and nonfinite weights fail admission.

The DirectML session uses one fixed `1×3×224×224` image, sequential execution, one CPU thread,
disabled memory patterns and disabled CPU execution fallback. These follow the
[DirectML execution-provider requirements](https://onnxruntime.ai/docs/execution-providers/DirectML-ExecutionProvider.html).
The existing 2 GiB process job, tensor bounds, source leases and joined cancellation remain.

GPU features have a separate image-pipeline identity binding both DLL digests and the precision
conversion. The embedding space and CPU text-pipeline identity remain unchanged. Readiness binds
the requested backend's full contract; a CPU worker cannot satisfy GPU readiness.

Before opening a dispatcher, `model_transition` replaces a recognized CPU/GPU setup through the
existing configuration transaction. It preserves category definitions, pause state, historical
runs, predictions and legacy decisions. GPU image features are rebuilt on demand; CPU features
remain reusable when switching back. This can require another analysis pass after first enabling
GPU. A pipeline change follows ADR 0034's existing uncalibrated-setup rule: the CPU-only screening
profile is not copied or relabelled as GPU calibration. No new confidence claim is introduced.

## Preferences and distribution

Installation storage schema 3 adds one default-enabled preference while preserving model selection
and pending download intent. Saves require proven native retirement. The existing desktop
configuration lifecycle joins the previous worker, commits the preference and opens the replacement.
UI failure recovery rereads the saved preference, including a committed save followed by failed
reopening. Catalog schema and source media are unchanged.

`accelerator_distribution.json` pins the two official NuGet archives, three x64 DLLs and six
license/privacy/third-party notices. Development, ZIP and MSI bundles require
`data/classification-accelerator` alongside the separate CPU runtime. Preparation verifies exact
lengths/digests, extracts only selected entries into temporary directories and publishes atomically.
It does not install a system runtime. Microsoft.AI.DirectML's binary uses Microsoft's redistributable
license, not MIT; retain its supplied Windows ML distribution terms and notices. ONNX Runtime is
MIT, ONNX schema and prost are Apache-2.0, and half is MIT/Apache-2.0 (used under Apache-2.0).

The generated ONNX types come from ONNX 1.20.1's `onnx/onnx-ml.proto` descriptor. Regeneration is
owned by `tool/bridge_generate_onnx_schema.ps1`; the generator pins the descriptor digest and prost
version. Schema attribution and the Apache-2.0 license ship through the native-license registry.

## Alternatives, validation and rollback

CPU-only execution remains the user-selectable fallback but leaves the measured acceleration unused.
DirectML FP16 is rejected by numerical comparison. CUDA would restrict the supported hardware;
Windows ML would add deployment and runtime-selection changes beyond this adapter. DirectML is in
sustained engineering; replacing it later must preserve Ame-owned contracts and admission evidence.

Validate numerical agreement and fixed retrieval results, worker retirement, default/migrated
preference, device switching, pipeline separation, unavailable hardware, and complete package
payloads. The [GPU evidence](../acceptance/gpu-analysis-acceleration.md) owns results and limits.

Turning the setting off restores CPU analysis without deleting history. Removing this implementation
requires retaining schema-3 read compatibility or restoring a pre-upgrade installation-settings
backup; an older binary rejects the newer settings version. Do not delete the catalog, cache or user
decisions to roll back. Backend replacement requires a new image-pipeline identity and equivalent
source, isolation, cancellation and distribution guarantees.
