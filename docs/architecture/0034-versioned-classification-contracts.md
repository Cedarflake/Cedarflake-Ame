# ADR 0034: Separate category rules from encoder compatibility

- Status: Accepted for implementation
- Date: 2026-10-30
- Last amended: 2026-11-03

## Context

Classification compares images with configurable descriptions. Display names, comparison rules,
image features, text features and human choices have different lifetimes. Invalidating all of them
when a label changes wastes image reads and can erase intent. Reusing them after an encoder change
can silently mix incompatible results. The retained runtime probe also found that INT8 text batching
changes some predictions even with identical tokens.

The [product contract](../product/r5-classification.md) owns whole-group authority, automatic results,
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

Current definitions are added or retired by admitted managed-maintenance plans. The desktop exposes
no definition creation, editing, or removal; existing user-authored definitions remain compatible.
Retirement removes a definition from future matching; persistence retains its identity and description
history for saved human choices. Refreshing the active taxonomy never remaps a retired human choice.
The current desktop has no manual category-decision commands, under the
[scope amendment](0035-classification-decision-authority.md#desktop-scope-amendment--2026-11-01).
An empty active scheme has no classifier and produces no invented `其他` result.

The application owns a finite curated topic catalog and category maintenance. Each topic declares
its permitted broad purposes and complete visible names. SigLIP supplies image/text compatibility;
its matching adapter does not generate category names or expand that catalog. The scoped
[discovery evidence](../acceptance/r5-category-discovery.md) records the naming probes separately from
the admitted screening encoder.

Do not generate categories from isolated low scores. A useful addition or split needs a repeatable,
meaningful distinction in the collection; small occupancy alone neither deletes a category nor
establishes an error. Maintenance must retain stable identities and saved human intent. Labeled
examples can supply evaluation evidence without silently training a model or researching
new cultural context. The curated-catalog decision below replaces the former requirement for
open-ended naming; it does not establish quality for every catalog entry.

### Curated topic catalog — 2026-11-03

Production nomination reads a small application-owned catalog directly. It performs no WordNet
traversal or generative naming. Consolidated topics replace overlapping definitions with new
identities where their meaning changes; unchanged topics retain their existing concept identities.
Each topic declares a nonempty, bounded mapping from supported purpose IDs to visible category
names. Absent combinations have no naming authority, including an unconditioned request. For
example, programming can refine screenshots without becoming a code illustration.

Purpose restrictions and names bind concept and catalog revisions. Complete composed comparison
descriptions retain their existing independent feature keys. Naming eligibility is separate from
comparison evidence: unsupported topic/purpose combinations can reject a competing candidate but
cannot be nominated or published. No composed descriptions are encoded when the purpose has no
allowed names. Final admission checks the same restriction. The ordinary six-purpose baseline,
compatible cached image features, distinct-content recurrence and guarded expansion/retirement
remain reusable. Unsupported refinements keep the broad category rather than inventing another name.

The catalog supplies a finite candidate set, not mandatory sidebar membership. Presentation consumes
one bounded category-presence snapshot and omits empty rows without retiring definitions or saved
choices. The adapter reads active definitions and populated identities in one database snapshot,
reusing gallery membership, latest compatible predictions, legacy human precedence and retained
display evidence. No per-category gallery reads or media access are required. The ordinary settings
query retains all definitions so a category becoming empty does not replace its open page or title.
Category membership remains single-primary and independent of R4 manual albums and Favorites.

The lexical retrieval, consolidation and source-resource sections below retain the contracts of
existing diagnostic and compatibility owners. They do not require those engines or lexical resources
in the production curated path. Their historical evidence remains valid only for its original inputs;
curated topic/purpose quality and the changed production path require their own affected verification.

Catalog upgrades reconcile generated identities before sampling in the existing idle maintenance
pass. A bounded immutable catalog identifies supported concept/purpose pairs. Retire an active
generated definition outside that set only when its origin, admitted definition and absence of
human choices still grant automatic authority. Preserve manual definitions, edited/protected origins,
purpose parents, existing allowed definitions and historical records. A reconciliation plan binds
the full retirement snapshot and catalog revision; publication revalidates configuration/run
authority and the protected set inside the existing atomic setup transaction. It records the catalog
and retired definitions as maintenance evidence, replaces the analysis run, and reuses compatible
image features. Cancellation or persistence failure rolls back the whole update. An identical replay
returns current state without undoing later control or user changes. No schema migration is needed.

### Automatic cohort formation

The sampling owner reads the existing distinct-current-content coverage projection in windows of
at most 64 records, examining at most 512 contents per call. It loads only compatible cached image
features; a missing feature is counted and skipped without media access or inference. The bound
applies to examined contents, so cache misses cannot turn a window into an unbounded catalog scan.
An opaque continuation retains the exact catalog, configuration, model and dispatch snapshot.
Cancellation, changed authority, unordered pages or incompatible cache data reject the whole window;
publication is revalidated after the last read. Automatic dispatch mode is required. These reads
neither update category decisions nor authorize a maintenance commit.

The preparation owner composes that window with the existing matcher, cohort builder, retrieval,
purpose-aware nomination and joint addition assessment. The caller supplies the admitted policies,
naming purposes, residency and cancellation. It uses current verified-content coverage for cumulative
capacity, returns at most one guarded expansion plan and revalidates the sampled publication after
assessment. Empty or insufficient cached samples start no encoder. Text features remain reusable
after cancellation; the operation neither reads images nor changes catalog state. The runtime
handoff and transactional maintenance submission remain the sole publication path.
Exhausted retrieval or context-expansion budgets defer that cohort and retain an explicit bounded-
cohort count; incomplete exploration cannot authorize an addition. Other invalid evidence and
resource failures retain their errors. The remaining cohorts and content windows can still proceed.

The application forms discovery cohorts from a bounded window of at most 512 distinct compatible
cached image features. It uses complete-link similarity: every pair in an admitted cluster must
meet the caller's minimum cosine. The dense link matrix is bounded to one MiB, and cancellation is
checked during feature validation, pair scoring, merge selection and cohort construction. No model,
source reader, persistence adapter or desktop state is involved in this operation.

Input content identities are sorted canonically before grouping. Closest-pair ties use that order;
completed clusters are considered by descending membership and then canonical identity. Reversing
the input preserves cohort membership, comparison membership and revision. Duplicated content or
incompatible feature/model authority rejects the entire window.

Each cluster is compared with the other contents whose current leading category also occurs within
the cluster. This preserves purpose-aware comparison without silently substituting unrelated images.
Insufficient membership, absent comparison and the caller's cohort ceiling produce separate deferred
groups with all affected identities retained. The cohort ceiling is at most 64, independently of
the category-publication budget. Empty windows return no work; cancellation returns no partial batch.

The retained calibration comparison uses minimum pair cosine 0.75 and at least three members.
These grouping inputs recover the measured discovery samples; they do not establish confidence,
name usefulness or category-publication authority. Admission, current-source revalidation, category
maintenance and background scheduling retain their separate owners. A different grouping policy
requires a compatible comparison before its discovery evidence is reused.

### Bounded candidate retrieval

The native discovery owner retrieves evidence from a versioned vocabulary without changing the
active taxonomy. Its port returns bounded, ID-ordered entry and related-concept windows. The adapter
owns lexical identifiers, graph interpretation and language resources; Ame receives opaque concept
identities, labels and descriptions. Vocabulary changes, changed definitions under the same identity,
invalid windows and incompatible features reject the result rather than publishing a mixed search.

A discovery cohort contains retained compatible image features for distinct content versions and
a separate comparison set. Their combined limit is 512 samples. The scorer retains two arithmetic
mean vectors and computes mean cosine and within-context contrast; it does not renormalize a diffuse
cohort into a unit vector or treat contrast as a probability. Repeated content within or between
the sets is rejected. Model, cohort, vocabulary, retrieval revision and explicit search limits
accompany the result. Cohort selection and semantic admission retain separate policy ownership.

Search streams up to 4,096 entry concepts, then expands selected neighbors within caller-supplied
limits: at most 4,096 additional concepts, 65,536 edges and four hops. A window contains at most
64 concepts, with at most 64 retained results and expansion seeds. These are resource ceilings,
not category-count targets, confidence thresholds or default maintenance settings. Traversal tracks
already scored and already scheduled identities separately, so a known intermediate concept can
still lead to a previously missing leaf. Budget exhaustion is explicit in the result.

Description preparation uses the same independently keyed feature cache and supervised text encoder
as ordinary matching. Compatible descriptions are reused; missing descriptions are encoded one at
a time. Completed cache entries survive cancellation or failure, and the text worker is retired on
every terminal path. Retrieval has no image-read, catalog-write or category-publication capability.
It returns bounded nearest and distinctive candidate lists for naming/consolidation evaluation.
The lexical probes do not admit raw dictionary nouns as smart albums or close the remaining
maintenance-policy and first-use integration requirements.

### Semantic consolidation

Retrieved fine-grained concepts can be consolidated through the vocabulary's distinct `Broader`
relationship. The adapter owns whether that edge means a generalization or a whole-image context;
ordinary related concepts do not establish either relation. A bounded naming frontier supplies the
candidate contexts for one evaluation. Its identities and the vocabulary revision bind that choice,
without imposing a permanent product category list.

The consolidation owner traverses at most four hops from the retrieved nearest and distinctive
concepts, stopping each path at its first frontier member. It retains at most 4,096 concepts and
limits both vocabulary-edge reads and traversal work to 65,536 edges. Pages contain at most 64
concepts; the frontier contains at most 64 identities. Per-origin scheduling prevents cycles and
shared paths from multiplying queued work. Incomplete retrieval, exhausted work limits, changed
definitions or changed cohort/model/vocabulary authority reject the proposal.

Each reached context is scored again using its own description and the retained image cohort.
Narrower-concept scores are not inherited, and multiple related words do not count as additional
images or establish confidence. The cohort also retains its member embeddings within the existing
512-sample/4,096-dimension limits: at most 511 member vectors, below 8 MiB of vector storage, or
approximately 1.5 MiB for the selected 768-dimensional encoder. Consolidation returns each member's
own similarity, bound to its exact content identity and ordered independently of the input. This
keeps weak or conflicting members visible to admission policy rather than hiding them behind a mean;
it adds no source reads or image inference and does not establish a confidence threshold.
Unresolved concepts remain explicit. The result binds the original
retrieval revision and limits, naming frontier, consolidation revision and resource limits. Text
features use the shared cache and supervised encoder; every terminal path retires description work.
This owner returns naming evidence and has no category-write or human-decision authority. Selection
of useful frontiers and final category admission remain application policy evaluated against the
collection, with existing categories retained during evaluation.

### Automatic context nomination

Application-owned nomination can enumerate broader contexts without a supplied naming frontier.
It shares the consolidation graph's paging, immutable-definition, cancellation, cycle and work
bounds, but visits every reached context through the bounded depth. Contexts are scored with their
own descriptions in windows of at most 64 features, against the current taxonomy's matcher.
The vocabulary adapter distinguishes naming candidates from traversal-only concepts. Abstract or
otherwise inapplicable senses can retain their graph connections without being encoded or nominated
as category names. This applicability belongs to the immutable concept definition and vocabulary
revision; it does not alter the description's feature key or define a fixed product category list.

The admission profile supplies the minimum repeated-content support, coverage, improvement and
contextual contrast for nomination. Evaluation first ranks baseline-eligible contexts by their own
member coverage, mean improvement, contrast and stable identity. At most 64 retain detailed evidence;
an omitted evaluation candidate is reported through the candidate-limit flag. Every eligible context,
including those outside that evaluation window, remains an alternative in distinct-topic comparison.
The evaluation owner applies the same per-member purpose, improvement and separation policy as final
admission before allowing a context to suppress another name. A broader context that loses support
to a competing topic cannot erase a supported child. This prevents an early coverage-only choice
from making later admission reject the only remaining name.

A qualifying broader context subsumes finer ones only when its own description covers all their
competition-supported members. This comparison follows every already-resolved
broader ancestor, including paths through inapplicable or unsupported intermediate concepts; those
intermediates contribute no scores or support. Per-origin visited sets prevent repeated expansion,
and ancestor-edge work has the same explicit edge ceiling as context traversal. Exhaustion rejects
selection without returning a partial name set. Closed relation cycles do not erase every
candidate. Remaining contexts are selected by additional member coverage, mean supporting improvement,
context contrast and stable identity. Each addition must cover the profile's minimum number of
previously uncovered members, preventing closely overlapping descriptions from multiplying categories.
Candidates keep their own scores; subsumption never inherits a finer description's evidence.
At most 64 retain full member evidence, and all other eligible contexts retain only compact coverage
and ranking data. The evaluation ceiling is independent of the caller's smaller output frontier;
asking for one name does not eliminate alternatives before their support is checked. Lexical origin
counts do not increase rank. The result records
considered/nameable/eligible/subsumed counts and candidate truncation. Final admission still resolves
competition and names among nominated contexts. Version 8 binds this evaluation-before-subsumption
order in the frontier and algorithm revisions; unchanged text and image features remain reusable.

Pruning a name does not remove its conflicting image evidence. Nomination retains the strongest
eligible, semantically distinct alternative for every member of each selected context. Distinctness
uses the original whole-image topic descriptions and an explicit, versioned text-feature relatedness
cutoff; identical topic descriptions remain related. Purpose-qualified descriptions still supply
image similarity. A shared purpose prefix must not make different topics exempt from competition.
Unconditioned naming uses the same descriptions for both comparisons. The cutoff is a measured
nomination input, not a probability or a product default. Alternative contexts are streamed after
evaluation-frontier selection, retaining at most 64 evaluated topic features and at most 64 features in each topic and
qualified-description window. Both views use independent description cache keys and the existing
worker lifetime. Evidence retains the alternative identity, qualified image similarity and topic
relatedness without promoting that alternative to a category. Final admission compares this evidence
alongside the current taxonomy and other nominated contexts. Incomplete or inconsistent comparison
evidence is rejected. Its explicit scope distinguishes selected-frontier comparison from complete
distinct-topic evidence, so losing every comparison row cannot silently fall back to the former.
Cancellation retires the same description worker and publishes no assessment.

Nomination binds its algorithm, profile, semantic taxonomy, selected identities and resource limits.
Uncovered retrieval seeds remain explicit. Missing descriptions use the existing supervised worker
and completed cache; every terminal path retires description work. Nomination neither requires user
category definitions nor grants category publication. Automatic naming quality and operating-profile
selection require their own measured admission before maintenance is enabled.

### Purpose-preserving names

Naming may refine an existing category while preserving its image purpose. A typed naming purpose
binds that category's identity and semantic revision to application-owned label and comparison
patterns. Each pattern contains one subject slot and explicit surrounding context. Vocabulary
adapters supply a bounded subject phrase independently of the raw retrieval description; parsing
dictionary glosses or inferring phrases from display names is outside the application contract.
Subject metadata changes concept authority without invalidating unchanged raw description features.

Nomination first resolves the original vocabulary graph, preserving retrieval provenance, then
projects its reached concepts into the selected purpose. The complete composed descriptions are
independently encoded and scored; raw noun scores never become scores for purpose-qualified names.
Missing subject phrases, overlong composed names and traversal-only concepts retain graph links
but cannot become naming candidates. No complete vocabulary pass or image decoding is repeated.

The current leading category of each member must match the naming purpose before it can support
a refinement. Other members remain explicit as a different purpose even when their topic similarity
is high. Nomination and final admission enforce the same scope; final admission also rejects a
missing or semantically changed parent and descriptions that do not match the bound composition.
Display-only parent renaming preserves this semantic authority. The naming revision accompanies
nomination and assessment, alongside the original vocabulary, taxonomy and comparison evidence.
Unconditioned nomination remains available for evaluation and category discovery. These contracts
do not select a fixed list of media types, make the user author patterns, or authorize category writes.
Useful local labels and the operating composition still require measured admission.

### Managed naming contexts

The application can compose the retrieval vocabulary with a separate, immutable naming-context
catalog. A context has its own identity, local display label, complete comparison description and
optional purpose-composition subject. Explicit cue links nominate it for evaluation; broader links
connect only declared contexts. The context catalog admits at most 4,096 definitions and 65,536
total links, rejecting duplicate definitions/links, unknown context parents and overlapping cue/context
identities. These bounds constrain the input vocabulary, not the number of active smart albums.

`ContextualVocabulary` preserves raw entry/related traversal and exact retrieval descriptions while
marking every source concept traversal-only. Dictionary labels and aliases therefore cannot acquire
name authority from a high retrieval score. Broader traversal merges the declared whole-image contexts
with existing source links through the same ordered, at-most-64-item windows. Context descriptions
receive their own scoring and purpose checks; a cue relation never grants image support or inherits
its score. An empty context catalog leaves retrieval available and nominates no names.

Context identity, wording, subject phrases and relations bind a canonical revision independent of
input order. The composed vocabulary also binds the underlying source revision. Changes during a
read, malformed underlying windows, context/source identity collisions and cancellation reject the
result. The composition has no model, source-media, persistence or category-publication capability.
Existing description features remain compatible when only local names or relations change.

The application supplies and evaluates these contexts; users need not author them. A context is a
candidate, not a pre-created category. Recurring-content nomination, useful-name admission and
guarded category maintenance still determine the active scheme. The retained calibration context
catalog is exploratory; its finite entries do not establish the product's permanent category set,
admit every purpose/topic combination or close the maintenance-policy requirement.

### Local vocabulary resources

`LocalClassificationVocabulary` loads a version-1 application resource through the vocabulary port.
The caller supplies the expected BLAKE3 digest; only those verified bytes are parsed. Loading checks
cancellation between 64-KiB reads and graph entries, rejects input above 64 MiB, and retains an
immutable in-memory snapshot. Later replacement of the resource file cannot change an active read.
The resource contains up to 100,000 concepts, 4,096 entry points and 1,048,576 source links, with at
most 16,384 links in one neighborhood. Naming contexts retain their separate 4,096-definition and
65,536-link bounds. Unknown fields, invalid definitions, duplicate identities/links and unresolved
references reject the complete resource. Reads return strictly ordered windows of at most 64 items.

Source keys remain inside the adapter. A stable source namespace and key derive opaque identities;
application-authored contexts have a distinct reserved namespace. Source concepts always remain
traversal-only. The adapter returns typed naming definitions for the existing application compositor,
which continues to own context nomination and category authority. Canonical revisions bind definitions,
entry points and each relationship independently of file order; display/relationship changes leave
unchanged description-feature keys reusable.

The Windows bundle supplies this resource from the pinned OMW English WordNet 2.0 and Chinese Open
WordNet 1.1 archives recorded in the distribution manifest. Their WordNet-style licenses permit
modified redistribution with the copyright notices and disclaimers retained. The bundle carries
both licenses, source citations and a modification notice. Source concepts remain traversal cues;
the separate application-owned naming definitions retain their measured context authority.

A build-only Python 3.11-or-later standard-library converter reproduces the admitted noun graph,
Chinese labels, entry points and context links. Local builds resolve the installed interpreter
through `uv`; hosted builds may use their installed Python. No interpreter is shipped or downloaded.
The build verifies archive, member and output identities, reads named regular archive members without
extracting paths, and writes only derived build storage. Runtime loading uses the expected BLAKE3
digest through the existing reader. Neither preparation nor loading enables analysis, reads source
media or changes a catalog. Updating this resource requires compatible context evidence and new
artifact identities; packaging the retained contexts does not close the broader coverage requirement
or fix the number of categories in a user's library.

### Candidate admission evidence

Admission compares each consolidated context against the current taxonomy through the existing
category matcher, using the cohort's retained image features. It also compares the context with
the strongest competing context for that same content. A supporting member must improve on the
current taxonomy and separate from its competitors; exact ties support neither proposal. Distinct
content identities, not files or lexical paths, determine support and coverage.

An immutable admission profile binds its calibration identity, required member count, coverage in
thousandths, improvement, separation, context contrast and bounded proposal count. It requires at
least two supporting contents. These parameters have no unmeasured product defaults. Profiles and
their measured selection remain separate from calibrated prediction confidence.

The application rejects mismatched model/cohort authority, traversal-only concepts, duplicate concepts,
incomplete member sets and invalid scores before assessing any candidate. Assessment preserves unsupported members
and the reason a candidate is deferred. Context contrast, support and coverage precede the proposal
budget; stable ties cannot silently discard another candidate's evidence. Existing and selected
display names are reserved case-insensitively; conflicting proposals are deferred without automatic
renaming. Cancellation returns no partial assessment. The result binds the current taxonomy's semantic
and display-name revisions separately, together with the discovery/consolidation/profile revisions.
A display-only rename preserves matching but invalidates stale name-conflict decisions.
It proposes additions only: it cannot replace existing categories, remap human choices or
publish predictions. Category maintenance retains its own guarded commit boundary.

### Whole-scheme comparison

A separate application assessment evaluates proposed definitions together with the current scheme.
Both matchers must use the same encoder compatibility, preserve every existing definition and label,
and produce identical scores for each existing category on each comparison sample. The input admits
at most 512 distinct content identities and 64 additions within the existing 256-category contract.
Duplicate identities, incompatible features and changed baseline definitions reject the assessment.

The comparison starts with all eligible additions. Definitions with the same complete description
set as an existing category refer to that existing identity; existing names remain reserved without
automatic renaming. A candidate's independent support must improve on the baseline and separate from
every other active candidate, using the admission profile's improvement, separation and recurrence
criteria. Exact ties supply no independent support. Cohort coverage, context contrast and naming
authority remain prerequisites owned by candidate admission; this comparison does not establish them.

The selector removes one unsupported candidate at a time, recomputing support after each removal so
equivalent proposals cannot eliminate every copy. It then resolves new-name conflicts and applies
the profile's addition budget. Removal prefers the least total marginal separation, then the fewest
independent members, then the highest canonical identity. Existing categories remain intact regardless
of their occupancy in this window. The result records sparse, overlapping, name-conflicting and
budget-deferred candidates separately. Deferred statistics describe their decision step; selected
statistics describe the final scheme. A budget is a ceiling, never a requested number of categories.

The result also retains each sample's old and proposed leading category. Final assignment uses the
matching contract's canonical identity tie-break; tied assignments do not become independent support.
The assessment revision binds semantic and display definitions, encoder compatibility, policy,
ordered content identities, baseline leaders and the comparison scores. Reordering the same input
preserves the result and revision. Cancellation returns no partial assessment.

This bounded comparison has no model, source-read, persistence or runtime-enablement capability.
Its selected window does not establish whole-library category usefulness or authorize retirement.
Persistent cumulative budgets, non-equivalent overlap policy, scheduling and first use retain
their separate maintenance requirements. Measured counterfactual budgets do not select a default.

### Single-category withdrawal comparison

The application can measure withdrawal of each current category against the unchanged complete
scheme on at most 512 distinct compatible cached contents. Each counterfactual removes one
definition only. An affected content keeps its former leading similarity and the highest-ranked
remaining category, with the ordinary canonical identity tie-break. Categories absent from the
sample window remain in the result with no affected members; absence supplies no retirement reason.

The assessment binds the semantic and display definitions, encoder compatibility, ordered content
identities and all category scores. Reordering inputs preserves the assessment. Duplicate content,
incompatible features, an empty or oversized window, and a scheme without a remaining alternative
reject the comparison. Cancellation returns no partial assessment. The result is bounded by the
taxonomy and sample window; the affected-member lists together contain each input identity once.

Similarity loss is the old score minus the best remaining score, not a probability or measured
classification error. Several independent single-category counterfactuals cannot authorize a joint
removal. Neither a small score gap, sample absence nor a similar label establishes semantic
equivalence, whole-library coverage, useful-count policy or permission to retire a definition.
These effects inform non-equivalent overlap admission alongside recurrence and naming evidence;
source coverage, human-choice guards and publication remain separate requirements.

### Catalog coverage for category maintenance

Maintenance can survey the current catalog's distinct exact contents through ordered windows of
at most 64 rows. A survey binds the catalog publication revision, complete classification setup and
dispatch authority. Each window uses a short read transaction; inference, source access and model
work never run inside it. Published direct fingerprints must still match cataloged file size, identity
and source revision. Retained display-only membership cannot prove current content. A source without
such evidence makes the report incomplete, even when another source already has a prediction.

Compatible predictions are read by content and configuration, retaining the latest published result
and its optional refinement. Reusing an earlier run with the same configuration is permitted;
results from another category scheme or encoder remain missing. Counts describe model suggestions,
not human choices. Exact copies count once. Unknown, missing and malformed prediction evidence is
never substituted with a category or silently treated as zero occupancy. The application validates
strict cursor advancement, complete category membership and matching content/configuration, and
checks the survey authority again after the final page.

The report retains counts for the bounded taxonomy and at most a caller-selected 1–64 content
identities per category, regardless of library size. All page results are bound into its evidence
revision. Source/configuration/publication changes or cancellation reject the survey without a
partial report. SQLite work is interruptible and releases its read transaction and cancellation
handler before returning. An incomplete or empty report supplies no category-retirement authority;
semantic relationships, independent-support policy, human protection and transaction revalidation
remain prerequisites for merging. This survey neither classifies images nor enables maintenance.

### Guarded category additions

Candidate assessment and category publication have separate owners. The application prepares an
immutable addition plan by reassessing the complete cohort against the current matcher and policy.
Category identity derives from the canonical concept identity and optional purpose-category identity,
independently of display labels, cohort, model, or vocabulary release. Previously known identities
are not recreated or overwritten. Active, manual and protected definitions remain intact. An inactive
generated definition can be reactivated under the fresh-admission rules below, using its existing
identity and origin rather than a second category.

Publication revalidates the classification and dispatch revisions, active encoder, and exact-content
membership of both support and comparison samples in one catalog transaction. A surviving verified
copy is sufficient; retained features alone are not source authority. The operation receipt binds the
prepared evidence identity and complete configuration. An identical committed retry returns current
state without undoing a subsequent pause, stop, or edit. Cancellation before commit rolls back both
the category addition and run replacement. No source-file access occurs in this transaction.

The changed category set creates an explicitly uncalibrated immutable setup; a screening profile
for the previous category set is not transferred. Existing runs and human decisions remain retained.
The addition boundary preserves the 256-category contract bound and per-assessment proposal limit.
Before preparing the configuration, it sends unknown admitted definitions through the shared joint
competition owner against the current taxonomy. Exact existing descriptions are deferred regardless
of a new name. Candidates compete for independent supporting members; membership rejected by naming,
purpose, context or the original admission pass cannot become support merely because a competing
candidate is removed. The published support list comes from the final selection.

The automatic cumulative capacity derives from the current catalog coverage and the admission
policy's recurrence requirement: `max(active_generated, min(256, verified_contents / minimum_members))`,
using integer division. Distinct verified contents count once even when predictions are unfinished;
unverified sources and duplicate copies grant no additional capacity. This is an upper bound, not a
target count or permission to create a category. Purpose/name admission and joint independent support
still decide which distinctions are useful. The retained count is a floor so a smaller or unavailable
library cannot turn this addition policy into retirement authority. Retirement keeps its separate
complete-coverage and human-protection requirements. Explicit numeric budgets remain available for
controlled comparisons, rather than requiring a user count during first use.

Every active category with a recorded automatic origin consumes a slot, including manually protected
definitions; manual/legacy categories consume only overall taxonomy capacity. Retired definitions
consume no active slot; reactivation consumes one without creating another identity. The effective
addition ceiling is the minimum of remaining generated slots, overall taxonomy slots and the
per-assessment proposal limit. An explicit budget below the retained count defers additions and
preserves existing definitions.

Publication rereads the active generated identities in the same transaction as source and revision
checks. They must match the prepared snapshot, and the resulting generated count must fit the supplied
budget. For automatic capacity it also validates the coverage's catalog publication, complete
configuration, encoder and source-verification authority. Count collection remains a bounded,
cancellable survey outside the write transaction. The operation evidence binds the budget, prior
active identities, coverage digest, verified-content count, recurrence threshold and selection
algorithm; changing that basis cannot replay an earlier operation under the same receipt. SQLite
validation/publication can be interrupted, with its handler removed before rollback or commit.
The origin read is bounded by the active taxonomy and at most 64 proposed identities. Capacity alone
does not establish naming quality, merge non-equivalent definitions or enable runtime maintenance;
broader context admission and first-use integration retain their separate requirements.

Schema 71 records immutable discovery evidence once per addition operation and a separate durable
origin for each generated category. Evidence retains the encoder/run, vocabulary and policy revisions,
actual work limits and admission criteria, complete cohort identities, naming parameters and member
support. Origins retain canonical concept/purpose, admitted definition and creation operation.
Explicit definition edits or retirement permanently protect that origin from automatic maintenance,
even if a later edit restores its former wording. Repeating an older configuration operation cannot
change that protection. Legacy categories without a recorded origin are treated as protected.

Portable archive version 6 includes category origins and manual protection, independently of derived
model evidence. Versions 1–5 remain readable and do not grant automatic authority to imported legacy
categories. Restoration cannot weaken existing protection or replace a conflicting origin. Missing
local discovery evidence requires fresh assessment before automatic maintenance; restoration itself
does not enable classification or recreate the source cohort.

### Reactivation of generated definitions

Automatic retirement preserves a generated definition's identity and creation origin. A fresh
discovery assessment can restore that definition only while it is inactive, its origin remains
unprotected, and its retained label and original semantic revision exactly match the proposed
definition. Active categories, legacy definitions without an origin, manual retirement or editing,
and changed naming or meaning remain excluded. Reactivation does not remap a saved human choice.

Restoration passes the same naming, member recurrence, coverage, contrast, joint competition,
cumulative capacity and current-source checks as a new category. Old origin metadata alone cannot
restore a category without fresh admission. The immutable plan records new creation versus
reactivation explicitly and binds the original origin, including its creation operation. Before
publication, the transaction rereads retained labels, origin/protection records and active membership,
in addition to the existing configuration, run, dispatch and source guards.

The active scheme, new uncalibrated run, operation receipt and fresh evidence commit atomically.
Reactivation reuses the stored definition/version and leaves its original origin and creation
evidence unchanged. A committed retry returns current state and cannot resume a later explicit stop.
The version-1 admission payload adds activation provenance and its evidence identity advances to
version 3; schema 72 and archive version 6 retain their existing shapes and readers.

### Equivalent-category retirement

The application can consolidate existing definitions with exactly the same complete description set
and naming purpose. This proves equal matching scores independently of the sampled images; a similar
label, similar embedding or small occupancy supplies no retirement authority. A generated category
is eligible only while its current name and semantic revision match its recorded origin. Missing
origins, explicit definition protection, a current human classification reference and use as an active
generated category's purpose all retain the definition. Protected equivalents can coexist.

Within an equivalence group, protected definitions have retention priority, followed by canonical
identity. At most 64 eligible definitions retire in one operation, with a stable retained identity
across subsequent rounds. Definitions, semantic history, origins, accepted human evidence and operation
history remain stored. No human choice is remapped and no original image is read or modified.

Preparation reads the bounded active taxonomy, origins and indexed human-category references.
Publication repeats the decision within the configuration transaction and requires the exact saved
catalog/dispatch state, origin/protection data, human references and compatible active run. Pause,
manual changes, stale run authority or cancellation reject uncommitted work. Categories, an explicitly
uncalibrated replacement run, the operation receipt and immutable retirement evidence commit together.
A committed retry returns current state without reactivating a later stop. Automatic retirement does
not become a manual edit or weaken an existing protection flag.

Schema 72 preserves version-1 admission evidence while admitting version-2 equivalent-retirement
records in the same bounded evidence table. Each retirement record identifies the former and retained
definition, their semantic revisions and labels, generated origin, human-category protection and new
run. Portable recovery continues to retain definitions/origins and omit derived maintenance evidence.
Non-equivalent overlap decisions, cumulative usefulness budgets and runtime scheduling remain separate
requirements; this equivalence proof does not supply their thresholds or enable maintenance itself.

### Purpose-preserving sparse-category merge

A generated refinement can return to the purpose category recorded in its original admission when
complete current-content coverage no longer meets that admission's repeated-content requirement.
The original naming purpose, its semantic revision and the admitted minimum membership supply the
relationship and threshold. Similar labels, small score differences and a new caller-selected count
do not establish this authority. Missing creation evidence, changed parent meaning, manual protection,
human-category references or active purpose children preserve the generated definition.

Coverage retains a bounded second-choice result from each final compatible prediction. A merge
requires every currently assigned content to be represented and its unique best remaining category
to be the recorded purpose. A tied or different alternative defers the merge. Complete zero occupancy
can retire an unchanged refinement to its unchanged declared purpose, but an empty or incomplete
catalog cannot. Categories still meeting their original recurrence requirement remain active.
No category is removed to force a requested count. Unconditioned categories and unrelated contexts
remain outside this merge rule.

If no original-purpose merge qualifies, a sparse refinement can merge into an existing broader
generated context under the same original purpose. Every retained source member must have that
target as its unique best alternative, and the target must still meet its own original admission's
recurrence requirement. Both original naming patterns and purpose revisions must agree; projecting
the current context definitions must reproduce both active semantic revisions. The naming catalog
must declare a strict broader path of at most four edges. Reverse reachability anywhere in the
bounded catalog rejects cyclic containment. Similar names or scores alone supply no relationship.
Zero occupancy supplies no alternate-context destination and retains the original-purpose rule.

The immutable plan binds the naming-catalog revision, both context revisions and their path. Commit
revalidates source eligibility, target evidence, definitions and support under the same complete
catalog authority. The target definition is retained unchanged; source human decisions and manual
protection still prohibit withdrawal. Existing category order cannot change the chosen relation.

One operation removes at most one definition. A subsequent merge requires fresh complete coverage
under the replacement scheme; independent single-removal assessments cannot authorize a joint change.
Preparation and publication retain catalog/configuration authority, origin and original naming
evidence, human references, cancellation and idempotent receipts. The replacement is explicitly
uncalibrated; original definitions, origins, human choices and past runs remain stored. No image or
model access occurs in preparation or the write transaction.

Schema 73 extends immutable maintenance evidence with version-3 purpose-merge records while preserving
versions 1 and 2. The migration preserves existing payloads atomically and needs no media rescan.
Broader-context merges use the same version with a distinct algorithm identity, retained category,
context path and target's original admission evidence. The purpose field continues to identify the
original naming purpose; no further schema change is needed.
Portable archive version 6 remains unchanged; missing restored model evidence grants no automatic
merge authority. This native operation supplies a maintenance action without enabling its scheduler
or selecting the remaining useful-count and first-use operating policy.

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

Description lookups use windows of at most 64 keys. The SQLite adapter validates the complete window
and updates its durable recency in one short transaction, preserving requested order, missing entries
and duplicates. A corrupt entry rejects the window and rolls back its recency updates. Empty windows
perform no database work, and oversized windows are rejected before access. The transaction ends
before any missing description is encoded; each completed encoding is still stored immediately.
The preparation owner deduplicates descriptions within a window while retaining encoding order,
cancellation checks and model/description identity validation. This changes storage scheduling only;
the admitted one-description inference policy and cache durability remain unchanged.

The initial ranking uses L2-normalized vectors and maximum description cosine per category, retaining
the comparison's measured aggregation rather than introducing an unvalidated averaging rule.
Ties have stable category-ID ordering and zero separation. A single-category scheme has no runner-up
margin. Ranking produces similarities and provenance; it neither manufactures probabilities nor
marks a result human-confirmed. Optional review, bounded re-screening and calibrated confidence are
application policies that consume this evidence, separately versioned with their calibration data.

### Bounded screening policy

The stored recommendation/review-band names below remain compatible analysis evidence. Following
withdrawal of manual classification review on 2026-11-01, they support quality evaluation and bounded
automatic refinement only; they create no review queue, confirmation action, or manual-time gate.

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
Thresholds come from an admitted, version-bound profile; the matching domain has no universal cutoff.
The [calibration evidence](../acceptance/r5-calibration.md) owns the measured initial candidate and its
limitations. An application-owned candidate binds its exact primary/refinement descriptions,
category IDs, calibration identity and frozen native model revisions. Explicit enablement of that
category set selects its measured profile. Display renames preserve it; changed descriptions, category
membership or encoder pipelines produce uncalibrated suggestions until a compatible profile is
admitted. The six categories establish the measured screening reference only. They do not complete
application-managed category discovery or determine the product's permanent output vocabulary.

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

The native source-input adapter uses the existing `image` 0.25.10 Rust codecs, detects format from
bytes, applies EXIF orientation, converts to RGB8 before Triangle resampling and builds the admitted
tensor. The image-pipeline revision binds this implementation; tensors prepared by another resize
implementation do not silently share its feature cache or screening profile. Reuse of the exploratory
Pillow calibration requires a measured native-input comparison, without retuning against its holdout.
The [fixed-policy comparison](../acceptance/r5-calibration.md#native-image-input-comparison) retains
the numerical cutoffs as a native-input integration candidate with a distinct pipeline revision.
The decoder's GIF first frame uses a transparent canvas outside its encoded rectangle; conversion to
RGB therefore differs from Pillow's palette-background fill in that region. This behavior remains
part of the versioned native input and is covered by a partial-frame fixture.
Decode admission bounds encoded content to 256 MiB, either dimension to 16,384, decoded area to
16,777,216 pixels and decoder allocation to 128 MiB. I/O checks cancellation, with additional checks
between decode, orientation, resize and encoding. Inputs outside these limits produce a per-item issue.
No alternate native codec or implicit cloud download is attempted.

The source adapter opens the existing read-only file/namespace capability against the current root,
physical identity and source revision. Without durable identity/revision evidence it also compares a
bounded exact hash with the leased content before decoding or using a cached feature. The capability
stays held through inference, cache insertion and the prediction/checkpoint transaction. Compatible
cache hits still revalidate the source but skip both decoding and image inference.

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
repository. The Windows installer includes these three artifacts with the Apache-2.0 license and
conversion notice under `data/classification-models`. Installer ownership covers their upgrade,
repair and removal under ADR 0027. Portable/development distributions may omit that payload and use
explicit managed download. Model files remain outside Git, catalogs, previews and source trees.

### Managed model installation

Explicit installation downloads only the three admitted model artifacts from the pinned conversion
revision above. The application resolves local model storage separately from catalogs and previews;
the runtime resolves from its own `data/classification-runtime` bundle. Callers select neither URLs
nor runtime libraries. Preparation does not enable classification, open a worker, or change a taxonomy.

Startup first recovers a previously authorized, unfinished download. Otherwise, an unselected bundled
model is verified in place with read-only held-file/namespace guards and then selected through the
same guarded publication boundary. No writable lock or duplicate model copy is created in the
installation directory. Missing bundles do not authorize a network request. The existing automatic
classification lifecycle activates the verified selection and reuses completed analysis evidence.

A separate settings-database migration preserves installation selection and adds one versioned
download intent. Its generation binds cancellation and publication; a stale operation cannot retire
a newer request. Model revision is part of that intent. Closing the application signals and joins
the active transfer while retaining intent and partial bytes. Explicit cancellation retires intent
after the transfer joins. Committing model selection retires the matching intent atomically. On
restart, current artifact lengths reconstruct progress and the existing notification center receives
the recovered task. No catalog migration or persistence of arbitrary notification callbacks is needed.

The Windows transport is `reqwest` 0.13.5 (MIT/Apache-2.0), with blocking streaming, native TLS and
system proxy support. Windows certificate validation uses Schannel; automatic decompression,
cookies and automatic retries are absent. Reqwest supplies maintained HTTP/redirect/TLS handling
behind the artifact-download adapter. HTTPS redirects are limited to five hops within Hugging Face's
`huggingface.co` and `hf.co` hosting domains. Credential-bearing URLs, other ports and unrelated
hosts are rejected. Errors omit signed redirect URLs and server bodies. The upstream
[blocking reader](https://github.com/seanmonstar/reqwest/blob/v0.13.5/src/blocking/response.rs)
applies the configured ten-second timeout per read, allowing a large model to stream without a
ten-second whole-file cutoff. A transfer also has a one-hour total deadline. Cancellation is checked
between reads, writes and hash windows; a stalled network operation can delay observation until its
ten-second timeout. The owning call retains the response until transfer has stopped. No detached
thread can continue writing or publish after cancellation returns.

An application installation owner spans preparation and publication. Downloads use one 64-KiB
buffer, exact length limits and incremental SHA-256. Each artifact has one deterministic partial
file whose name includes its pinned digest. The partial is reopened without following reparse or
cloud entries and its existing prefix is rehashed before an HTTP Range request. Admit a 206 response
only when Content-Range exactly matches the requested offset and pinned total; a valid 200 response
restarts only this incomplete file. Wrong ranges, encoding and oversized transfers fail closed.
Complete bytes pass length/digest verification before synced publication without replacing a file.
Its final file handle is independently verified and held against changes until publication. Verified
complete files and bounded partials survive interruption and are rechecked before reuse. Invalid
partial bytes are reset within this installer-owned namespace; corrupt final artifacts still fail
admission. A package-directory file lease excludes concurrent installers
across processes. Under that lease, a bounded directory read retires only the exact temporary naming
format from the previous non-resumable installer. The three current digest-bound partials remain;
repeated interruptions cannot accumulate another full partial per attempt. Complete artifacts and
unrelated entries remain untouched. Existing corrupt, reparse or cloud-placeholder entries fail
admission without download, replacement or hydration. License and conversion/quantization notices
are installed from application-owned text and remain alongside the model artifacts.

Preparation leaves the prior installation selected. Publication requires the existing runtime to
have fully retired and compares the expected settings within the same immediate transaction that
saves the complete installation. Cancellation is rechecked after writer admission and before commit.
Cancellation, changed settings and retirement failure preserve the previous selection. The prepared
owner retains all file and namespace guards until this commit or disposal. The caller then opens a
fresh session through the existing configuration lifecycle; there is no runtime hot reload.

The desktop bridge exposes one opaque installation operation. Preparation and publication use
asynchronous calls; bounded progress reads and cancellation use synchronous signals. Cancellation
retains a running preparation until its owning call returns. A prepared operation owns its verified
files until commit or disposal, and cannot be prepared or committed twice. The desktop application
owns that lifetime independently of the settings dialog, joins it before classification shutdown,
and changes configuration only after preparation completes. Enablement retains the observed
configuration revisions so a download cannot silently overwrite a later pause or category change.

This adds no catalog migration or new encoder identity. Replacing the HTTP adapter must preserve
fixed artifact admission, bounded streaming, cancellation, complete-file reuse and atomic selection.
Model or runtime revision changes retain the original admission and feature-compatibility rules.

### Worker ownership and protocol

The default `classification-worker` Cargo feature builds a separate native executable and its adapter.
Windows desktop bundles include that executable beside Ame; portable and installer payload checks
require it. The admitted CPU runtime ships in `data/classification-runtime` with its license,
third-party notices, privacy documentation, version and upstream revision. Installer model weights
use the separately verified payload above; optional managed downloads keep their own storage and
resume lifecycle. Bundling the compatible runtime removes a second user installation choice and
binds it to the application build. Desktop startup does
not load an inference runtime. The worker accepts one encoder for its
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
the bundled DLLs.

Model preparation validates both encoders and the bundled runtime before storing the installation
in a separate versioned settings database. The worker repeats held-file verification when an encoder
is demanded. Desktop activation serializes installation publication and analysis configuration through
the existing runtime-retirement boundary. Preparing files alone does not enable analysis. Feature
storage is a separate rebuildable database beside the active catalog, outside the source tree.

The Windows CMake dependency prepares the hash-pinned official runtime archive before the application
target can complete. Its distribution manifest records archive and individual-file lengths and
SHA-256 digests, with a regression binding both DLL entries to the worker's compiled artifact pins.
Preparation streams at most the admitted archive length with a five-minute timeout, extracts only
the seven admitted files, verifies them and publishes a complete new build directory by rename.
Interrupted staging is disposable; an existing build directory must pass every artifact check before
reuse. Corrupt existing output fails the build for explicit inspection rather than being silently
trusted. The package cache lives under repository `build`; no runtime download occurs during app
startup. Portable, unsigned and installer payload checks require the runtime and all notices.
Model graphs, headers, import libraries and debug symbols are excluded from this runtime payload.

An upgrade replaces the bundle through the existing application packaging lifecycle. Runtime version
or digest changes require renewed adapter admission and feature compatibility; there is no fallback
to a system-installed runtime or a different DLL found on the search path. Reverting a bundle restores
its matching runtime without changing model files, catalogs or human decisions.

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

Fixed model output classes would prevent application description updates and are rejected. Re-encoding images for
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
