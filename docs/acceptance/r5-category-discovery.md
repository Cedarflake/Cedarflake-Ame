# R5 category discovery evidence

The manual-classification workflow and human correction-time acceptance were withdrawn on
2026-11-01. Historical references below to integrated human-work verification retain their original
scope; current acceptance follows [R5 requirements](../product/r5-classification.md). Category
discovery, automatic maintenance, measured quality, and legacy-decision protection remain applicable.

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

## Sense-aware coverage and semantic consolidation

The retained photographic calibration cohorts expose two different naming problems: missing activity
concepts and ambiguous dictionary senses. A bounded sense check selects 82 nearest/distinctive seeds,
then their immediate parents and shared-lemma senses. It encodes 409 missing descriptions across 268
concepts, combining the lemma with its dictionary definition in a photographic context. Encoding
takes 18.966 seconds with peak working set 390,684,672 bytes; all rows match input SHA-256
`5bcb95cf4c63da6db2d2151df1d7aed6768353474c4518a7664381e5f027f9d6`.
The financial sense of `actual damages` leaves the first twelve results, and B057 includes play.
Local object/material and unsupported fine-grained candidates remain, so the check establishes the
need for sense-aware descriptions without admitting dictionary ranking as a naming policy.

The expanded native fixture adds translated activities and branching concepts to the initial
entry selection. Its 3,921 entries cover the same 82,115 nodes and retained image vectors. The
15,261,699-byte fixture has SHA-256
`988d2baf65a2446b603b9eb9fee2895262ca237eb88875fca7d63c14c693af9d`.
After the sandbox namespace guard rejects cache opening, the ordinary-user run completes in 86.641
seconds with peak process-tree working set 656,822,272 bytes. B021 scores 4,145 concepts in 71.848
seconds and repeats in 1.330 seconds; B057 scores 4,347 in 11.392 seconds and repeats in 1.734 seconds.
The cache grows from 3,561 to 4,947 entries. Repair and classroom activities become reachable, while
raw financial/material candidates still show why retrieval needs a separate consolidation boundary.

The native consolidation owner follows the distinct broader-context relation, stops at the supplied
naming frontier, and scores reached contexts using their own descriptions. Seven new boundary cases
pass together with the five retained retrieval cases. They cover independent rescoring, duplicate-word
deduplication, unresolved concepts, cycle termination, graph budgets, incomplete retrieval, changed
cohort/vocabulary/definition authority, cancellation, retirement and frontier identity. Broader
concepts never inherit a narrow candidate's score or count lexical paths as additional image votes.

The real-model diagnostic uses seventeen exploratory activity/scene/subject contexts to exercise
this boundary. They are supplied by the fixture, not an admitted product class list or automatic
frontier-selection policy. The 18,213,872-byte fixture contains 3,924 entries and has SHA-256
`d0d69e5978ae5f24492fc515745448d6489047f35bb6327a97b6cbdb68e87756`.
The Release build and four-pass probe pass without warnings. Total probe time is 11.579 seconds,
peak process-tree working set is 694,767,616 bytes, and the cache grows from 4,947 to 4,966 entries.

| Cohort | Scored concepts | Consolidated first candidate | Context contrast | First pass | Cache-only repeat | Unresolved lexical candidates |
| --- | ---: | --- | ---: | ---: | ---: | ---: |
| B021 | 4,299 | 建筑施工 | 0.05113 | 4.146 s | 1.128 s | 52 |
| B057 | 4,350 | 儿童游戏 | 0.04390 | 3.459 s | 1.379 s | 61 |

B021 also retains room, repair, renovation and building contexts; B057 retains teaching activities.
Consolidation traverses 371 and 322 edges respectively, adds approximately 6–10 ms per pass after
retrieval, and reproduces identical candidates and scores on each cache-only repetition. All passes
finish within their resource bounds and retire the worker. The timing includes new descriptions
and retained cache entries; it does not measure a wholly cold vocabulary installation.

These are exploratory calibration observations. The manually chosen frontier and provisional
cohorts cannot establish automatic naming quality or whole-group membership: B021 includes a finished
corridor and B057 includes a home desk scene. Automatic frontier selection, useful-name admission,
coverage and category maintenance remain required before first-use integration. All new checks use
retained image features, read no original media or held-out samples, and change no category or human
decision. Inputs, resource records and JSONL results remain in ignored discovery storage.

| Affected owner | Production lines | Inline test lines | Dedicated test lines |
| --- | ---: | ---: | ---: |
| Retrieval lifecycle | 213 | 0 | 408 |
| Consolidation lifecycle / graph frontier | 173 / 116 | 0 | 244 |
| Native discovery diagnostic | 314 | 0 | Uses the production retrieval/consolidation/cache boundaries |

The preceding `20cb3978654aedc41c8fe6390de942eadf9c5051` completes the ordinary
[hosted Windows gate](https://github.com/Cedarflake/Cedarflake-Ame/actions/runs/41235559649), including
the corrected title branch, Flutter, native partitions, scan/accessibility integrations, synthetic
workloads and unsigned Release. The three signing/release jobs are intentionally skipped. That run
covers the preceding retrieval/cache and sidebar commits; consolidation requires its own pushed-head
gate.

### Automatic frontier diagnosis

The complete photographic sense fixture applies the same lemma-plus-definition description to all
lexical nodes, retaining the 3,921 entry concepts. The 21,803,744-byte input has SHA-256
`0f35020ae37704d6dbab916f212a9e224e1dedc9e8a5b734993882c034a23abe`.
The native diagnostic now allows a result-window choice within the existing 1–64 bound, independently
of whether consolidation is enabled. With 64 results per ranking, the four-pass Release run takes
342.141 seconds and peaks at 763,994,112 bytes. It adds 4,746 description features to the retained
cache. B021 scores 4,545 concepts in 324.609 seconds and repeats in 1.422 seconds; B057 scores 4,136
in 13.805 seconds and repeats in 1.213 seconds. No work bound is exhausted and every worker retires.
Sense descriptions remove the earlier financial ambiguity but still favor specific materials and
activities, including claims that the photographs cannot establish.

A separate automatic-frontier diagnostic traverses broader relations to four hops from these
nearest/distinctive results. It ranks eligible translated contexts by distinct lexical origins,
with at most 32 selected per cohort, and uses the existing native consolidation path to rescore
their own descriptions. The union contains 63 concepts; no category names are manually chosen for
this run. Its 24,757,162-byte fixture has SHA-256
`1775a047c72ffaea624fb100e4b62423ac88985d6bd102aed226e38252b365fc`.
The complete run takes 9.313 seconds, peaks at 454,201,344 bytes and adds 21 cached descriptions.
Cache-only repetitions take 1.508 and 1.354 seconds. The first returned names remain pipes for B021
and schoolwork for B057; unrelated candidates also remain competitive. Dictionary labels contain
ambiguous translations, and lexical-origin counts omit potentially useful contexts with few links.
This frontier-selection policy is not admitted. Useful-name admission needs image-level coverage,
competition and stable meaning; the number of related words cannot establish those properties.
These checks reuse calibration features without original-media reads, held-out evaluation, another
model, category publication or changes to human decisions.

### Per-content consolidation evidence

Consolidation now retains each member's own similarity and exact-content identity alongside the
aggregate score. The domain keeps the existing combined 512-sample limit and sorts member evidence
by content identity; matching input order does not become publication authority. Six cohort tests
and twelve retrieval/consolidation tests pass. The new cases keep an unsupported member visible
despite the aggregate, preserve identity/order, reject incompatible text pipelines and dimensions,
and verify that the application reports the broader description's member scores.

The Release probe reuses the unchanged automatic-frontier fixture and its 9,733 cache entries.
Its four passes complete in 6.093 seconds with peak process-tree working set 75,042,816 bytes.
B021 returns 104 image scores across 26 contexts; B057 returns 69 across 23. Content identities match
the fixture's complete member sets, and each member mean agrees with its aggregate within `1e-7`.
All prior candidate lists, scores and retrieval bounds are preserved exactly. No cache entry is
added, no image is read or encoded, and every pass retires its worker. Per-pass consolidation time
including member evidence is 12–19 ms; these are cache-only observations, not cold-model timings.

Member evidence enables subsequent coverage and competition checks. It does not admit the rejected
frontier policy or make raw similarity a probability. Existing categories and human decisions remain
unchanged. The comparison record is retained as `native-member-comparison.json` beside the probe
outputs in ignored discovery storage.

| Affected owner | Production lines | Inline test lines | Dedicated test lines |
| --- | ---: | ---: | ---: |
| Discovery concepts and cohort scoring | 235 | 0 | 163 |
| Consolidation lifecycle | 179 | 0 | 247 |
| Native discovery diagnostic | 329 | 0 | Uses the production retrieval/consolidation/cache boundaries |

### Candidate coverage and competition assessment

The application now assesses consolidated candidates against the current taxonomy through its
existing matcher. Each exact-content member records improvement over its current category and
separation from the strongest competing context. Assessment retains rejected members and deferred
candidates, including name conflicts and the proposal budget. The result binds semantic taxonomy
and display-name revisions separately: renaming preserves matching but invalidates old name checks.
Nine admission regressions pass, covering partial support, coverage, ties, ordering, invalid/stale
evidence, cancellation, policy revisions, name conflicts and rename authority. The twelve existing
retrieval/consolidation cases also pass during this change. The Release diagnostic builds, and the
repository format check reports 685 files with no changes.

The native probe prepares the six retained screening definitions as its comparison baseline and
applies an exploratory profile requiring at least two supporting contents and 50% coverage, with
strictly positive improvement, separation and contextual contrast; it permits at most eight
proposals. These diagnostic values are not product defaults or a calibrated confidence policy.
The photo input has SHA-256
`0be844998559a09fad51cfc81a08571e291634bef799ecdf44964620028c638b`; the code input has
`592377ca97a729e87cde73baa5f88194713a6374407c3a9f104028e7529817b7`.

| Calibration cohort | Assessment outcome | Preserved unsupported contents |
| --- | --- | --- |
| B132, seven code screenshots | `代码` is supported by five members; `编译器` has only one and is deferred | B100 has a competing context; B219 does not improve on its current category |
| B021, four interior photographs | No proposal; pipe, window, doorway and corridor each win only one member | All four retain their current category |
| B057, three play/desk photographs | `游戏` is supported by B057 and B230 | B103 supports another context and retains its current category |

Photo retrieval plus assessment completes four passes in 8.812 seconds with a peak process-tree
working set of 656,515,072 bytes, including 24 new baseline-description features. The two code passes
take 27.141 seconds and peak at 659,750,912 bytes; 533 new descriptions enter the retained cache.
The code cache-only repetition takes 0.861 seconds. Assessment itself takes 0.084–0.190 ms per cohort.
All workers retire, all work bounds are respected, and repeated decisions/member evidence match
exactly. Earlier photo retrieval/consolidation scores remain unchanged. These runs reuse retained
calibration features, read no original media or held-out samples, and publish no categories or
human decisions. Inputs, outputs and the checked comparison remain in ignored discovery storage as
`native-admission-photo*`, `native-admission-code*` and `native-admission-comparison.json`.

The code frontier uses the two concepts retained by the earlier calibration probe; the photo
frontier retains the exploratory lexical-origin policy. This verifies the assessment boundary,
not automatic frontier/name selection. Sensitivity computed from the same scores retains all five
code supporters at a 0.002 separation, but only one play supporter. Select and validate the operating
profile before integration; do not infer broad quality from these three provisional cohorts.

| Affected owner | Production lines | Inline test lines | Dedicated test lines |
| --- | ---: | ---: | ---: |
| Candidate assessment / immutable profile | 283 / 70 | 0 | 349 |
| Discovery concepts and cohort scoring | 239 | 0 | Existing 163 |
| Native discovery diagnostic / assessment diagnostic | 342 / 135 | 0 | Exercises production matching, retrieval and assessment |

### Automatic context nomination

The native nomination owner now collects broader contexts without a supplied frontier, scores their
own descriptions against existing assignments, and selects bounded additional member coverage.
Qualifying broader descriptions subsume finer ones only when they retain those members; closed
relation cycles preserve a candidate. The shared graph owner retains the existing frontier traversal
for callers that explicitly supply one. Thirty-one application and seven domain regressions pass,
including unchanged consolidation, cache reuse, cancellation, work limits, current-taxonomy retention,
scope authority and traversal through concepts that cannot become category names. Release builds
without warnings. This establishes the native evidence boundary, not a product operating profile.

The first automatic probe retained up to 64 high-coverage contexts independently. Similar names split
support and no candidate passed final admission. Coverage-aware subsumption retained code and play
contexts, but also selected the abstract `上限` sense of ceiling. Filtering only lexical domains did
not remove it: the vocabulary places that sense in communication, with economic policy and reasoning
ancestors. The final adapter fixture excludes senses in abstract domains and their descendants from
naming, while preserving their graph paths. Naming scope belongs to the concept revision; admission
also rejects traversal-only concepts. Description features remain reusable.

The final fixtures retain 59,756 naming candidates and 22,359 traversal-only concepts. They use the
same calibration samples, descriptions and exploratory admission profile as the preceding assessment,
with no supplied category frontier. Photo fixture SHA-256 is
`d5b0c827962634d04ba59acce1bd40bce33c27489ab0732b9e9f3392e0cb5944`; code fixture SHA-256 is
`a3741296357861090d249a9475565a678a61848ade4927c96570834805d3196b`.

| Cohort | Reached / nameable / eligible contexts | Selected name | Supported contents | First / repeated pass |
| --- | ---: | --- | ---: | ---: |
| B021, interior photographs | 167 / 129 / 115 | `repair` | 4 of 4 | 1.085 / 1.052 s |
| B057, play/desk photographs | 172 / 145 / 106 | `游戏` | 3 of 3 | 1.121 / 1.505 s |
| B132, code screenshots | 164 / 116 / 18 | `代码` | 6 of 7 | 0.889 / 0.874 s |

The four photo passes take 5.234 seconds and peak at 79,007,744 bytes; the two code passes take
2.313 seconds and peak at 71,344,128 bytes. Nomination plus assessment adds 31–67 ms per pass after
retrieval. All 10,337 cached features are reused, repeated decisions and member evidence agree exactly,
and earlier retrieval results remain unchanged. Resource limits are respected and workers retire.
These cache-only runs read no original media or held-out samples and publish no categories or human
decisions. Inputs, failed variants and checked output remain in ignored discovery storage under
`native-automatic-*`, `native-frontier-*`, `native-nameable-*`, `native-grounded-*` and
`native-nomination-comparison.json`.

Support here means improvement over the retained comparison descriptions, not verified correctness.
The completed corridor remains inside the repair proposal, and the desk remains inside play;
the untranslated `repair` label also needs naming coverage. B219 retains its existing classification.
The code fixture still uses term-only descriptions, whereas the photo fixture includes definitions.
These observations narrow the remaining operating-profile, localized naming and useful-scene policy
work; they do not establish broad automatic naming quality or a final accuracy percentage.

| Affected owner | Production lines | Inline test lines | Dedicated test lines |
| --- | ---: | ---: | ---: |
| Nomination / coverage selection | 249 / 127 | 0 / 54 | 326 |
| Shared context graph / consolidation | 164 / 171 | 0 | Existing 247 |
| Candidate assessment | 284 | 0 | 362 |
| Discovery concepts and cohort scoring | 260 | 0 | 179 |
| Native diagnostic / naming diagnostic | 331 / 138 | 0 | Exercises the production nomination and assessment boundaries |

### Retained competition after nomination

The preceding nomination discarded alternative descriptions when it reduced the name set. Final
assessment then counted images with a stronger alternative as support for the surviving broad name.
The application now retains each member's strongest eligible, semantically distinct alternative,
including its identity, image similarity and text relatedness. Closely related descriptions do not
split support merely because only one name survives. Complete-context comparison has an explicit
scope: missing some or all comparison rows cannot silently become selected-frontier comparison.
The existing frontier workflow remains supported without that additional scope.

Thirty-three discovery regressions pass, covering the pruned-alternative case, related-description
behavior, comparison identity, malformed/incomplete evidence, cancellation, cache reuse and the
previous retrieval/consolidation boundaries. An initial synthetic-test expectation used a cutoff
that left another distinct competitor active; the fixture's cutoff was corrected to match its
intended related-description case. The final complete focused run and Release build pass.

The bounded cache diagnosis compares text relatedness without another model, prompt change, source
read or held-out sample. Cutoffs from 0.88 through 0.92 preserve six code supporters while deferring
the two photographic proposals. The native run uses an explicit exploratory cutoff of 0.90; it is
not a product default or a calibrated probability. The complete eligible-context comparison confirms:

| Cohort | Retained name | Supported contents | Assessment |
| --- | --- | ---: | --- |
| B021 | `repair` | 0 of 4 | Deferred; each member has stronger distinct contextual evidence |
| B057 | `游戏` | 1 of 3 | Deferred; B230 and B103 have stronger distinct contextual evidence |
| B132 | `代码` | 6 of 7 | Proposed; B219 retains its existing classification |

Photo fixture SHA-256 is `09ac598c83015b721d08eeaf8dbe8247343bcf38a0eea7eec21b84d92b7fe23d`;
code fixture SHA-256 is `cb3409ffc5aff218c894344ff8e105dec3dec95d94ee8669bc4d028bf31af5a5`.
The four photo passes take 6.063 seconds with peak process-tree working set 75,255,808 bytes; the two
code passes take 2.547 seconds and peak at 70,754,304 bytes. Nomination plus assessment takes 37–99 ms
per pass. All 10,337 cached features are reused, retrieval and nominated-description scores remain
unchanged, repeated assessments agree, work bounds hold and workers retire. These are cache-only
measurements, not cold-model timings. No category, human decision or original media is changed.
Raw results and the checked comparison remain in ignored discovery storage under
`native-competition-*`, with the preceding diagnosis in `context-competition.json`.

The repair closes the lost-competition boundary. The retained three provisional cohorts do not
establish a complete operating profile or useful naming coverage for the collection. Broader
calibration, localized category naming and guarded maintenance remain required before first use.

| Affected owner | Production lines | Inline test lines | Dedicated test lines |
| --- | ---: | ---: | ---: |
| Context comparison evidence | 201 | 0 | Covered by nomination and assessment cases below |
| Nomination / assessment | 274 / 294 | 0 | 418 / 416 |
| Native diagnostic / naming / assessment | 334 / 154 / 141 | 0 | Exercises the production comparison boundary |

### Shared-media naming coverage

The next check fixes the retained complete-link threshold at 0.75 and evaluates all 28 supported
cohorts, covering 139 of the 236 distinct calibration contents. The other 97 contents remain outside
these cohorts. No provisional reference label selects a cohort, and no held-out input is used.
Comparisons use the other retained contents sharing the members' current leading categories.
The nomination/admission parameters remain unchanged, including the exploratory relatedness cutoff
of 0.90; this checks generalization beyond the preceding three selected cohorts.

The first attempt reused the photo fixture's `A photograph showing ...` descriptions and retained
photo-specific entry additions. That input was unsuitable for a shared-media check. Part one completed;
part two was deliberately stopped through its owned text worker, which returned
`classification_worker_exited`; parts three and four did not start. Completed cache entries were
retained. This interrupted attempt is preserved as `native-broad-*` and is not a completed evaluation.

The corrected input uses `An image of {noun}. {definition}.` uniformly and the existing corpus-wide
entry rules, without cohort-specific additions: 3,921 entries and 82,115 lexical concepts. The local
Release probe uses the discovery implementation committed in `4d13614c`; its build retains unrelated
native worktree changes. It completes four seven-cohort batches. All 56 passes remain within work
bounds, retire workers, and produce identical repeated retrieval, nomination and admission evidence.
The first passes add 13,156 description features. Batch times are 313.203, 125.735, 111.187 and
148.890 seconds; peak process-tree working set is 792,600,576 bytes. Per-cohort first passes take
7.659–179.932 seconds, compared with 0.951–1.462 seconds on repetition. These timings include native
retrieval and nomination; they are distinct from the earlier cache-only naming timings. Image
features are reused, original-media reads remain zero, and no model download occurs.

Ten cohorts retain nominated candidates; six pass the exploratory numerical admission criteria.
Inspection of their six representative cached thumbnails and retained provisional member notes
finds the following name-utility problems. This is a naming diagnosis, not an independent accuracy
estimate or a replacement for the unfinished human labels.

| Cohort | Proposed name | Supporting contents | Observed naming problem |
| --- | --- | ---: | --- |
| B132 | `command line interface` | 4 of 7 | Code editors and a mobile code viewer are included alongside a terminal; the name is narrower than the content. |
| B191 | `corrigenda` | 4 of 5 | Paper forms and notes do not establish a list of printing corrections. |
| B001 | `公猫` | 2 of 4 | Reaction-image purpose is lost; the proposed animal sex is not established. |
| B186 | `雌狐` | 3 of 4 | Character illustrations are reduced to an animal type without retaining their purpose. |
| B075 | `行` | 3 of 4 | A row of text describes a component shared by screenshots, not a useful whole-image category. |
| B151 | `暹罗猫` | 3 of 3 | Stylized reaction stickers do not establish a real cat breed. |

Raising a score cutoff can suppress these names, but does not supply the missing purpose or suitable
level of abstraction. Raw lexical nomination remains evidence for a naming policy, not an admitted
automatic category scheme. Complete purpose-preserving whole-image naming and useful local labels,
then calibrate the profile and connect guarded maintenance. The existing categories, user decisions
and R4 state remain unchanged throughout this diagnostic.

Corrected fixture SHA-256 values, in batch order:

- `a12f92fca5a9cfe2d550c27ace2d476e16dcabbd4642be1633ef529444cf0422`
- `270ea626dfdc7bfaf95682e751b07a85b7195685304d63d1f7102373184214d1`
- `7c69fd546ba7e0fda86014b0128e0e63ee1ac7eaaa24ba631c83337d1dbb6852`
- `8402f1eb6a9c325ea6c63fc4f9bd8111f38fc97337b300aef7297b80811b92af`

Raw outputs, resource measurements and checked comparisons remain in ignored discovery storage as
`native-general-*`; preparation and comparison are retained in `prepare_native_broad.py` and
`check_native_broad.py`.

### Purpose-bound candidate composition

The native naming contract now binds an existing category's identity and semantic revision to
application-owned label and comparison patterns. The vocabulary supplies a separate subject phrase;
the application projects the bounded, already-retrieved context graph and scores each complete
purpose-qualified description. Original noun scores are not reused as composed-description scores.
Missing subjects and oversized names remain traversable without becoming candidates. Nomination
and final assessment exclude members whose current leading category differs from the bound purpose.
Semantic category changes invalidate the result, while display-only renaming preserves its scope.
This boundary still has no category-write, original-image or human-decision authority.

Twelve domain and 37 application regressions pass, including composition identity, retained raw
features, malformed patterns, size limits, mixed-purpose high-scoring members, stale categories,
raw-description rejection, cancellation and repeat cache reuse. The Release diagnostic and worker
build pass. The read-only formatting check reports 685 files with no changes. These local native
builds retain the unrelated pending native worktree changes; hosted checks own the committed tree.

The targeted real-model comparison reuses the six previously problematic cohorts and their 27
distinct retained image features. No original-media or held-out input is read. All raw descriptions,
entry rules, current categories, graph limits and exploratory numerical criteria remain unchanged.
Four explicit fixture patterns preserve reaction, illustration, screenshot and document purposes;
they are calibration inputs, not admitted product defaults or a fixed category list.

| Cohort | Composed candidate | Supporting contents | Naming observation |
| --- | --- | ---: | --- |
| B001 | `猫表情` | 2 of 4 | Retains reaction purpose and drops the unsupported male-animal claim. |
| B151 | `小猫表情` | 2 of 3 | Retains reaction purpose and drops the breed; the finer age label still needs normalization. |
| B186 | `银狐插画` | 3 of 4 | Retains illustration purpose, but the animal variety is still too specific for the character artwork. |
| B132 | `编译器截图` | 7 of 7 | Retains screenshot purpose, but code editors and a code viewer do not establish a compiler. |
| B075 | `monologue截图` | 2 of 4 | Separates two screenshot supporters from other-purpose members; the term is neither a useful interface category nor localized. |
| B191 | None | 0 of 5 | Existing categories remain; the earlier printing-corrections label is not proposed. |

All 12 passes preserve the exact raw retrieval results from the previous shared-media check. Repeated
nomination and admission evidence agree, no work bound is reached, and every worker retires. Two
otherwise competing members explicitly retain a different purpose. The first composed passes add
1,254 text features and take 8.858–15.102 seconds each; repetitions take 0.948–1.372 seconds without
new features. Four batch times total 83.001 seconds; peak process-tree working set is 787,562,496
bytes. These results establish purpose isolation and cache behavior, not independent naming accuracy.
Useful scene/interface names, local labels and measured maintenance policy remain outstanding.

The reaction, illustration, screenshot and document fixture SHA-256 values are, respectively:

- `2e54f39be3a6123c843b4f72cb91bf52dbdcdc540f27dc5494bbad771fe2ad84`
- `8206ff85537ff1921f6d89a6cc75a95406dd97943c6e6f14c1a61d1dff40e7b3`
- `685c871c3496778fa586205666c05d94a6806e888763a7ce7fc4ab2cb773eceb`
- `5ca50fd9b3cb360cba5c705b297506429a7ab253dd9b8af286e19cab5fb289f0`

Ignored `native-purpose-*` records retain raw evidence, resource measurements and binary digests;
`prepare_native_purpose.py` and `check_native_purpose.py` retain preparation and comparison.

| Affected owner | Production lines | Inline test lines | Dedicated test lines |
| --- | ---: | ---: | ---: |
| Discovery concepts / naming purpose | 290 / 132 | 0 | Existing 179 / new 167 |
| Nomination / candidate assessment | 293 / 310 | 0 | 423 plus new 181 / 417 |
| Native diagnostic / naming / assessment | 339 / 156 / 170 | 0 | Exercises the production composition and admission boundaries |

### Broader ancestry across unsupported intermediates

A bounded 23-description diagnosis compares the retained scores behind the problematic names. It
reuses the three code, fox-artwork and cat-sticker cohorts, adding seven missing text features but no
image inference or source reads. The fox description improves on the current category for all four
artwork members, as does silver fox; the intermediate red-fox description covers only two. The
previous selector considered only immediate qualifying parents, so this intermediate prevented the
broader fox concept from subsuming the finer name. The code comparison also finds support for all
seven members under a programming description, with mean similarity 0.149637 versus 0.149516 for
compiler. That useful topic does not follow the retrieved software-type ancestry; higher scores alone
cannot supply the missing whole-image naming relation.

Selection now visits all already-resolved broader ancestors through unsupported intermediates,
requiring the selected ancestor's own member coverage throughout. Per-origin visited sets preserve
cycles without erasing every candidate. The shared explicit edge limit bounds this additional work;
exhaustion returns no partial selection and retires the description worker while keeping completed
cache entries. Forty-one application regressions and the Release probe build pass, including three
new coverage/cycle/limit cases and the terminal worker/cache boundary. Formatting reports no changes.

The same six purpose fixtures complete all 12 passes with unchanged raw retrieval, repeatable
evidence, zero new text features and retired workers. Ancestor work is 4,982, 8,506, 4,041, 437, 484
and zero edges for B001, B151, B186, B132, B075 and B191 respectively, below the 65,536 ceiling.
Four batches take 18.578 seconds in total and peak at 80,134,144 bytes. The build retains unrelated
native worktree changes; committed-tree verification remains with hosted CI.

Complete ancestry exposes overly broad eligible vocabulary as well as removing narrow labels:
B001 proposes `chordate表情` for three of four members, B132 proposes `coding system截图` for seven
of seven, and B075 proposes `语言截图` for two of four. B151's `discord表情` and B186's
`secretaryship插画` retain zero supporters and are deferred; B191 still proposes nothing. The repair
establishes the traversal invariant, while this exploratory raw-lexical naming policy remains
unadmitted. Whole-image topic authority and useful local labels must constrain displayable candidates
independently of the much wider retrieval vocabulary. Existing categories and human choices remain
untouched; these numerical proposals do not reach smart albums.

Ignored `native-ancestry-*` records retain the unchanged fixture hashes, resource and binary digests,
and checked evidence. The bounded score diagnosis is retained as `native-purpose-ranking-*`, with
fixture SHA-256 `8c14a0b9ba99d93f0d3db238319b0b34103a7196d5a011be0874a0d14303034b`.
The selection owner has 145 production and 109 inline-test lines; nomination has 296 production
lines with 456 existing and 181 purpose-specific dedicated-test lines. The naming diagnostic has
157 production lines.

### Separate retrieval cues from managed naming contexts

The application now composes the source vocabulary with a separate immutable context catalog.
Every raw concept remains available for retrieval and traversal, but only declared contexts supply
names. Context definitions carry local labels, complete descriptions and purpose subjects; explicit
cue links connect them to retrieval without inheriting scores. Ordered merged windows, independent
revisions and disjoint identities retain the existing discovery, cache and cancellation boundaries.
No category, decision, catalog, original image or desktop control is changed by this owner.

The [frozen context fixture](fixtures/r5-naming-contexts.json) contains 42 application-authored
whole-image topics, including scenes, activities, digital content and character contexts. Its
ordinary lexical cues resolve to 714 links in the retained vocabulary. It is a calibration input,
not the active product taxonomy or a permanent category limit. The fixture SHA-256 is
`baa23594bed2a95b51c288f65ebb7d4f0b0e4b8957092c101ff78b9f22d23834`.
The source lexicon, current categories, purpose patterns and numerical admission criteria are
unchanged. This comparison measures the effect of name authority and whole-image descriptions;
it does not retune cutoffs or inspect either held-out partition.

| Cohort | Managed-context result | Supporting contents | Retained observation |
| --- | --- | ---: | --- |
| B001 | `人物肖像表情` | 4 of 4 | Incorrect for the mixed panda-face and cat reactions; purpose-qualified similarity still overstates topic support. |
| B151 | `猫咪表情` | 3 of 3 | Preserves reaction use without a breed or age claim. |
| B186 | `兽耳角色插画` | 4 of 4 | Names the depicted character context rather than a biological fox variety. |
| B132 | `代码截图` | 7 of 7 | Covers editors, terminal code and the mobile code viewer. |
| B075 | None | 0 of 4 | Existing categories remain; no language/row/monologue name is proposed. |
| B191 | None | 0 of 5 | Existing categories remain; the unsupported printing-corrections name stays absent. |

The initial fixture projection adds 41 text features, takes 27.141 seconds across four batches and
peaks at 799,584,256 bytes of process-tree working set. The implemented contextual vocabulary then
reproduces its retrieval, nominations and admission evidence exactly across all 12 passes over six
cohorts and 27 distinct contents. Those passes add no features, take 18.515 seconds across four
batches and peak at 87,363,584 bytes. Workers retire and no work limit is reached. Neither run reads
original media or publishes a category. Retained calibration thumbnails confirm the mixed-reaction
error; these targeted observations are not independent accuracy estimates or corrected user labels.

Forty-nine application regressions pass, including eight new context-catalog and connected naming
cases. They cover merged pagination, raw-name exclusion, empty context catalogs, name/meaning/link
revisions, malformed and oversized catalogs, identity collisions, invalid source windows, source
revision changes, cancellation, independently scored composition and other-purpose members. An
initial new-test assertion incorrectly assumed the top aggregate cue from one member's score; it
was corrected to verify that the cue is retrieved, while preserving the independent whole-cohort
ranking and naming assertions. The Release diagnostic/worker build and read-only formatting check
pass; formatting reports 685 files with no changes. Local builds retain unrelated native worktree
changes, so committed-tree verification remains with hosted CI.

The context owner has 201 production lines and 222 dedicated-test lines; connected purpose tests
now occupy 255 lines. The diagnostic facade/context adapter has 346/99 production lines. There are
no schema, bridge, model, runtime or dependency changes. Context applicability, broader collection
coverage, measured admission and guarded category maintenance remain required before first use.

Managed fixture SHA-256 values, in reaction/illustration/screenshot/document order:

- `fd3e6053eb92cbf3c428531df655c88dff943037520f358cdccc7060bbc74d5d`
- `88aac870c3916dee1ccc4baa00ad9c74146e6f9edd467b7dad2dcdb5a99a4f98`
- `e42147359804148c3f71492dd114eabde9cd88f68c726aeec7308a71eb437508`
- `f456323fdc28104758a5f2b5d08bdcfe3d0faa8a8d2771f33b8f2c4f80680ae4`

Ignored `native-topics-*` and `native-managed-*` retain the projection, implemented comparison,
resources and binary digests. Preparation and verification remain in `prepare_topic_contexts.py`,
`prepare_managed_contexts.py` and `check_managed_contexts.py`.

### Topic competition independent of shared purpose wording

A source-free comparison scores the same 42 context descriptions both unconditioned and with each
existing purpose pattern. It reuses the six cohorts and 27 contents above, adding 169 text features
across 24 passes. Five batches take 15.579 seconds and peak at 718,553,088 bytes of process-tree
working set. Repetitions reuse completed features, retain identical scores and retire their workers.
Neither sealed holdout nor original media is read. This diagnosis changes no descriptions or cutoffs.

For B156 and B046, the qualified cat description scores 0.107720 and 0.146567, above the qualified
portrait description's 0.099201 and 0.121394. The previous comparison nevertheless treated these
topics as related after their common reaction-purpose prefix, dropping that stronger alternative.
The original cat and portrait topic descriptions have relatedness 0.866569, below the unchanged
0.9 comparison cutoff. Topic distinctness now uses those original descriptions; member image scores
continue to use the complete qualified description. Algorithm and frontier revisions advance to v7.
Both description views remain independently cached and bounded, with the existing cancellation and
terminal worker-retirement behavior.

The production comparison reuses the four unchanged managed fixtures and completes all 12 passes.
Raw retrieval, nominated names, qualified scores, purpose scope, current taxonomy and numerical
profile remain identical. The changed evidence is the retained distinct-topic alternative and its
effect on per-content support:

| Cohort | Result after topic comparison | Supporting contents |
| --- | --- | ---: |
| B001 | `人物肖像表情`; B156 and B046 now report `CompetingContext` | 2 of 4, previously 4 |
| B151 | `猫咪表情` retained | 3 of 3 |
| B186 | `兽耳角色插画` retained | 4 of 4 |
| B132 | `代码截图` retained | 7 of 7 |
| B075 / B191 | No candidate, existing categories retained | 0 of 4 / 0 of 5 |

The panda-face reactions still support the portrait name under the exploratory profile. This repair
removes the demonstrated competition defect, while purpose/topic naming quality remains a measured
admission question. No category is published or human decision changed. Targeted calibration
observations remain separate from independent accuracy or corrected user labels.

Four batches take 17.297 seconds and peak at 99,241,984 bytes of process-tree working set. They add
no text features, reach no work limit and retire every worker. Fifty-one discovery regressions and
the Release diagnostic/worker build pass. The two new regressions cover a shared prefix hiding a
distinct competitor, repeated cache-only evidence, and cancellation during topic preparation with
completed features retained and the worker retired. Local builds retain unrelated native worktree
changes; committed-tree checks remain with hosted CI.

The competition and nomination owners have 215 and 296 production lines and no inline tests.
Their dedicated admission and purpose tests have 417 and 347 lines. There are no schema, bridge,
runtime, dependency or source-read changes. Ignored `native-views-*`, `topic-views-reached.json` and
`native-topic-competition-*` retain the diagnosis, repeated comparison, unchanged managed-fixture
hashes and binary digests; `prepare_topic_views.py` and `check_topic_competition.py` retain the checks.

### Managed-context coverage and recurrence candidate

The broader comparison reuses all 28 retained complete-link cohorts at the unchanged 0.75 threshold:
139 of 236 distinct calibration contents, with the other 97 outside the recurring cohorts. Current
model assignments select the eligible purpose, requiring at least two members and half the cohort;
provisional reference labels select neither groups nor purposes. All 28 cohorts have one eligible
purpose: ten illustration, two comic, five reaction, eight screenshot, two photographic and one
other-content cohort. The six preceding diagnostic cohorts are included without changing their
samples. No held-out data, original media or new image inference is used.

The frozen 42-context catalog and existing four purpose patterns remain unchanged. Photographic
and comic contexts use the additional patterns recorded in the
[candidate profile](fixtures/r5-naming-profile.json). These are calibration compositions, not an
active fixed category scheme. With the original two-content support criterion, ten candidates are
proposed with 31 supporting contents. Direct inspection of retained B013 and B166 thumbnails shows
character artwork against buildings or lanterns, not the proposed `代码插画`. B144 contains foliage
around a character without establishing the `树林插画` scene. B027 and B234 do visibly show group
photographs. These are targeted naming observations, not independently corrected user labels.

The recurrence comparison changes only minimum support from two to three distinct contents and
reruns complete nomination and admission. It does not merely filter the previous winners: eligibility,
selection and competing evidence can all depend on support. Descriptions, raw retrieval, context
definitions, model, purpose assignments, 50% coverage, strict-positive gain/separation/contrast and
the eight-proposal ceiling stay unchanged. Its proposed additions are:

| Cohort | Candidate | Supporting contents |
| --- | --- | ---: |
| B186 | `兽耳角色插画` | 4 of 4 |
| B042 | `兽耳角色表情` | 5 of 6 |
| B151 | `猫咪表情` | 3 of 3 |
| B132 | `代码截图` | 7 of 7 |

Nineteen contents support these four candidates. The remaining 120 cohort members retain their
existing broad assignments, as do contents outside the recurring cohorts. The portrait-reaction
candidate still appears in evidence but has only two supporters and is deferred. The code-artwork
candidate no longer meets nomination support. The threshold also defers useful two-supporter
construction and group-photo candidates, along with forest-artwork and game-screenshot candidates;
new independent content can supply further recurrence. It creates no mandatory confirmation work.

Select the three-content profile as the bounded candidate for guarded maintenance implementation.
It reduces speculative additions on this calibration set while preserving the four supported
distinctions. The four observed additions are not a prescribed category count. This selection is
not an accuracy percentage, a confidence band or proof that every purpose/topic combination is
useful; it leaves first-use publication, cumulative scheme budgets, overlap/retirement handling,
human-choice preservation and integrated human-work verification with their owning requirements.
No automatic category changes are enabled by this calibration record.

Both comparisons complete 56 repeatable passes within all work bounds and retire every worker.
The coverage run adds 29 text features, takes 91.422 seconds over six batches and peaks at
788,221,952 bytes of process-tree working set. The three-content run adds no features, takes
86.376 seconds and peaks at 82,804,736 bytes. Image/source reads remain zero. The native code is
`05fe0771`; binaries retain unrelated worktree changes as in the preceding diagnostic. No product
code, model, runtime or numerical score cutoff changes in this comparison.

Ignored `native-managed-coverage-*` and `native-managed-profile3-*` retain inputs, raw results,
resource records, binary/fixture hashes and checked summaries. `prepare_managed_coverage.py`,
`prepare_managed_profile.py` and `check_managed_coverage.py` preserve preparation and assertions.
The profile probe reuses one bounded temporary fixture path serially; each purpose's exact payload
is reproducible from its retained source fixture and hashed preparation record.

### Guarded category addition transaction

The native application now reassesses a cohort against the current matcher and admission policy,
prepares an immutable addition plan, and publishes it through the existing configuration/run
transaction. Managed identity binds canonical concept and optional purpose-category identity;
renaming, changed vocabulary releases and parent wording do not create another category. Lookup
covers all historical category identities, so an existing manual edit or retirement is preserved.
The bounded addition path neither removes existing definitions nor remaps human classification.

Publication checks classification/dispatch revisions, encoder compatibility, and current published
exact-content evidence for every member and comparison sample. Another current verified copy can
retain that evidence. Cancellation before commit rolls back categories, run and receipt together;
an identical committed retry preserves later stop/source removal. Changed category sets receive an
explicit uncalibrated setup, retaining the previous immutable run rather than transferring its
category-specific screening profile. No schema migration, source reads, runtime download or desktop
interaction is involved.

The focused application-to-SQLite diagnostic passes 18 settings/expansion regressions and one stable
identity regression. Cases cover restart, missing/unpublished/changed source evidence, duplicate-copy
survival, concurrent pause/rename, incompatible encoder, manual definition/choice preservation,
independent Favorites, insufficient recurrence, capacity, idempotency, failed activation and
cancellation during run insertion. The first diagnostic exposed three fixture issues: an expected
error-code mismatch, a missing root publication identity, and a cancellation hook on a WITHOUT ROWID
table. Those fixtures were corrected without weakening the assertions; the final run has no warnings.
The fixture support threshold of two exercises the configurable boundary, not a change to the
three-content operating candidate.

The preparation owner has 207 production lines, the SQLite publication owner 171, the shared settings
transaction 135, and the cohort owner 311; each has zero inline test lines. Dedicated new tests and
fixtures have 53, 299, 181 and 161 lines; the existing settings regression owner has 236 lines.
The domain plan and persistence port have 29 and 21 lines. Unrelated desktop/batch worktree changes
remain outside this delivery, and hosted CI owns exact-commit verification.

This establishes the native publication boundary. Cumulative soft budgets, overlap/retirement rules,
retained maintenance provenance, automatic scheduling/first use and integrated human-work measures
remain required. It does not enable managed maintenance in the desktop application.

### Managed origins and portable manual protection

Schema 71 retains an immutable admission record and a durable origin for each generated category.
The record binds the new run, comparison taxonomy and names, vocabulary/policy revisions, actual
criteria/work bounds, naming parameters, cohort/source identities and admitted member support.
It contains no paths, media, embeddings or model binaries, and its encoded payload is bounded to
512 KiB. The origin separately records canonical concept/purpose identity, the admitted definition
and a monotonic manual-protection flag. Categories with no origin retain manual authority.

Definition edits, retirement and later restoration of the original wording keep that protection.
Configuration replay cannot claim newly generated categories or undo later edits. Origin and
evidence insertion share the category/run transaction, including rollback on insertion failure.
Expansion also requires the catalog's active run and configuration to match dispatch; direct rule
replacement cannot reuse an obsolete automatic run.

Portable version 6 includes origins and protection while omitting derived admission evidence.
Restoring an older backup cannot remove protection. An incoming origin does not claim an existing
category with unknown ownership, and retained local names and rules take precedence. Re-export and
restart preserve the same result. Versions 1–5 retain their original record counts and remain
readable; the version-5 case preserves definitions without inventing origins or changing input bytes.

Focused diagnostics use repository-contained temporary SQLite catalogs and no source-media reads,
model downloads or desktop interaction. The initial compile exposed private codec access from the
archive boundary; origin validation now uses the shared domain identity and local format parsing.
The final diagnostics cover 25 settings/expansion cases, two migration cases, three existing
classification recovery cases, 13 archive-inspection cases and the stable-identity case. Tests cover
failed-migration rollback/retry, legacy ownership, full-parameter persistence, stale-run rejection,
committed retry, manual protection and portable recovery. Hosted checks retain full-suite ownership.
The read-only formatting gate passes across 685 Dart files with no changes, alongside Rust format
validation and `git diff --check`.

The preparation owner has 270 production lines, the domain plan/origin owner 118, SQLite origin
persistence 144, evidence encoding 64, publication 182 and shared settings 141, with zero inline
test cases. Dedicated expansion, origin and stable-identity tests have 329, 266 and 53 lines.
The migration has 80 production lines, 14 test-helper lines and 70 dedicated test lines. Portable
origin restoration has 40 production lines; classification staging has 241, with 542 lines in the
shared archive-inspection test owner. Schema registration and archive framing keep their existing
owners. Unrelated desktop/batch edits remain outside this delivery.

This supplies provenance and manual authority for subsequent maintenance. Cumulative category
budgets, overlap/retirement decisions, automatic scheduling/first use and integrated human-work
verification remain in the canonical queue.

### Whole-scheme candidate comparison

The native assessment now compares additions jointly against the unchanged current scheme. It
recomputes independent support after each overlapping, conflicting or budget-constrained proposal
is deferred. Existing definitions remain intact, including categories with no winners in the sampled
window. Cohort naming and purpose admission retain their earlier authority; this pass assesses the
combined scheme's competition and does not publish it or enable background maintenance.

The diagnostic reuses the retained six-category comparison and four selected managed proposals,
with three independent supports required and strictly positive improvement/separation. It reads
236 cached calibration vectors after the earlier known visual-duplicate exclusions, plus existing
text features. The feature-cache lookup streams 512 rows at a time across 29,597 retained entries.
No original images, model workers or sealed holdouts are opened; no new inference is performed.
Opaque sample IDs supply diagnostic identities, not current source-publication proof. Earlier
visual labels remain provisional and are not treated as user-confirmed truth.

| Addition ceiling | Selected proposals and final independent support | Changed leaders | Two ordered/reversed assessments |
| --- | --- | --- | --- |
| 1 | Character reactions: 43 | 43 | 0.258 s |
| 2 | Character reactions: 43; code screenshots: 17 | 60 | 0.242 s |
| 3 | Character artwork: 16; character reactions: 37; code screenshots: 17 | 70 | 0.242 s |
| 8 | Character artwork: 16; character reactions: 34; cat reactions: 5; code screenshots: 17 | 72 | 0.233 s |

These are Debug measurements of the in-memory comparison and its reordered repetition, excluding
fixture parsing and matcher construction. Every repetition returns identical evidence and decisions.
The full four-proposal result agrees with the independent retained-cache calculation for all 236
sample assignments. Ceiling 8 retains the earlier per-round profile; 1, 2 and 3 are counterfactual
limits, not selected product settings. The six existing definitions are retained in every case.

Joint matching changes the evidence available to an individual proposal. The cat-reaction candidate
retains only two of its original three cohort members, while finding five independent winners in
the complete diagnostic window. The character-reaction candidate also attracts twelve prior artwork,
two prior comic and one prior screenshot assignments alongside nineteen prior reaction assignments.
This records category-set effects rather than an accuracy result: some images have overlapping uses,
and the earlier machine categories do not establish the intended human category. A smaller ceiling
can remove a useful distinction; this comparison does not select a cumulative category-count budget.

The retained fixture is 4,299,568 bytes, with SHA-256
`2acb51ba7e4002fdac0ca52a572259de422d72a902062c186303f31a7ca0edd9`.
The final diagnostic binary SHA-256 is
`9aafe3091fe1c7d6367305b79a966c7a88b3010aa66b95f970086837674e95f7`,
and the JSONL result SHA-256 is
`d0ab5565287002d34a99a0c99bfba9329c9b90919c58a8deca49a76ee7bf223e`.
The ignored `.build/r5-category-discovery/native-scheme-fixture.json` and `native-scheme.jsonl`
retain the vectors and outputs locally. Build the existing `classification_worker_probe` example
and pass `<fixture> --scheme`; the bounded fixture supplies categories with description vectors,
baseline IDs, sample IDs/vectors, the recurrence requirement and counterfactual addition ceilings.

Eight focused native regressions pass: distinct versus redundant broad proposals, sparse support,
existing-description reuse, equivalent proposals, budget selection, input-order invariance,
duplicate-content rejection, name conflicts, final assignment ties, incompatible input, cancellation
and evidence identity. The evidence identity includes the baseline winning category as well as its
score, so a changed old assignment cannot retain the same assessment revision merely because the
winning score is unchanged. Final native compilation has no warnings. The non-mutating formatting
gate passes across 685 Dart files with zero changes and the Rust sources. Hosted CI retains
full-suite and lint ownership.

The scheme owner has 237 production lines, selection 157 and the diagnostic 190, each with zero
inline test cases; the dedicated boundary tests have 332 lines. The existing discovery facade and
public native facade expose the typed assessment, while the existing diagnostic entrypoint selects
it. There is no schema, bridge, model, dependency or desktop change in this delivery.

Persistent cumulative budgets, guarded merge/retirement publication, runtime scheduling, first-use
integration and measured reduction of human work remain in the canonical queue. This bounded window
does not authorize deleting an existing category or replacing a human decision.

### Equivalent-definition retirement and recovery

Existing generated categories with the same complete descriptions and naming purpose now have a
native preparation/publication path. It retains any category with an unknown origin, explicit
protection, a changed definition, a human classification reference or an active purpose dependency.
Protected equivalents remain available together. Other equivalents use one retained canonical
identity, with at most 64 retirements per transaction. Distinct descriptions and purposes are kept
separate. No image count, embedding similarity or low occupancy is used as retirement proof.

Publication rereads the complete bounded configuration and protection snapshot and repeats the
decision within the shared configuration transaction. Retirement removes only active rule membership;
historical definitions, runs, origins, human choices and independent Favorites remain. The replacement
run has no inherited calibration profile. A committed retry returns the current configuration and
preserves a later stop. The shared active-run authority check is extracted without changing the
existing addition boundary's catalog/dispatch/model checks.

Schema 72 extends the bounded evidence table with version-2 retirement records while preserving
version-1 payload bytes and foreign-key identities. The migration rebuilds only that derived-evidence
table transactionally; it neither rescans media nor removes category history or manual protection.
Retirement evidence records both identities, names and semantic revisions, the generated origin,
human-category protection, the previous rule set and the new run. Portable format 6 remains unchanged.

Focused diagnostics cover durable retirement/restart, idempotent retry after stopping, retained human
classification and Favorites, manual/legacy/edited definitions, pause/protection/choice races,
mid-transaction cancellation, failed evidence insertion, bounded repeated rounds, purpose parents,
different purposes, cancelled preparation and obsolete run authority. Forward-migration cases check
old payload/configuration preservation, rejection of unsupported or empty retirement records, failed
upgrade rollback and successful retry. Existing configuration/addition/protection/recovery regressions
exercise the shared active-run owner and earlier migration path.

The first transaction diagnostic passed 27 cases and exposed four successful-publication failures at
the version-1 evidence constraint. Schema 72 addresses that owning constraint; failure-injection cases
also check the specific injected/cancellation error, so an unrelated constraint failure cannot stand
in for a passing rollback case. All diagnostics use isolated temporary catalogs, with no original
media, runtime/model downloads or desktop interaction.

After the migration repair, all 33 settings cases pass, including eight retirement cases and the
existing configuration/addition/protection checks. Both new retirement-migration cases and both
earlier maintenance-migration cases pass: 37 focused cases, zero failures. The repository format
check passes with 685 Dart files unchanged; `git diff --check` passes. Hosted CI retains full-suite
and lint ownership for the delivered commit.

The retirement application owner has 133 production lines, its domain types 37, the shared active-run
guard 21, SQLite publication 125 and evidence encoding 50, with no inline tests. Dedicated retirement
tests and fixtures have 483 lines. The new migration has 41 production lines and 27 test-only helper,
import and module-registration lines, with 125 dedicated test lines. The existing migration registry
has 4,173 lines before its 2,290-line inline test module; the catalog facade has 3,333 total lines with
its tests in a dedicated module. Changes there only register the migration and current schema version;
retirement policy and transaction ownership remain in the dedicated owners above.

This establishes exact-description retirement. Persistent cumulative usefulness budgets,
non-equivalent overlap decisions, automatic maintenance scheduling and first-use integration retain
their canonical scope.

### Joint publication and cumulative category capacity

Category expansion now uses the same joint selection owner as the retained scheme comparison before
constructing its replacement configuration. The original name/purpose/context admission remains a
prerequisite: selection cannot turn a previously rejected member into support when it removes another
candidate. Existing descriptions are not duplicated under new names, and the final supporting-content
list is derived from the selected joint result.

The native request requires an explicit cumulative generated-category budget. Active automatic origins
count across cohorts and restarts, including protected definitions. Manual categories retain only the
overall taxonomy constraint; retired origins do not consume an active slot and their identities cannot
be recreated. The remaining generated slots, 256-category bound and existing per-pass ceiling jointly
limit additions. Existing categories are preserved when a lower budget has no remaining capacity.

The publication transaction repeats the active-origin membership and count check alongside the
existing catalog/dispatch/model/source guards. Its receipt and bounded immutable evidence bind the
budget, prior generated identities, algorithm and final support. The evidence payload retains its
version-1 envelope with additive selection fields; schema 72 and portable archive version 6 remain
unchanged. All verification fixtures use isolated temporary catalogs without original image reads,
model inference, downloads or desktop operations.

The preparation owner has 181 production lines and a two-line dedicated-test registration; provenance
construction has 102 and final selection 137. The shared scheme owner has 237 production lines and a
two-line dedicated-test registration, with selection at 168 and dedicated scheme tests at 365. Domain
maintenance types have 143 production lines, SQLite publication 190 and evidence encoding 69, with
no inline tests. Dedicated capacity tests have 393 lines and the existing shared fixture has 171.
The preparation split isolates final selection and immutable evidence construction without changing
the shared configuration transaction or moving policy into presentation.

The nine scheme regressions pass, including preservation of member ineligibility and the earlier
joint competition/order/cancellation cases. The settings run passes 39 of 40 cases; the remaining
case stops in fixture setup because its two extra locations were not cataloged before fingerprint
publication. After adding those isolated location and source-authority records, the unchanged
selection/publication code passes that exact case. The 49 relevant cases therefore pass across these
runs. They cover cumulative limits after restart and protection, retirement capacity, origin changes
without revision changes, budget-bound receipts, equivalent descriptions, full taxonomy capacity,
one-slot competition and deterministic publication, alongside the existing source/recovery checks.
An earlier compile attempt identified a test comparison of a command type without equality support;
the assertion now compares its selected identities, taxonomy and run configuration directly.
The repository format check passes with 685 Dart files unchanged, and `git diff --check` passes.
The delivered commit retains hosted CI ownership for the complete suite and lint checks.

Useful default-budget admission, non-equivalent existing-category retirement, automatic scheduling
and first-use integration remain in the canonical queue. The explicit capacity parameter is an
enforced policy input, not a newly selected shipping category limit or a measured accuracy result.

### Native cohort formation from cached features

The discovery application now forms its own complete-link cohorts from compatible cached image
features. It sorts distinct content identities canonically, selects the closest eligible clusters,
and requires every pair within a resulting cluster to meet the supplied cosine. The bounded link
matrix uses at most one MiB for 512 contents. Current-category matching supplies the comparison set:
other contents with a leading category represented among the members. It never substitutes an
unrelated comparison set. Sparse groups, missing comparisons and groups beyond the supplied ceiling
retain explicit deferrals; cancellation returns no partial batch.

The six focused cohort tests pass, covering deterministic membership/comparison/revision under
reordered input, a similarity chain with dissimilar endpoints, current-purpose comparison, missing
comparison and capacity deferrals, empty/duplicate/incompatible/oversized input, and cancellation.
The diagnostic's initial build rejected its nested module path; an explicit relative path fixes
that registration and the rebuilt example succeeds. No production policy changed for that repair.

The existing cached-feature diagnostic now accepts `--cohorts` using the same bounded eight-MiB
fixture reader as `--scheme`. The retained 236-content calibration fixture, pair cosine 0.75,
minimum three members and maximum 64 cohorts produce:

| Check | Result |
| --- | --- |
| Native cohorts | 28 |
| Admitted cohort contents | 139 |
| Deferred contents | 97 |
| Every member and comparison set against the four retained broad-cohort fixtures | Identical |
| Reversed input, including cohort revisions and deferrals | Identical |
| All 236 input identities accounted for once | Pass |
| Debug formation / reversed repetition | 0.3262 / 0.3265 seconds |
| Original image reads / new inference / held-out use | 0 / 0 / none |

The four retained fixture digests are checked before comparing complete sets. This is a cache-only
algorithm equivalence check using opaque calibration identities, not source-publication authority
or an accuracy score. The four existing whole-scheme diagnostic cases remain identical after the
shared fixture-reader extraction, excluding elapsed time. No image, model or runtime is downloaded.

Reproduce by building the existing `classification_worker_probe` example with
`--features classification-worker`, then passing the ignored
`.build/r5-category-discovery/native-scheme-fixture.json --cohorts`. The ignored
`native-cohorts.json` retains output, and `check_native_cohorts.py` compares the retained inputs.
The repository format check passes with 685 Dart files unchanged; `git diff --check` passes.
Complete automated coverage and lint remain with the pushed commit's hosted gate.

The cohort owner has 135 production lines plus two dedicated-test registration lines, its complete-link
helper has 59 production lines, and the dedicated tests have 315 lines; there are no inline tests.
The discovery facade has 245 production lines plus two test-registration lines and only registers
the new owner. The domain discovery owner has 317 production lines plus two test-registration lines;
its additions expose read-only member/comparison identities. The shared diagnostic has 222 lines,
its cohort mode 61, and the existing probe entrypoint 167, without inline tests. No persistence,
schema, public desktop bridge or source-file behavior changes.

Runtime sample scheduling and automatic category maintenance remain the next integration boundary.
This native grouping preserves the measured discovery input without selecting the cumulative
category budget or claiming useful names from similarity alone.

### Native single-category withdrawal effects

The next comparison measures each current category's withdrawal independently against the complete
unchanged scheme. For every affected content, the native owner retains its former similarity, the
best remaining category and the score difference. The alternative follows the existing canonical
tie-break. Categories absent from this bounded window remain explicit with no affected members;
neither sample absence nor a score gap grants retirement authority.

Four focused regressions pass. The main case independently rebuilds and rematches the taxonomy after
each single removal and checks every changed and unchanged sample. Other cases cover zero-loss ties,
stable alternative identities, input-order invariance, display/score/content evidence,
duplicate and oversized windows, incompatible features, missing alternatives and pre-cancellation.
The assessment retains at most 512 affected contents across all categories and has no source,
model-execution, persistence or desktop dependency.

The existing probe's `--withdrawal` mode uses the retained 236-content, ten-category fixture
`native-scheme-fixture.json` with SHA-256
`2acb51ba7e4002fdac0ca52a572259de422d72a902062c186303f31a7ca0edd9`.
All 236 initial assignments match the earlier whole-scheme assessment. Reversing the input preserves
the complete assessment and revision. One Debug comparison takes 0.0628 seconds; original-image
reads, new inference and held-out use remain zero. The ignored `native-withdrawal.json` retains the
complete category destinations and per-content scores.

| Generated definition | Affected contents | Mean similarity loss | Contents above 0.01 loss | Contents above 0.02 loss |
| --- | --- | --- | --- | --- |
| 兽耳角色插画 | 16 | 0.01135 | 7 | 3 |
| 兽耳角色表情 | 34 | 0.01437 | 20 | 9 |
| 猫咪表情 | 5 | 0.02553 | 4 | 2 |
| 代码截图 | 17 | 0.01891 | 8 | 7 |

All 17 code images would return to the broad screenshot category. A single shared alternative
therefore cannot by itself establish redundant meaning: this case loses a retained useful distinction.
Combining a 0.02 minimum loss with the earlier three-content recurrence rule would also discard the
cat-reaction distinction retained by the preceding calibration comparison. These are counterfactual
threshold checks, not admitted confidence bands or measured classification-error rates. The code
does not merge or retire any definition and does not alter human decisions or R4 state.

This establishes measurable removal effects for the remaining non-equivalent overlap policy.
Naming relationships, useful independent coverage, protection and publication must still determine
whether a replacement is admissible. Several single-category results cannot be combined into a
joint retirement without reassessing the complete replacement scheme. The cumulative default budget,
automatic maintenance and first-use integration retain their canonical requirements.

The withdrawal owner has 139 production lines plus two dedicated-test registration lines and 218
dedicated test lines. The discovery facade has 250 production lines plus two test-registration lines;
its change only exposes the new owner. The diagnostic has 49 lines, its shared fixture owner 224 and
the existing entrypoint 170, without inline tests. No schema, desktop bridge or dependency changes.
The repository format check passes with 685 Dart files unchanged; `git diff --check` passes.
Complete automated and lint checks remain assigned to the pushed commit's hosted gate.

### Reactivation of automatically retired definitions

Fresh discovery admission can now restore an inactive generated category with its original identity,
label, semantic revision and creation origin. The application distinguishes this from new creation;
the catalog transaction repeats the retained-label, complete-origin, protection, active-membership,
capacity and current-source checks. New configuration, uncalibrated run and immutable admission
evidence commit together, while the original origin and historical category versions remain intact.
Manual retirement, edited definitions and legacy definitions without origins remain protected.
The new identity owner keeps these bounded reads and checks out of the publication coordinator.

Five dedicated regression cases cover real expansion followed by automatic equivalent retirement
and fresh restoration, retained identity/version counts, immutable origin, restart and retry,
preservation of a later explicit stop, protected/changed/missing origins, concurrent changes without
a catalog revision, source-evidence loss, cancellation and evidence-insert rollback. Restoration
consumes a cumulative generated slot and still requires fresh context admission. These use isolated
temporary catalogs; they read no original media and perform no model or desktop operations.

The settings diagnostic passed 44 of 45 cases. The remaining new case supplied inconsistent aggregate
scores: its modified contrast no longer equaled member similarity minus comparison similarity.
The unchanged evidence validator rejected that fixture. Correcting both comparison and contrast
values makes the focused case pass, including an explicit `WeakContext` assertion and successful
restoration when capacity is available. All 45 affected cases therefore pass across these runs.
An initial exact-name retry matched zero tests because it omitted the module prefix; that result
is excluded, and the full-name retry executes and passes the intended case.
The repository format check passes with 685 Dart files unchanged; `git diff --check` passes.
Complete suite and lint execution remain assigned to the delivered commit's hosted gate.

The domain maintenance types have 146 production lines and their activation rule 35. Application
preparation has 191 production lines plus two test-registration lines, provenance 126 and selection
138. SQLite publication has 149 lines, evidence encoding 81, the transaction coordinator 145 and its
identity reader/validator 106. These owners contain no inline tests. The dedicated reactivation suite
has 308 lines; its existing parent test file has 331 with only the new module registration added.
Schema 72, portable archive version 6 and the desktop bridge remain unchanged. The admission payload
keeps its version-1 envelope with additive activation provenance; its evidence identity is version 3.

Non-equivalent merge admission, useful default-budget selection and automatic first-use integration
remain in their canonical scope. Reactivation supplies their identity-preserving restoration path;
it does not enable automatic maintenance on its own.

### Bounded catalog coverage for maintenance

The native survey now traverses distinct current exact contents in keyset windows of 1–64 entries.
It counts the latest compatible model suggestion, including refinement, and retains at most 1–64
opaque content identities per category. Duplicate files share one count. Missing predictions and
sources without matching published direct fingerprints retain explicit incompleteness. Malformed
records fail the read rather than becoming missing suggestions. Configuration, dispatch and catalog
publication authority are rechecked across pages and after the final page; changed authority or
cancellation returns no partial report.

Six application regressions pass, covering page-independent results, bounded examples, missing
evidence, strict ordering, malformed windows, incompatible category membership, final validation,
pre/mid-survey cancellation and refined suggestions. Five SQLite workflow cases pass across the
diagnostic runs, covering exact copies, latest predictions, compatible stop/re-enable and restart,
unpublished or changed source evidence, missing analysis, category-scheme replacement, malformed
records and publication/dispatch changes. Two query/transaction cases pass: the page seeks the
existing content and prediction indexes, and actual SQLite `OperationInterrupted` is observed before
verifying read-transaction retirement and cancellation-handler removal. Thirteen focused cases pass.

The first compile identified a missing test import and unsigned SQLite conversions; the adapter now
uses the established checked integer helpers. The initial query plan used nullable file identity for
source membership; the query now explicitly selects the source-generation index for that lookup.
The range assertion also recognizes SQLite's composite digest/length seek. Two workflow assertions
initially tested incorrect setup: saving unchanged configuration deliberately preserves its run,
and the source edit matched no asset row. They now exercise Stop followed by Enable and assert one
actual selected-location update; both affected cases pass without weakening the production guards.

The survey performs catalog reads only. Its SQL is cancellable, each page uses a short read transaction,
and all decoded results are bounded by the page and taxonomy limits. Fixtures use isolated temporary
catalogs; no original media, model inference, downloads, desktop operations or held-out samples are
involved. There is no schema, dependency, portable-archive or desktop-bridge change. Counts alone do
not grant retirement authority: semantic merge admission, protection and atomic publication remain
in the canonical queue with useful-budget selection and runtime integration.

The domain coverage records have 48 production lines. Application surveying has 155 production lines
and two dedicated-test registration lines; its tests have 344 lines. SQLite coverage has 90 production
lines, querying 126 plus two test-registration lines, and its cancellable transaction 49 plus two
test-registration lines. Dedicated query, transaction and catalog-workflow tests have 47, 39 and 218
lines respectively. There are no inline tests. Existing facades only register or export these owners;
the discovery facade has 252 production lines plus two test-registration lines, domain maintenance
151, settings 144 and the configuration ports 20 and 53.
The repository format check passes with 685 Dart files unchanged, and `git diff --check` passes.
The delivered commit's hosted gate retains complete-suite and lint ownership.

### Returning sparse refinements to their original purpose

The native merge operation now reads each generated definition's original admission evidence and
uses its recorded recurrence requirement and semantic purpose. The complete current catalog survey
retains bounded unique second choices from the final prediction, including refinement. Preparation
requires every assigned content to be represented, every alternative to be the same unchanged
purpose, and support below the original admission requirement. Ties, truncated evidence, incomplete
or empty libraries, missing original evidence, changed meanings and human protection retain the
definition. Complete zero occupancy remains distinguishable from missing analysis. One operation
removes one refinement; another operation needs fresh coverage after the scheme changes.

Twenty-seven focused cases pass across application policy (4), SQLite merge workflows (4), schema-73
migration (2), coverage (6), SQL cancellation (1), shared equivalent-retirement protection (8) and the
schema-72 migration chain (2). Workflow cases exercise retained historical predictions and definitions,
uncalibrated run replacement, restart and idempotent replay after Stop, late source/publication/pause/
human/origin/evidence changes, missing creation evidence and changed parent semantics. Injected write
failure and cancellation during run replacement preserve the prior configuration, run and receipt;
the connection returns to autocommit and its interrupt handler is removed. Migration checks retain
version-1 and version-2 payloads byte-for-byte, reject malformed version-3 records, and preserve the
old schema and evidence on failure before successful retry. Existing R4 state remains unchanged.

The first compiled diagnostic exposed an unused plan-accessor warning despite passing assertions.
The write adapter now consumes the typed configuration accessor; all four affected transaction checks
pass again with a warning-free compile. Full lint and complete-suite execution remain hosted.

The retained 236-content withdrawal report has 16 animal-ear illustration, 34 animal-ear reaction,
5 cat-reaction and 17 code-screenshot assignments. Each still meets the measured three-content
recurrence candidate and is retained by this rule. This is a compatibility comparison with retained
evidence, not a new inference or held-out accuracy measurement. Small score gaps and a common fallback
remain insufficient reasons for merging these useful distinctions. No original-media reads, model
work, downloads, desktop operations or held-out samples were used for this slice.

The new domain records have 47 production lines. Application merging has 149 production lines and
two test-registration lines, with 290 dedicated test lines. SQLite merge publication, original-basis
reading and evidence encoding have 81, 90 and 53 production lines; their workflow tests and fixture
have 305 and 142 lines. Coverage has 167 production lines plus two registration lines and 358 test
lines; its SQL transaction owner has 58 production lines plus two registration lines. The migration
has 46 production lines, 23 test-helper lines and two registration lines, with 117 dedicated test
lines. The existing migration facade adds three registration/dispatch lines and no inline tests;
the shared catalog facade changes only the schema-version constant. No unrelated extraction or
desktop change is included.
The repository formatting check passes with 685 Dart files unchanged, and `git diff --check` passes.

Schema 73 preserves catalogs and maintenance history without a rescan. Portable archive version 6,
model/runtime dependencies and the desktop bridge are unchanged. Non-parent overlap policy, useful
default-budget selection, broader context admission and first-use integration remain in the canonical
queue; this transaction does not itself enable automatic maintenance.

### Catalog-backed addition capacity

The native addition request now supports automatic capacity from the current distinct-content
survey, alongside explicit ceilings for controlled comparisons. Capacity divides verified contents
by the same recurrence requirement used for admission, bounded by the existing 256-definition
contract and never below the retained generated count. Joint competition and the per-assessment
proposal bound still select useful additions; the calculated capacity is not a target. Missing
predictions do not postpone capacity for already verified contents, while duplicate copies and
unverified sources add no slots. A reduced library cannot remove existing or protected definitions.

The retained 236-content, three-support comparison corresponds to a cumulative capacity of 78,
before overall taxonomy and per-assessment limits. Its four admitted distinctions remain within
capacity; the per-assessment limit remains eight. This is a compatibility calculation using the
retained comparison, not a new inference, held-out accuracy result or broad granularity measurement.
It avoids selecting the demonstrably restrictive one-, two- or three-category ceilings solely to
minimize the number of categories. Naming/context quality and integrated human-work measurements
remain separate acceptance obligations.

The operation records the coverage digest, publication revision, distinct-content count, recurrence
requirement and computed capacity. Publication revalidates catalog, configuration, encoder and
source-verification authority in its existing transaction. Scoped SQLite cancellation now covers
expansion validation and writes, with the handler removed before commit or rollback. Committed replay
retains later Stop and does not revalidate obsolete discovery counts. No schema, archive version,
model dependency, desktop bridge or source-media behavior changes.

Five new native workflows pass: admission before prediction completion and replay after restart;
capacity unchanged by unverified sources or duplicate copies but increased by new verified content;
rejection after publication/source evidence changes; incompatible configuration rejection; and
preservation of retained protected categories when current capacity shrinks. The related expansion
suite initially passes 35 of 37 cases. Two unchanged archive-export paths return Windows access denied
in the restricted process; both pass in the isolated unrestricted recheck with identical product
code. That initial compile also reports an incremental-cache hard-link warning. The scoped recheck
uses non-incremental compilation and completes without warnings. The combined valid results cover
all 37 cases; complete-suite execution and lint remain hosted.
The read-only repository format check passes with 685 Dart files unchanged, and `git diff --check`
passes. Its initial restricted invocation completed formatting but failed updating the SDK-owned
telemetry-session timestamp; the same command passes with the required process access.

The capacity policy has 61 production lines. Addition preparation has 185 production lines plus two
test-registration lines, provenance 135, domain maintenance 162, SQL publication 155 and evidence
encoding 92. The discovery facade has 256 production lines plus two registration lines; the public
classification facade has 95 and the maintenance port 67. New dedicated workflow tests have 255 lines;
the existing expansion tests have 332, shared fixture 185, capacity tests 393, origin tests 266,
preservation tests 181 and reactivation tests 308. Existing test changes replace cancellation signals
with shared signals for SQLite interruption; there are no new inline test cases.

All catalogs and backup artifacts in these diagnostics are disposable fixtures. No original media,
retained user catalog, model process, download or desktop interaction is used. Automatic scheduling,
non-parent overlap admission and broader context coverage remain in the canonical R5 queue.

### Sparse refinement into an existing broader context

The merge operation now accepts a declared broader context under the same original purpose when
the original-purpose destination does not qualify. Source occupancy must fall below its original
admission threshold, every source member must be retained with the same unique alternative, and
the target must retain its own admitted recurrence. Both composed meanings and original naming
authority must still agree. The versioned context catalog supplies a strict containment path of
at most four edges; a reverse path anywhere in the bounded catalog rejects the merge. Names and
score gaps alone cannot supply this authority.

The replacement retains the target definition, original source identity, historical runs and
predictions. Its existing transaction binds current coverage, both admission records and human
protection. Version-3 evidence records the context revision/path and retained target separately
from the original purpose. Schema 73, portable archive version 6, model dependencies and the desktop
bridge remain unchanged. This is a native maintenance boundary; automatic dispatch and first-use
integration retain their remaining work in the canonical roadmap.

Application regressions cover supported containment, unrelated names, cycles longer than the
admitted forward path, missing/changed meaning, source and target recurrence, incomplete coverage,
human protection, deterministic traversal and cancellation. SQLite workflows exercise atomic
publication, unchanged target definitions and R4 state, retained history, restart and Stop replay,
changed target authority and predictions, failed writes, cancellation and successful retry.

All 15 application and SQLite merge checks pass with warning-free non-incremental compilation.
The first run passed 14 cases; the remaining fixture supplied three scores for its new four-category
taxonomy and was correctly rejected as incomplete. The fixture now ranks the whole active taxonomy,
and the repeated suite passes. The read-only repository format check passes with 685 Dart files
unchanged; `git diff --check` passes. No original media, model process or desktop window is used.

Merge eligibility, context containment and graph traversal own 63, 149 and 92 production lines.
Merge preparation has 123 production lines plus two test-registration lines; domain merge records
have 58, SQL publication 80 and evidence encoding 99. The existing context catalog has 201 production
lines plus two registration lines, domain maintenance 162, discovery facade 256 plus two registration
lines and public classification facade 95. Dedicated application merge tests have 292 and 325 lines;
SQLite merge tests have 306 and 368, with the unchanged 142-line shared fixture. These owners have
no inline test cases. The graph owner verifies containment; the policy owner verifies admissibility;
the existing transaction continues to own authority, replacement and rollback.

### Native maintenance submission and runtime handoff

Prepared additions, equivalent retirements and sparse-category merges now use the same native
retirement gate as explicit configuration. The gate rejects a live runtime before opening the
catalog and holds ownership through the existing guarded commit. It accepts no unvalidated category
definitions or replacement settings; each plan retains its original proof, cancellation and replay
boundary. Successful submission leaves runtime reopening to the established lifecycle owner.

The focused native workflow opens a real runtime session over a disposable catalog, rejects a merge
while that session owns resources, joins its close, commits, preserves a later Pause on replay, and
opens a fresh session with the committed run. The paused replacement remains idle with no feature
cache, model or worker files. A second workflow covers cancellation before admission and rejection
after Stop, preserving the prior setup and maintenance history. This verifies the handoff boundary;
scheduled discovery, context coverage and automatic first use remain in the canonical queue.

Both native handoff cases pass with warning-free non-incremental compilation. The repository format
check passes with 685 Dart files unchanged, and `git diff --check` passes. The submission owner has
50 production lines; the runtime facade has 261 plus two test-registration lines and the public
classification facade 96. The new dedicated handoff file has 150 test lines; the existing merge-test
registration file has 308. No inline cases, schema changes, new dependencies or bridge changes are
included. All runtime paths and catalogs are disposable fixtures; no source media or desktop window
is accessed.

### Resource-backed native vocabulary

The native resource adapter now supplies the discovery port from digest-verified application data,
with opaque namespaced identities, canonical graph revisions, ordered pages and separate typed
naming contexts. Raw lexical concepts remain traversal-only. Loading rejects malformed or
unsupported resources, repeated/unknown references and exceeded input, entry or graph bounds.
An admitted snapshot is independent of later changes to its file. No catalog schema, model,
desktop bridge or notification behavior changes.

Five focused adapter cases pass, covering paging and graph composition, input-order-independent
revisions, display-only description reuse, malformed/duplicate authority, digest mismatch,
cancellation, replacement of an opened resource, input-size rejection and empty/page-boundary
behavior. The first normal library/example build exposes seven existing unused warnings because
coverage survey and original-purpose merge preparation were missing from the native public facade.
Both operations are now exported through that existing facade, and the repeated build passes without
warnings. No lint exclusion or weaker check is introduced.

The retained lexical data is converted without model or media access into a 23,249,187-byte resource,
SHA-256 `2cea3ceca629faac3d51271c2928871548ddc709cb769cc50f45a771ae68835c`.
The reader validates all 82,115 concepts, 248,493 source relations, 3,921 entry points, 42 contexts
and 714 cue links. This is the same calibration vocabulary; its upstream notices and derived files
remain outside Git. This check admits the resource reader, not a distributed vocabulary release or
additional category-quality claim.

The resource-backed probe repeats the retained eight screenshot cohorts twice. All 16 naming and
per-member admission results equal the prior path exactly. Cache entries remain unchanged; missing
worker/model/runtime paths ensure a cache miss cannot silently fall back to inference. The completed
Debug comparison takes 76.406 seconds, peaks at 89,559,040 bytes of sampled process-tree working set
and starts zero model workers. Resource loading takes 3.258 seconds in that build. Original media
and the held-out partition are not read. Ignored `native-resource-*` artifacts retain the converted
resource, input identity, inspection, comparison and aggregate evidence.

The initial restricted comparison is rejected by the existing native cache namespace ACL guard.
The unchanged guard passes under the normal Windows token. That comparison then passes all 16
semantic checks but its diagnostic incorrectly counts the Windows console host as a model child.
A scoped observation identifies `System32/conhost.exe`; the corrected monitor retains that exact
system-host distinction and rejects other children. The full repeated comparison passes, including
the zero-worker check. No product safeguard changes. The earlier head's
[Static job](https://github.com/Cedarflake/Cedarflake-Ame/actions/runs/41263540047/job/124721543478)
also reports the same seven missing-export errors; the normal warning-free build covers their
native cause, while complete verification belongs to the replacement commit's hosted gate.
The read-only repository format check passes with 685 Dart files unchanged; `git diff --check`
passes. Resource distribution, automatic maintenance scheduling and first-use enablement remain
the next integration boundaries in the canonical roadmap.

The adapter has 180 production lines plus two test-registration lines; resource decoding has 171.
The shared discovery domain has 323 production lines plus two registration lines, context composition
197 plus two, and the public classification facade 96. Dedicated adapter tests have 268 lines, with
no new inline cases. The native probe entrypoint, discovery workflow, vocabulary composition and
resource inspection have 177, 372, 115 and 51 diagnostic lines respectively.

The [subsequent native CI partition](https://github.com/Cedarflake/Cedarflake-Ame/actions/runs/41267206754/job/124732442286)
finds the vocabulary module missing from the exact source-loading roster. Registering its private,
unconditional file module and extending the existing mutation cases restores all 13 focused
root-availability checks, including rejection of alternate paths, inline bodies and broader visibility.
The repair changes seven test lines and no production logic. The enclosing local-files owner has
2,630 lines before its 3,469-line test module; the dedicated topology suite has 533 lines.

The same head's [Schema and Queue partition](https://github.com/Cedarflake/Cedarflake-Ame/actions/runs/41267206754/job/124732442106)
finds two maintenance tables absent from the v33 upgrade fixture's expected additions. Both tables
now require explicit empty-row assertions before comparing every retained table with its original
contents. All four foreground-index migration cases pass, covering repeated reopen, rollback and
invalid authority. Only two assertion-roster lines change in the 359-line dedicated test file;
production migrations and retained-data assertions remain unchanged. The repository format check
passes with 690 Dart files unchanged, and `git diff --check` passes.

### Bundled vocabulary distribution

The Windows build now produces the retained vocabulary from pinned OMW English WordNet 2.0 and
Chinese Open WordNet 1.1 archives. The tracked manifest binds archive, member, naming-context and
output identities. The six-file bundle contains the resource, both upstream licenses, both citations
and Ame's modification notice. The archives and generated 23,249,187-byte resource remain outside Git.
The production naming-context input retains the frozen calibration fixture's exact bytes; packaging
does not enlarge its coverage or establish automatic category maintenance.

Offline preparation from the retained archives reproduces the admitted resource's SHA-256 exactly.
The production CMake target prepares and verifies the resource in the normal Windows build tree.
No model inference, new source download, catalog or original-media access occurs. Normal hosted
Release retains ownership of the complete application build and installed payload.

Five focused fixtures pass for noun relations and Chinese alias preference, regular and unique
archive members, corrupt/extra output, atomic publication failure, bounded and reusable downloads,
and retained manifest identity. Unsigned payload, portable archive and installer component checks
pass, including omission of each of the six new files and the added directory's ACL ownership.
The first packaging checks exposed old fixed file/component counts; their exact expectations now
include the new files and directory. No required payload or rejection case is removed.

The restricted test run could not use the system temporary directory or publish hard links. Fixture
storage now stays under repository build storage; the unchanged hard-link and junction checks pass
under the normal Windows token. Source/output hashes also match with LF repository normalization.
PowerShell syntax and the read-only repository format check pass with 690 Dart files unchanged.

| Owner | Production lines | Inline test lines | Dedicated test lines |
| --- | ---: | ---: | ---: |
| Vocabulary preparation / lexical transformation | 93 / 148 | 0 | 188 shared fixture lines |
| Artifact admission and atomic publication | 168 | 0 | Included above |
| PowerShell artifact contract / preparation entrypoint | 40 / 28 | 0 | Shared packaging checks |
| CMake dependency/install module | 12 | 0 | Production target and hosted Release |

### Current-content sampling for maintenance

The discovery sampler connects the existing distinct-content coverage query to compatible cached
image features. It examines at most 512 contents in pages of at most 64; missing features consume
that bound and do not trigger image reads or inference. Exact continuation carries the original
catalog/configuration/model/dispatch authority. Current content can be sampled while other sources
still lack exact verification; incomplete global coverage remains explicit in the snapshot.

Six focused cases pass for bounded paging and exact continuation, an all-missing 512-content cache
window, changed publication and manual Pause, cancellation and final revalidation, invalid pages and
incompatible features, and empty/invalid-limit input. The cache fixture rejects any feature write,
text analysis, enumeration or cleanup. Native non-incremental compilation passes without warnings.
This supplies the sampler for maintenance preparation; automatic scheduling/publication and the
integrated category-quality result remain under the roadmap's existing acceptance boundaries.

The preparation owner connects sampled features to the current matcher, cohort formation, bounded
retrieval, nomination and joint addition assessment. It returns at most one immutable expansion plan
using verified-content coverage for cumulative capacity; catalog publication remains behind the
existing runtime-retirement gate. Empty or insufficient cached samples start no encoder.

Five focused preparation cases pass for guarded additions and text-cache reuse, absent features,
cancellation, changed publication after assessment and invalid policy before I/O. The initial run
finds an incorrect test accessor; the assertion now uses the setup's primary taxonomy. The restricted
Windows token then rejects temporary-cache namespace admission; all five cases pass with the normal
token and the existing ACL guard unchanged. No image inference or catalog write is permitted by the
fixtures. These checks establish native preparation, with scheduling and first-use acceptance still
pending.

The sampler has 135 production lines plus two test-registration lines and 336 dedicated test lines.
Preparation has 182 production and 333 dedicated test lines. The discovery facade has 263 production
lines plus two registration lines; its shared test module has 420 lines, and the public native facade
has 106. The read-only format check passes with 690 Dart files unchanged; `git diff --check` passes.
No schema, bridge, user-interface or original-media behavior changes.

### Native idle discovery handoff

The managed native entrypoint now composes ready queue execution with bounded discovery windows.
It uses the bundled vocabulary's manifest/digest and the retained naming-policy inputs, shares the
executor's feature cache and encoder residency, and returns a maintenance plan after worker retirement.
The existing submission gate still requires the enclosing runtime to be dropped. The joined desktop
handoff below connects that entrypoint to publication and reopening.

All 13 queue/maintenance cases pass, including the seven retained dispatch regressions. The six
maintenance cases cover work priority, quiet/foreground/pause/close admission, cancellation, a pause
committed without a signal, elapsed retry deadlines and preparation/retirement failure. The final
resource-release change passes those six cases again. Six preparation cases pass, including search
and context-budget exhaustion deferring the cohort without publishing partial discovery.

The runtime-filtered test invocation passes 30 cases, including the extracted executor's unchanged
source-error classification, ownership gates and three new policy checks. The policy preserves the
retained calibration inputs and distinguishes display renaming from semantic edits or unrelated
custom categories. An initial policy assertion exposes JSON integer-versus-float normalization;
the production input now preserves the frozen numeric representation. No threshold changes.

| Owner | Production lines | Inline test lines | Dedicated test lines |
| --- | ---: | ---: | ---: |
| Runtime composition / native executor | 105 / 207 | 0 | 61 retained executor lines |
| Native discovery pass / policy | 165 / 139 | 0 | 100 policy lines |
| Idle dispatch composition | 147 | 0 | 244 maintenance lines |
| Discovery preparation | 198 | 0 | 355 preparation lines |

The vocabulary adapter has 181 production lines plus two registration lines, its bundle resolver
42, and the contextual wrapper 201 plus two. The shared queue test module has 387 lines. No new
inline tests are added. The read-only format check passes with 690 Dart files unchanged, and
`git diff --check` passes. Current-commit hosted execution and connected client verification remain
distinct acceptance steps.

### Idle category reduction

An addition-free discovery pass now advances to a separately admitted reduction turn. Equivalent
retirement precedes a complete-coverage survey and the existing sparse purpose/context merge policy.
The owned plan enum carries each original immutable plan to the same guarded publication boundary.
This adds scheduling without changing recurrence, protection, alternatives or containment thresholds.

Four focused reduction cases pass: equivalence without content coverage, two-member sparse support,
recurrent/incomplete/protected retention, and cancellation/manual pause. Two native publication cases
pass with the owned plan, covering retirement before commit, replacement setup, stopped replay and
stale/cancelled rejection. Initial incomplete-coverage fixture attempts referenced an absent location
and then supplied conflicting fingerprints for an unchanged source generation; the final fixture
uses the established source-revision invalidation and explicitly verifies incomplete coverage.
No production source guard changed. These cases use isolated catalogs without image/model access.

The native discovery owner has 192 production lines, reduction preparation 48, submission 60 and
the plan domain 168. The discovery facade has 265 production lines plus two test-registration lines.
There are no inline tests; the new reduction fixture has 143 dedicated lines, the affected native
submission fixture 157, and its existing merge/retirement fixture owners 309/483. Actual first-use
acceptance remains in the current queue.

### Joined desktop maintenance publication

The desktop session now returns an opaque plan after native retirement. The client joins root recovery,
closes and disposes that session, commits the unchanged plan and reopens the saved configuration.
Explicit reconfiguration and shutdown cancel pending plans and join admitted commits. Superseded
plans are discarded; runtime-ownership and persistence failures remain errors. The replacement receives
current foreground, pressure and source-availability state. No notification layout or category UI changes.

All 21 lifecycle cases pass, including seven new maintenance races and cleanup cases. Six native
publication/desktop cases pass, preserving pause, cancellation, stale rejection, idempotent replay
and active-runtime errors. A separate real idle-dispatch case returns and commits an equivalent
retirement in 5.30 seconds using an empty isolated catalog and absent worker/model/vocabulary paths;
it creates no feature cache. Its initial test path-ownership type error was corrected before execution.
These checks do not substitute for model-backed first-use interaction.

The bridge adds an asynchronous maintenance commit and a typed session-run result. All 28 asynchronous
contracts and the rejecting method-boundary fixtures pass. The explicit contract roster is updated
from 27 to 28. Independent generation from committed source plus this handoff verifies the same 28
contracts while excluding unfinished desktop-review changes; its six changed generated files are
staged separately. One working-tree generation attempt returned Windows mapping error 1224 before
writing output; the diagnostic invocation succeeded, with the original transient cause unestablished.
Focused Dart analysis reports no issues; the final read-only format and whitespace checks pass.

The preceding `cc25a830` hosted Static job rejected the dispatch event re-export as unused in
non-test builds. That event is now re-exported only for its actual test consumers; dispatch behavior
and warning policy are unchanged. Focused library Clippy with all features and warnings denied passes.
Generated wire entrypoints retain the existing set and add only maintenance commit/cancel; the pinned
formatter preserves the generated dispatcher layout.

| Owner | Production lines | Inline test lines | Dedicated test lines |
| --- | ---: | ---: | ---: |
| Desktop lifecycle facade / session lifetime / environment | 62 / 145 / 149 | 0 | 677 shared lifecycle lines |
| Native desktop session / maintenance handle | 204 / 53 | 0 | 297 maintenance lines plus retained desktop fixtures |

The real-client first-use check and integrated R5 quality/resource acceptance remain open.
