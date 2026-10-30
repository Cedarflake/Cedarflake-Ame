# ADR 0034: Separate category rules from encoder compatibility

- Status: Accepted for implementation
- Date: 2026-10-30

## Context

Classification compares images with configurable descriptions. Display names, comparison rules,
image features, text features and human choices have different lifetimes. Invalidating all of them
when a label changes wastes image reads and can erase intent. Reusing them after an encoder change
can silently mix incompatible results. The retained runtime probe also found that INT8 text batching
changes some predictions even with identical tokens.

The [product contract](../product/r5-classification.md) owns whole-group authority, optional review,
manual-album separation and source safety. This decision owns category and feature compatibility;
it does not grant model downloads, source reads, publication or human-decision authority.

## Category contract

An active taxonomy is a bounded snapshot of flat definitions. Each definition has an opaque stable
ID, a display label and one to eight comparison descriptions. Names and ordering are presentation
data. The canonical rule revision includes active IDs and their descriptions, sorted independently
of presentation order. UTF-8 descriptions are trimmed at admission; internal spelling and whitespace
remain significant. No folder name or model-generated label becomes a category identity.

One snapshot admits up to 256 active definitions, with labels of at most 80 characters and individual
descriptions of at most 1,024 characters. These are resource/admission limits, not a target category
count. Exact duplicate descriptions within a category and duplicate category IDs are rejected.
Shared descriptions across different categories are allowed and can produce an ambiguous tie.

Definitions can be added or retired explicitly. Retirement removes a definition from future
matching; persistence retains its identity and description history for saved human choices. Remapping
a retired human choice is a separate explicit command, never a side effect of refreshing the active
taxonomy. An empty active scheme has no classifier and produces no invented `其他` result.

R5 does not generate categories from isolated low scores. Start with an editable broad scheme and
retain its versioned inclusion/exclusion rubric. A useful new category needs a repeatable distinction
in the user's collection; small occupancy alone neither deletes a category nor establishes an error.
Repeated corrections can supply local examples for evaluation. They do not silently train a model,
change definitions, research new cultural context or create categories. Category-growth automation
requires a separate measured policy before admission.

## Features and matching

Adapters supply Ame-owned, opaque revisions for the shared embedding space and the independent image
and text pipelines. A pipeline revision binds artifact digests, runtime/build identity, tensor and
tokenizer behavior, preprocessing, normalization and relevant execution parameters. Vendor object
types and model-repository names stay inside adapters. Dimensions must agree with the admitted
contract; non-finite and zero-length vectors are rejected before ranking.

Text features also bind the exact description revision. Missing, duplicate, unexpected or mismatched
text features reject construction of a matcher. Image inference and category matching have separate
boundaries: a new taxonomy can match retained compatible image features and reuse unchanged text
features. A display rename or reorder preserves the rules revision. Semantic edits or a changed
active set change the matching revision while preserving the independent image-pipeline revision.

### Rebuildable feature storage

A separate versioned SQLite feature cache stores normalized F32 vectors outside the catalog and
source trees. Image keys bind exact content scheme/digest/length, embedding space, image pipeline
and dimensions. Text keys bind description revision, embedding space, text pipeline and dimensions.
Neither key includes a run ID, display label, whole taxonomy revision or the other encoder's pipeline.
Each payload has an integrity digest over its key and exact bytes; loading preserves those bytes
instead of normalizing the vector again. Invalid entries produce a structured cache error.

The initial cache budget is 256 MiB for vectors and a fixed per-entry allowance; the database has
an independent 544 MiB page limit and a 2 MiB SQLite page cache. Indexed recency eviction removes at
most 64 entries per insertion. Explicit cleanup removes at most 256 entries and reclaims at most
256 free pages per call. SQLite transactions cover only cache reads/writes, never encoding.
The cache namespace is pinned through the existing directory capability, and its fixed-name file
has a separate application identity and schema version. An existing unrelated or newer database is
rejected before SQLite opens it for writes. New caches are initialized in owned temporary storage
and published without replacing an existing file, so interrupted setup does not expose a partial schema.

Description preparation reuses compatible entries, encodes only missing descriptions and releases
the text worker after the batch, including failure/cancellation. Completed description entries survive
an interrupted batch. Cache loss or eviction does not change queue checkpoints, immutable predictions,
human choices or stopped-run intent; missing vectors are recomputed only for admitted analysis work.
Source identity and claim checks still authorize image-feature publication and prediction publication.

The initial ranking uses L2-normalized vectors and maximum description cosine per category, retaining
the comparison's measured aggregation rather than introducing an unvalidated averaging rule.
Ties have stable category-ID ordering and zero separation. A single-category scheme has no runner-up
margin. Ranking produces similarities and provenance; it neither manufactures probabilities nor
marks a result human-confirmed. Optional review, bounded re-screening and calibrated confidence are
application policies that consume this evidence, separately versioned with their calibration data.

### Bounded screening policy

`ScreeningClassifier` owns first-pass matching and at most one refinement of the same cached image
feature. A refinement uses the same model and active category IDs, with a different description-rule
revision. It cannot add a category or request another image read. The result retains both rankings,
the selected suggestion and its screening-profile revision; it carries no human confirmation.

`ScreeningProfile` binds the model, primary/refinement rule revisions, calibration-evidence revision
and finite top-two separation thresholds. Thresholds can be uniform or explicitly cover every active
category ID. A category without sufficient review evidence has an uncalibrated review band; it keeps
its suggestion available. Missing, duplicate or extra category entries reject the profile. Its
canonical revision includes the rule form, sorted category entries and absent review bands. Names can
change without invalidation; semantic edits, category changes or encoder replacement reject an old
profile before matching. A missing profile still produces a usable uncalibrated suggestion. One
category has no runner-up separation and cannot establish a calibrated band from its score alone.

The initial leading category selects the refinement cutoff; the final leading category selects the
review cutoff. Refinement therefore cannot accidentally reuse the previous category's review band.
The profile selects refinement below its first-pass cutoff, then optional review below its final
cutoff. Exact ties always remain boundary cases; threshold equality otherwise uses the separated side.
The remaining band is eligible for routine sampling, not automatically confirmed or guaranteed
correct. Application admission of a measured profile remains separate from structural validation.
No numerical cutoff is a built-in default. The [calibration evidence](../acceptance/r5-calibration.md)
records candidate profiles and their limitations before any runtime setup enables them.

## Selected runtime boundary

The candidate is the pinned SigLIP 2 Base/16-224 FP16-image/INT8-text export documented in the
[runtime evidence](../acceptance/r5-runtime-admission.md). Preserve both encoders. The initial CPU
configuration is one supervised process, one inference thread, sequential execution, disabled
spinning and one image or description per batch. Encode descriptions individually; appending an
unrelated description must not change another description's quantization context.

The image contract is oriented RGB, first frame, whole-image bilinear resize to 224 × 224, F32 NCHW
input and normalization by mean/standard deviation 0.5. Text uses the pinned tokenizer, EOS and
64-token padding/truncation policy. Both return 768 values. These are adapter details, included in
pipeline revisions rather than hard-coded in the general taxonomy/matching domain.

The admitted CPU adapter uses ONNX Runtime 1.31.1, `ort` 2.0.0-rc.15 and `tokenizers` 0.23.2. Runtime code is
MIT; bindings are MIT/Apache-2.0; tokenizer code is Apache-2.0. Retain runtime license, third-party
notices and privacy documentation. Disable default binding features and runtime download, tokenizer
network support and tokenizer parallelism. Set `ORT_DISABLE_TELEMETRY=1` in the child environment
before loading the runtime, then disable it through the API as well.

The [upstream Google model](https://huggingface.co/google/siglip2-base-patch16-224/tree/75de2d55ec2d0b4efc50b3e9ad70dba96a7b2fa2)
declares Apache-2.0. The [pinned conversion](https://huggingface.co/onnx-community/siglip2-base-patch16-224-ONNX/tree/ba1f3b0843f24bc5417d38e19c37b287d719b2f4)
identifies its weights as an ONNX conversion of that base model, with no separate license declaration.
The selected graphs retain that provenance and have no external tensor files. Admit explicit local
installation of these hash-pinned conversions under the upstream
[Apache-2.0 grant](https://www.apache.org/licenses/LICENSE-2.0), which includes mechanically converted
object forms. Preserve Google's attribution, the license, conversion source
and quantization notice. This admission applies to these artifacts, not every file in the conversion
repository. Model weights are separate, explicitly installed data; they are not bundled or mirrored
by the application. A later redistribution must retain the applicable license and notices.

### Worker ownership and protocol

The `classification-worker` Cargo feature builds a separate native executable and its adapter.
Default desktop builds do not load an inference runtime. The worker accepts one encoder for its
entire lifetime; replacing image with text inference requires retiring that process first. Ame's
supervisor starts it below normal priority, attaches a Windows Job before sending initialization,
and bounds the job to one process and 2 GiB of committed process memory. Closing the job kills its
worker. Source admission, publication and the application's single-worker scheduling remain outside
this adapter.

Anonymous standard-input/output pipes carry versioned, length-prefixed JSON control records and
fixed-size little-endian F32 tensors. Control records are bounded to 32 KiB, image input to one
3 × 224 × 224 tensor and output to one 768-value embedding. The worker receives prepared pixels or
one bounded description, never an original-media path or catalog access. Session and request IDs
must match before a response is admitted. Readiness also binds the complete encoder/pipeline
revision, so an older worker executable cannot label features with its caller's newer identity.
Protocol errors retire the connection; a native inference
failure is a structured terminal outcome. A new process can retry without replaying a published
decision. Reader/writer work stays off the caller's wait loop so cancellation can terminate a worker
even while its pipe is blocked.

Before loading native code, the worker verifies the pinned runtime DLL, provider helper, selected
graph and text tokenizer with incremental SHA-256 (`sha2` 0.10.9, MIT/Apache-2.0). Windows namespace
guards hold each directory against rename and reject reparse/cloud-placeholder entries; file guards
deny writes/deletion from verification through use. No artifact lookup falls back to another version,
network download or source directory. Hashing uses a bounded buffer. Runtime notices remain alongside
the separately installed runtime.

DLL dependency lookup is restricted to Windows system libraries and the pinned runtime directory;
the current directory and arbitrary search-path entries are excluded inside the worker.

The narrow Windows Job, DLL search and artifact guards require `unsafe` calls to Win32. Owned handles close
exactly once; borrowed process handles remain valid during job assignment; fixed initialized structs
and their actual byte lengths are passed to the kernel. UTF-16 path buffers live through each call,
and each DLL-directory cookie is removed once. No borrowed pointer escapes a call. Boundary tests
exercise invalid artifacts, held namespaces, malformed frames, process exit and cancellation; a real
model process check verifies the same pinned tensors and tokenizer used for admission. These native
operations do not add privilege or execute in the catalog/domain layers.

The application owns single-worker admission, source admission, immutable runs, cancellation fences
and publication. The worker never opens the catalog or changes a human decision. Native runtime
errors become structured per-work outcomes. The supervisor joins process and pipe-thread retirement
before reporting cancellation. Complete retirement precedes replacement; a timeout cannot authorize
another worker while the first still owns memory or input. Failed retirement retains its owner for
recovery. Foreground scheduling and result publication remain application responsibilities.

`ClassificationEncoderResidency` owns demand loading and reuse through an Ame encoder factory port.
The native factory maps that port to the verified worker without exposing protocol/runtime objects
to application policy. Creation acquires ownership before initialization can start a process or
load a model, so failed initialization cannot abandon a worker whose retirement is incomplete.
An image worker retains its last completed-use time under a monotonic clock;
the configured idle interval is supplied by scheduling policy. Description batches explicitly retire
their text worker when their required features are ready. Switching encoders first completes process
and pipe retirement. Failed or incomplete retirement preserves ownership and blocks replacement,
including reuse of the same encoder. Cancellation rejects a late feature before returning it and
retires its worker. Pressure, disable and suspend use the same explicit retirement operation;
durable dispatch intent and publication fences belong to the analysis scheduler.

## Alternatives and consequences

Fixed output classes would prevent user descriptions and are rejected. Re-encoding images for
description edits wastes source reads; canonical rule and pipeline revisions make that unnecessary.
Comparing raw unversioned arrays is rejected because dimensions alone do not establish compatibility.
Mean/prototype aggregation showed no consistent advantage in the retained diagnostic and would
require a new scoring revision and calibration before adoption. GPU acceleration remains optional.

The feature cache has its own schema; it adds no catalog or desktop bridge schema. Persistence and
worker adapters use the same validated inputs without depending on each other's engine types. The 256 × 8 description limit
bounds matching storage; future increases need resource evidence, not new category semantics.

## Validation and replacement

Boundary tests cover renaming, reordering, semantic edits, active-set changes, malformed definitions,
missing/mixed features, invalid vectors, ties and non-fixed category counts. Runtime measurements and
their limits remain in the linked evidence. Purpose-aware calibration and independent held-out
evaluation retain the requirements in the [execution plan](../plans/r5-classification.md); this
decision establishes no high-confidence threshold or product quality pass.

Replace the encoder behind its feature contract, create new pipeline revisions and rematch only
compatible features. Preserve older immutable evidence and human decisions. Replacing the scoring
policy creates a new matching/calibration revision; it does not require recataloging source media.
