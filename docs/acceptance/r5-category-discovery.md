# R5 category discovery evidence

Date: 2026-11-01. This record covers the missing category discovery and naming policy in
[R5-02](../plans/r5-classification.md#r5-02-taxonomy-model-and-runtime-admission). It does not
establish product acceptance or replace the admitted SigLIP screening runtime. The
[roadmap](../roadmap.md#current-execution-queue) owns delivery order and remaining work.

## Retained-feature grouping

The exploratory probe reuses the 240 retained calibration feature vectors from the native SigLIP
adapter. Known visual duplicates reduce the diagnostic sample to 236 groups. It reads no source
media, performs no new inference and does not inspect either sealed holdout. Earlier provisional
labels describe the sample only; they are not user-confirmed category truth.

Complete-link cosine grouping, with at least three members required for a supported candidate,
produces the following results. The support count and similarity levels are exploratory inputs,
not admitted production thresholds.

| Minimum pair cosine | All clusters | Supported clusters | Groups in supported clusters |
| --- | --- | --- | --- |
| 0.88 | 205 | 6 | 23 |
| 0.82 | 160 | 16 | 65 |
| 0.75 | 112 | 28 | 139 |
| 0.68 | 70 | 22 | 175 |
| 0.60 | 38 | 21 | 209 |

The grouping takes 0.014 seconds. Several supported clusters still share the same broad use,
including screenshots, illustrations and reaction images. Visual grouping alone does not supply
useful names or settle category granularity. This diagnostic justifies checking semantic naming
and overlap before publishing new categories; it does not justify selecting any of these cutoffs.

The retained feature SHA-256 is
`8253957608da1157aee3224dcb9bb6bbbbc07d6af67c7f293f2c685050699826`.

## Small naming candidate

The first isolated candidate is
[HuggingFaceTB/SmolVLM-256M-Instruct](https://huggingface.co/HuggingFaceTB/SmolVLM-256M-Instruct),
revision `7e3e67edbbed1bf9888184d9df282b700a323964`, under Apache-2.0. Its INT8 vision, embedding and
decoder graphs occupy 259,855,281 bytes. The diagnostic uses Transformers.js 3.8.1 with ONNX Runtime
1.31.1 CPU, one inference thread, a single 512-pixel image, greedy generation and at most 24 new
tokens. Remote model access is disabled during inference. An owned subprocess watchdog limits the
run to 180 seconds and 2 GiB resident memory.

Eight cluster representatives use already-retained full-frame calibration thumbnails: B084, B039,
B132, B037, B186, B161, B054 and B023. English naming runs on all eight; Chinese naming additionally
runs on the first two. This is a diagnostic sample, not an accuracy estimate. No original or cloud
placeholder is opened, and no image is sent to an external service.

The ten requests finish in 42.22 seconds including startup. Model readiness takes 0.82 seconds;
individual requests take 1.66–4.85 seconds. Peak resident memory is 837,533,696 bytes (799 MiB).
The earlier ONNX Runtime 1.21.0 attempt rejects the graph's signed integer convolution before
inference; the 1.31.1 CPU runtime supports that operator. Its restricted-environment telemetry-ID
warning remains in the diagnostic log, so this diagnostic configuration is not a production gate.

Observed naming failures include:

- B132, a code screenshot, receives an incorrect book description rather than a useful category.
- B054, a reaction icon, receives an echo of the task instruction.
- B039 receives several overlapping English names instead of one category.
- Both Chinese requests fail to produce a usable category: one starts an unfinished list, and the
  other repeats characters.
- A screenshot receives its visible heading as a name, which would fragment the scheme around
  individual images rather than recurring useful distinctions.

The candidate is not admitted for automatic category naming. Existing SigLIP encoder evidence and
durable classification behavior remain applicable. Naming, semantic consolidation, stable category
identity and maintenance admission still require a connected implementation and their own evidence.
Private thumbnails, raw outputs, downloaded models and diagnostic package caches remain ignored
workspace data; none are distribution assets or committed fixtures.

## Multilingual naming candidate

The second isolated candidate uses
[Qwen3.5-0.8B](https://huggingface.co/Qwen/Qwen3.5-0.8B), with the Apache-2.0
[ONNX conversion](https://huggingface.co/onnx-community/Qwen3.5-0.8B-ONNX-OPT) pinned to revision
`9a4c21d8f724176b0a7edaaed36303044451886e`. Its Q4 graphs and supporting files occupy 734,883,221
bytes. Transformers.js 4.3.2 and ONNX Runtime 1.31.1 run on one CPU inference thread, with remote
model access disabled and the same 180-second/2-GiB subprocess bounds. This diagnostic dependency
change does not alter the product runtime or admit another distributed model.

| Probe | Inputs | Total time | Peak observed resident memory |
| --- | --- | ---: | ---: |
| Bilingual category name, up to 32 generated tokens | Eight retained representatives | 173.14 s | 1,361,121,280 bytes |
| Short Chinese category, up to 12 tokens | Four previous failure cases | 48.02 s | 1,204,871,168 bytes |
| Shared category, up to 12 tokens | Three groups, three retained thumbnails each | 54.66 s | 1,167,396,864 bytes |

Memory uses the greater of the watchdog samples and the process's per-request observations.

The bilingual probe produces some usable broad artwork labels, but also headings, repeated
formatting and descriptions instead of category names. It labels a chat screenshot as smart
devices/electronics. The shorter request improves that case to a phone-chat category and names a
reaction image correctly; it calls a Python-code screenshot `Java` and leaves a long description
unfinished for a text screenshot. A short output limit constrains cost but does not enforce a useful
semantic name.

The group probe supplies three members rather than relying on one representative. It names the
code/documentation group `代码工程`, the text-card group `科幻作品`, and the reaction-icon group
`快乐表情合集`. Inspection of the retained members rejects the science-fiction interpretation and
shows that the last name is narrower than the group's varied expressions. Supplying group context
alone does not resolve naming or granularity. These runs remain diagnostic examples rather than
accuracy measurements or an admitted naming policy.

## Semantic support of generated names

The next probe reuses the 240 native image vectors and the admitted SigLIP INT8 text encoder. It
encodes 16 candidate/control names and the 24 existing primary descriptions individually, using the
same tokenizer and ONNX Runtime 1.31.1. It opens neither original media nor holdouts. Cosines below
are diagnostic means, not probabilities or production admission thresholds.

| Retained group | Members | Name | Mean cosine | Members above their best existing description |
| --- | ---: | --- | ---: | ---: |
| Code/documentation | 19 | 代码截图 | 0.1391 | 17/19 |
| Code/documentation | 19 | 文字截图 | 0.1211 | 11/19 |
| Text cards | 16 | 文字截图 | 0.1245 | 16/16 |
| Text cards | 16 | 代码截图 | 0.1233 | 16/16 |
| Text cards | 16 | 科幻作品 | 0.0995 | 10/16 |
| Reaction icons | 8 | 快乐表情合集 | 0.1057 | 0/8 |

Semantic similarity does not reliably reject the generated errors. Both code and text names exceed
the existing descriptions for every text-card member; even the unsupported science-fiction name
exceeds them for ten members. English and Chinese versions also differ substantially. A policy
that admits a name merely because it beats the starting vocabulary would publish incorrect or
redundant categories. The existing broad descriptions already fit the reaction-icon group better
than its generated name.

The next policy boundary must separate recurring visual groups, observed content descriptions,
useful category names and admission against neighboring categories. Existing categories and human
choices remain authoritative while a candidate is evaluated. No generated category or automatic
taxonomy replacement is admitted by these probes. A finite sample list, manual naming prerequisite
or score-only gate remains insufficient.

## Caption-based discovery check

A separate probe tests whether a model trained for image description supplies better evidence
before category naming. It uses the MIT-licensed
[Microsoft Florence-2-base-ft](https://huggingface.co/microsoft/Florence-2-base-ft) and its
[ONNX conversion](https://huggingface.co/onnx-community/Florence-2-base-ft), pinned to
`e88a44eaf3791a35eae0c5a47b3dbcd36e67eb6f`. The four INT8 graphs and supporting files total
277,619,471 bytes. Downloaded model files match the pinned artifact digests; an interrupted vision
file is resumed and fully verified before inference.

The same eight retained representatives use the model's `DETAILED_CAPTION` task with its bundled
image processor, one beam and at most 48 generated tokens. Transformers.js 4.3.2 and ONNX Runtime
1.31.1 run on one CPU inference thread under the existing subprocess time/memory bounds. The run
finishes in 37.31 seconds, including 2.11 seconds to readiness; individual requests take 3.99–4.89
seconds. Peak sampled resident memory is 1,638,424,576 bytes. These are local diagnostic measurements,
not cloud-primary acceptance or product scheduling results.

The captions describe screens, text and drawings without the earlier incomplete category-name
formatting. However, the code representative B132 receives only a generic text description, and the
reaction-icon representative B054 is described as a logo. The descriptions do not preserve the
distinctions needed for useful purpose-aware categories. Translating or shortening those captions
cannot recover the missing evidence, so the dependent caption-to-name probe is not executed.

This candidate is not admitted. Existing encoder, persistence and dispatch evidence remains valid;
automatic category maintenance is still an implementation and admission gap.

## Context, input isolation and visual precision

The next Qwen probe compares two neighboring three-image groups while supplying the six existing
category descriptions. It can keep an existing category or propose two names. The screen comparison
uses B132/B007/B231 and B037/B005/B239; the artwork comparison uses B039/B008/B235 and B186/B065/B238.
Each image has 65,536 processor pixels, and generation is bounded to 48 tokens. The input budget,
single-thread runtime and subprocess watchdog remain unchanged.

| Vision precision | Screen comparison | Artwork comparison | Total time | Peak sampled resident memory |
| --- | --- | --- | ---: | ---: |
| Q4 | Splits the screens into illustration and photography | Splits standalone artwork into illustration and memes | 30.30 s | 1,625,182,208 bytes |
| FP16 | Echoes the requested category-name placeholders | Repeats the unsupported illustration/meme split | 32.91 s | 2,054,471,680 bytes |

The FP16 comparison changes only the vision graph from the same pinned conversion. Its graph and
external data add 204,985,940 verified bytes; embedding and decoder graphs remain Q4. The input,
prompt, generation settings and representative groups are identical. Higher visual precision does
not resolve the observed naming failure and increases memory substantially.

A separate vision-boundary diagnostic compares B132 and B039 individually, together, in reverse
order, and with the first image duplicated. All five corresponding feature comparisons are exact:
cosine 1 and maximum absolute error 0. Individual outputs have shape 64 × 1,024 and the paired output
128 × 1,024. This establishes isolation of the tested visual batching boundary; it does not verify
every decoder or prompt behavior. The diagnostic completes in 5.66 seconds with 473,477,120 bytes
peak sampled resident memory.

These controlled comparisons reject the tested generative naming policy. No Qwen or Florence model
is admitted to the product. Further work uses the retained SigLIP features to investigate candidate
concept retrieval and consolidation, with library evidence determining active categories rather
than exposing a dictionary as the user's category table.

## Lexical retrieval and whole-content context

The lexical probe tests candidate retrieval without another image model. It uses the pinned
[OMW English WordNet 2.0](https://github.com/omwn/omw-data/releases/tag/v2.0) and
[Chinese Open WordNet 1.1](https://github.com/bond-lab/cow/releases/tag/v1.1) archives. The combined
download is 14,190,860 bytes. Both release digests are verified before parsing:

| Archive | SHA-256 |
| --- | --- |
| `omw-en-2.0.tar.xz` | `0e09dfb7f096bc3f10b9de68ffecf13839fa22ae46fd9b227cec890d204ca1dc` |
| `cow-1.1.tar.xz` | `93e9357dda50a38ce626904c2229479afa177df74d9c504aebfeba06ff2f241e` |

The WordNet and COW licenses permit use and modification with their notices and disclaimers retained.
The original license files accompany the ignored derived diagnostic data. The lexicons, Python
parsing and resulting vectors are not admitted production dependencies or distribution assets.

The English archive supplies 82,115 noun concepts; 28,707 have linked Chinese nouns. The first
bounded retrieval set contains 2,445 branching or communication concepts. This selection tests
retrieval behavior, not complete vocabulary coverage. The two variants encode each English headword
in an image prompt, with the second additionally including its dictionary definition. Both reuse
the 240 native calibration image vectors and the existing INT8 text graph, one description at a
time. No source media or held-out partition is read.

| Text variant | Encoding time | Peak sampled resident memory |
| --- | ---: | ---: |
| Headword in an image prompt | 88.37 s | 386,539,520 bytes |
| Same headword and dictionary definition | 85.49 s | 396,386,304 bytes |

The first run reports a runtime telemetry persistence warning. The second sets
`ORT_DISABLE_TELEMETRY=1` before loading the runtime and has no such warning. These are isolated
diagnostic timings, not the native worker's resource acceptance or new product confidence bands.

Direct top-five voting finds `source code` in 16 of the 19 code/documentation members, but also
selects ASCII characters for artwork and an asterisk for text cards. Adding dictionary definitions
does not resolve that error: asterisk is the top result for 49 of 240 images. Generic noun retrieval
needs whole-image purpose and context, not just word-sense disambiguation. The translated dictionary
also lacks several modern image-use concepts, and some short Chinese nouns are unsuitable display
names. It cannot become the product's category table unchanged.

A further diagnostic combines the existing broad-content evidence, whole-content ontology branches,
contrast against neighboring images and agreement between the two lexical views. It requires a
candidate to improve on the existing descriptions for a majority of a repeated visual group. These
are exploratory conditions; their parameters and branch selection are not admitted production policy.
The resulting `source code` candidate persists at all three tested grouping levels, 0.75, 0.68 and
0.60. At 0.68 it exceeds the previous descriptions for 13 of 19 members. The earlier asterisk and
ASCII-character candidates are excluded, and the artwork/reaction groups retain their existing
broad categories in this probe.

Photograph proposals expose the remaining granularity problem. The B021 group produces plaster,
wall and building-material candidates; B057 produces workshop, room and chair. Inspection of the
retained representatives shows an unfinished interior in B021 and a child playing with construction
toys in B057. These related objects do not yet supply satisfactory scene/use categories. The probe
does not measure recognition of every group member or establish category accuracy.

The next admission boundary is whole-scene/use naming and consolidation, with an explicit fallback
to the retained category when the proposed distinction is unsupported. Do not expand the active
scheme from these raw noun candidates, add another mandatory setup table, or repeat the same
generative naming comparisons. Existing screening, persistence and dispatch evidence remains valid.

## Photo context and missing lexical coverage

This check reuses the retained B021 and B057 calibration groups, existing model artifacts and native
image features. It reads seven retained thumbnails, not original media or held-out samples. Florence
2 INT8 detailed captions take 32.906 seconds, including 1.851 seconds of readiness, with observed
peak working set 1,650,778,112 bytes. They describe walls, openings, wiring and a sign in B021, and
children, tables, toys and an indoor desk in B057. Direct inspection confirms that the groups contain
different settings and uses; visual grouping alone does not justify one new shared category.

Two bounded text-only checks pass these captions and existing lexical evidence to the retained Qwen
0.8B Q4 candidate. The parent-context version takes 10.047 seconds and peaks at 1,008,189,440 bytes,
returning generic real-life/photo names. The scene-specific version takes 15.265 seconds and peaks
at 992,505,856 bytes, returning a formatted architecture/structure answer and an English workshop
label. Neither supplies a dependable product naming policy. Both complete under the existing
180-second/2-GiB watchdog. No additional generator or runtime is selected from these checks.

The initial 2,445-concept selection omitted useful leaf scenes such as `classroom` while retaining
`workshop`. A bounded expansion chooses 62 contrast-ranked seeds across the retained photograph
groups and traverses parents and children to distance two, stopping expansion through the general
noun roots. It adds 1,409 translated concepts without a hard-coded classroom seed. Encoding the
additional descriptions takes 51.776 seconds and peaks at 391,450,624 bytes. All 1,409 output rows
match the input digest `1d15916ec99b5fb68c12b0b2888bf08fc8fe67d66182d65c64579ad3be2784c8`.
Combined retrieval contains 3,854 concepts and uses the same 240 retained image vectors.

For B057, child's room and classroom have mean cosine 0.1076 and 0.1072, ahead of workshop at 0.0935.
Within the existing broad photographic context, classroom and child's room also lead the contrast
ranking, at 0.0534 and 0.0522. This establishes a coverage defect in the first candidate selection.
It does not select either label for the whole group: the retained members include classroom and
home settings. B021 still favors cavity wall, concrete, grout and plaster, and some unrelated lexical
senses remain competitive. Expanded coverage improves candidate retrieval while whole-scene naming,
useful granularity and consolidation remain separate unresolved requirements.

The reproducible ignored inputs and outputs are `photo-context-caption-probe`, `scene-naming-probe`,
`scene-specific-naming-probe`, `lexical-expansion-policy.json` and
`lexical-expansion-retrieval.json` under the existing discovery workspace. Resource limits, seed count,
distance, contextual routing and clustering settings in these probes remain exploratory parameters.
No source bytes, human decisions or active category definitions changed.

## Native bounded retrieval

The native retrieval owner now connects a versioned vocabulary port, retained compatible image
features, contextual ranking, bounded graph expansion and the existing description cache/worker.
It retains candidate evidence without modifying categories or human choices. Thirteen focused native
checks pass: four cohort invariants, five retrieval/cache/lifecycle cases and four existing description
preparation cases. They cover duplicate-content exclusion, mean scoring, semantic compatibility,
leaf discovery through an already ranked intermediate node, cycles, page/work limits, stable ties,
cancelled work, changed vocabulary, cache reuse and worker retirement.

The initial test invocation selected additional unrelated filesystem-discovery tests and encountered
the sandbox's namespace ACL guard before cache setup. The narrower classification checks pass under
the ordinary Windows user context with repository-owned temporary storage. The safety guard and
test expectations are unchanged. Native diagnostic and worker builds also pass after the linker is
run outside the workspace sandbox's output-file restriction.

The real-model probe uses the production retrieval entrypoint, SQLite feature cache and supervised
SigLIP text worker. Its 15,232,181-byte retained calibration fixture contains 82,115 lexical nodes,
2,445 entry concepts, 240 image vectors and the B021/B057 cohort/comparison sets. The input SHA-256 is
`fbe3533a98e229d9b72ba80f8bede89f7ec6a5f7610eef62b8d24296c5171113`.
Lexical identifiers are translated inside the diagnostic adapter to opaque Ame identities; the
fixture and third-party lexical data remain outside Git.

The Debug-worker run reaches its 360-second diagnostic deadline before returning a complete cohort.
The watchdog stops it at 360.188 seconds with a peak process-tree working set of 746,381,312 bytes.
Its cache retains 2,689 completed entries, accounting for 8,948,992 bytes. No source image or held-out
sample is read. This attempt provides resource and interruption evidence. Its completed entries are
reused by the Release diagnosis below; the failed cold run is not repeated.

### Native cache cost and completed retrieval

The Release cost diagnostic separates cache lookup, encoding and storage, using retained and missing
descriptions from the same fixture. The first run records 98 cache hits averaging 5.534 ms, 32 stores
averaging 9.226 ms and 31 post-initialization encodings averaging 143.965 ms. Its first encoding,
including worker initialization, takes 3.855 seconds. Total time is 11.203 seconds, peak process-tree
working set is 638,332,928 bytes, and retirement and exact cache round trips pass.

A controlled follow-up uses two disposable copies of the same retained cache and the same 32 missing
descriptions. The normal background priority averages 49.183 ms per post-initialization encoding;
normal process priority averages 49.555 ms. Whole-run times are 5.109 and 5.094 seconds. Changing only
the owned diagnostic worker's priority does not explain the earlier variation. The product keeps
its admitted background priority, single-description batching, runtime and thread policy.

The complete Release retrieval then finishes both cohorts and their cache-only repeats in 141.281
seconds, with a peak process-tree working set of 757,559,296 bytes. B021 scores 2,994 concepts over 782
edges; B057 scores 3,248 over 1,182 edges, without exhausting a work bound. B057 includes preschool,
infant school, children's room and classroom; B021 retains overly specific wall/material names and
wrong word senses. This completes native retrieval verification while leaving useful-name admission
and taxonomy maintenance open.

The cache owner previously committed recency separately for every hit. Description preparation now
loads at most 64 keys per transaction, preserving exact feature bytes, requested order and durable
recency. Missing descriptions are still encoded and stored individually outside that transaction.
The bounded window is shared by ordinary category rematching and candidate retrieval.

| Retained cohort | Concepts | Cache-only before | Cache-only after | Ratio |
| --- | ---: | ---: | ---: | ---: |
| B021 | 2,994 | 22.066 s | 1.223 s | 18.04× |
| B057 | 3,248 | 24.792 s | 1.525 s | 16.26× |

All four before/after candidate lists, scores, visited-edge totals and bound results match exactly.
The optimized four-pass run takes 6.594 seconds and retains 3,561 cache entries without adding an
entry. Its 53,620,736-byte peak is a cache-only observation, not a loaded-model memory reduction.
The timing improvement applies to compatible retained features; it does not measure first-time
vocabulary encoding. No source image or held-out sample is read, and all observed runs retire their
workers. Detailed fixtures, per-operation timings and comparisons remain in ignored derived storage.

Twenty-one affected native checks pass after the window change: eleven cache cases, five description
preparation cases and five retrieval cases. New cases cover missing/duplicate keys and requested
ordering, oversize rejection, persisted recency and eviction, rollback after corruption, and matching
across multiple windows. Existing cancellation, committed-feature reuse, mixed identities and warm
image-worker retention checks pass unchanged. The four cohort-domain checks from the preceding
retrieval change remain applicable. No cache schema, codec, model revision or journal durability
policy changes. The successful ordinary-user test/build runs report no warnings; an earlier sandbox
diagnostic build reported incremental hard-link fallback and is not the passing build evidence.

| Owner | Production lines | Inline test lines | Dedicated test lines |
| --- | ---: | ---: | ---: |
| Discovery concepts and cohort scoring | 193 | 0 | 102 |
| Retrieval lifecycle / bounded search / retained rankings | 208 / 135 / 61 | 0 | 406 |
| Shared description feature preparation | 130 | 0 | 342 |
| Feature-cache facade / validated read window | 265 / 66 | 0 | 324 / 103 |
| Native discovery / cost diagnostics | 254 / 95 | 0 | Uses the production retrieval/cache boundaries |
