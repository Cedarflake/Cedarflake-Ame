# R5 exploratory model comparison

Date: 2026-10-30. This record preserves the local comparison used to choose an initial-screening
candidate. Stage status belongs to the [roadmap](../roadmap.md); runtime admission, calibration,
product integration, and final quality acceptance remain in the [R5 plan](../plans/r5-classification.md).

## Scope and provenance

The local-primary experiment sampled 240 locally readable image groups with seed 20261030. It used
24 fixed English descriptions across six comparison categories, with four descriptions per category.
These comparison classes do not constrain configurable product categories. Original media remained
read-only; placeholders were not hydrated, and no source images were uploaded or copied into Git.
Model files, cached previews/features, manifests, and raw results remain in ignored local storage.

Reference labels were provisionally assigned through visual review, then reviewed a second time by
the same evaluator. They have not received independent human validation. The second pass knew prior
aggregate results; its labels were sealed before rescoring unchanged predictions. The unfinished
historical folder classifications were not used as ground truth. The second pass has 191 definite
labels and 49 uncertain cases; 17 certainty assignments changed from the original pass.

The subsequent intended-use clarification changes how cases such as photographed reaction toys
should be judged. The figures below retain their original provisional rubric; no new percentage is
claimed under that revised purpose-aware rubric. Known near-duplicate/crop relationships remain a
sampling limitation, and this small exploratory cohort is not the held-out acceptance set.

| Retained artifact | SHA-256 |
| --- | --- |
| Sample manifest | `dda0ad72af426474ee589c91e35ecba7008517ff6ae547f68a8ea7e0da28b2f7` |
| Fixed descriptions | `dbdcfd6c19d06174f7e9a0526e68148d772c5e47c20106d4e70d5bb544b58ab4` |
| Original labels | `99439c5a158efe09846724d5ed181bbf4a3ab1d7058f4a2d1467bab5d7edf705` |
| Rechecked labels | `54b227943cd0fc11dc1a042d30396f5d05ee40e981046ac9ab5547f7d562abc1` |

## Retained comparison

Scores use normalized image/text cosine similarity and the highest of four descriptions per class.
They are label agreement, not calibrated confidence or final-library accuracy.

| Configuration | Agreement on 191 definite labels | Encoder weights |
| --- | ---: | ---: |
| SigLIP 2 Base/16-224, FP16 image + INT8 text | 167/191 (87.43%) | 469.48 MB |
| SigLIP 2 Base/16-224, INT8 | 142/191 (74.35%) | 377.99 MB |
| SigLIP 2 Base/32-256, FP16 image + INT8 text | 158/191 (82.72%) | 472.81 MB |
| SigLIP 2 Base/32-256, INT8 | 134/191 (70.16%) | 379.35 MB |
| CLIP B/32, FP16 image + INT8 text | 152/191 (79.58%) | 240.59 MB |
| CLIP B/32, INT8 | 152/191 (79.58%) | 153.62 MB |
| DataComp B/32-256, FP32 | 132/191 (69.11%) | 606.17 MB |
| DataComp B/32-256, INT8 | 132/191 (69.11%) | 161.37 MB |
| TinyCLIP 45M/32, LAION + YFCC | 133/191 (69.63%) | 177.29 MB |

The selected candidate's tokenizer adds approximately 34.36 MB, bringing its local model artifacts
to approximately 503.84 MB before the inference runtime. Weight sizes are decimal MB and include
both encoders, retaining open-vocabulary text matching.

The selected [ONNX export](https://huggingface.co/onnx-community/siglip2-base-patch16-224-ONNX/tree/ba1f3b0843f24bc5417d38e19c37b287d719b2f4)
is pinned to `ba1f3b0843f24bc5417d38e19c37b287d719b2f4`, with `vision_model_fp16.onnx` and
`text_model_quantized.onnx`. The [Google model card](https://huggingface.co/google/siglip2-base-patch16-224)
describes zero-shot classification and identifies Apache-2.0 licensing; R5-02 still owns full
code/weight/runtime redistribution admission and packaged-artifact verification.

## Local CPU observation

ONNX Runtime 1.24.4, CPU execution provider, four threads, batch size 16:

| Selected candidate observation | Recorded value |
| --- | ---: |
| Encode 240 images | 29.14484 seconds |
| Image session load | 0.26678 seconds |
| Text session load | 0.34915 seconds |
| Encode the fixed descriptions | 0.34955 seconds |
| Separate source read/hash/decode preparation | 21.90495 seconds |
| Sampled test-process peak RSS | 1472.21 MiB |

Image preprocessing used EXIF orientation, first frame, whole-image bilinear resize to 224, and
0.5 mean/standard deviation. All 24 text token sequences matched the pinned upstream tokenizer.
The preparation record is separate and reusable; adding its time to inference is a phase estimate,
not a measured integrated Ame latency. Background transfers were present, and this was not a repeated
controlled speed benchmark. RSS includes the evaluation process, runtime, model, and input batches.
No selected-candidate GPU or cloud-primary timing is established here.
The later [runtime probes](r5-runtime-admission.md) retain separate local native and cloud CPU
measurements, including the text-batch compatibility difference; they do not revise this cohort's
original quality figures.

## Evidence needed during implementation

Retain this comparison rather than rerunning all models. Establish the purpose-aware configurable
rubric, independent content-group holdout/calibration, useful first-pass error rates, optional checks
and corrections per thousand groups, and actual human time saved. Validate one bounded feature-reusing
re-screen against the same rubric. Product admission also needs worker isolation/recovery, exact
artifact verification, two-core/8-GiB resource bounds, and foreground interference measurements.
