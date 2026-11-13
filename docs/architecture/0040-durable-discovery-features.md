# ADR 0040: Share image features without making the LRU cache a search index

- Status: Accepted
- Date: 2026-11-07

## Context and alternatives

Classification and semantic discovery consume the same SigLIP 2 image embeddings. The existing
256-MiB classification cache may evict vectors and cannot establish durable search coverage.
Re-encoding everything for a second feature consumer wastes source reads and model execution.
Expanding that cache would still couple coverage to its eviction and explicit-cleanup policy.

## Decision

`domain/image_features` owns normalized F32 embeddings, model compatibility and analysis revisions.
Classification retains category policy and its existing revision aliases. Preserve existing hash
contexts and stored feature identities; moving ownership does not invalidate compatible features.
CPU and GPU image pipelines retain the distinct identities required by [ADR 0039](0039-gpu-classification-execution.md).

Use a separate, rebuildable SQLite feature store beside the catalog, outside source trees. Its
identity includes exact content scheme, digest and byte length, embedding space, image pipeline and
dimensions. Store original F32 bytes with a checksum. Never overwrite a completed feature under the
same identity; publication atomically advances its generation and storage accounting.

The initial vector/entry budget is 1 GiB, with a 2-GiB SQLite page limit and 2-MiB page cache.
Reads validate bounded payloads before decoding. Reaching the entry budget stops
new feature publication, preserves existing features and records incomplete storage coverage. It
does not evict old vectors or prevent classification from using its ordinary bounded cache.
Catalog membership, current source identity and active model compatibility remain separate from
the number of stored features; storage counts cannot be presented as current library coverage.

`SharedImageFeatures` reuses durable compatible features, then the existing classification cache,
before encoding. Reused cache entries and newly verified image features are published independently
of taxonomy results. Text features retain their existing cache. Clearing that cache leaves discovery
features intact; discovery storage retains its own rebuild/reset operation and accounting.

Storage adapters retain namespace and file-identity guards, schema admission and atomic creation.
Unknown or damaged storage reports an error. Rebuilding this derived store cannot alter the catalog,
saved decisions, source images or model installation. No media read or model execution occurs while
a storage transaction is held. Classification's source lease and publication revalidation remain
in their existing application owners.

Search retry retires and joins the analysis runtime before checking the feature store. A separate
ownership receipt binds a successfully admitted store to its physical file identity. Recovery may
reset a corrupt store only while retaining its namespace, file and receipt guards; a substituted or
unrecognized file is rejected. SQLite owns reset locking and journaling. Missing stores are recreated,
healthy features are retained, and busy or I/O failures remain errors. An interrupted reset can be
completed from the same ownership proof. Restarted indexing reuses compatible classification-cache
entries and resumes normal source admission for missing features.

## Retrieval engine evaluation

USearch 2.26.6, Apache-2.0, was evaluated with default features disabled on Windows x64. The retained
768-dimensional F32 corpus supports filtered search, mapped reopening and removal through its Rust
adapter. Search expansion 128 missed one exact top-ten neighbor; expansion 512 recovered all exact
top-ten neighbors in that corpus. This small corpus does not establish large-library recall.

Engine integration must retain exact cosine comparison, eligibility during retrieval, bounded
resident windows and cancellable work between windows. Engine keys and file formats remain behind
the retrieval port. The evaluation does not add USearch to the application dependency set or make
its approximate result count the coverage authority. The [R7 evidence](../acceptance/r7-semantic-discovery.md)
owns measurements and production admission evidence.

## Relevance admission

Search publishes matching candidates rather than every indexed item sorted by distance. For the
admitted SigLIP 2 package, description results require cosine at least 0.09 and reference-image
results require at least 0.65. The threshold is independent of the selected source or folder.
Expanding a scope must not reject existing matches because another candidate scores higher.
Retain every qualifying candidate in the selected scope, ordered by relevance; never pad an empty
or short result. The former 500-result cap is retired. Feature reads retain 256-item windows;
the result manifest holds matching source identifiers and generations, while photo metadata and
previews use the existing gallery pages. Sort qualifying identifiers once after the scan instead
of repeatedly sorting and truncating each candidate window. Manifest memory grows with the number
of matches; it does not retain the scanned embeddings or decoded photos.
Manifest identity versions the policy. Indexed coverage continues to count analyzed candidates,
including rejected matches.

The former best-candidate gap (0.025 for descriptions, 0.08 for references) is retired: a stronger
match in a broader scope raised the cutoff and removed otherwise matching images. Fixed admission
preserves scope consistency, with relevance ordering keeping stronger matches first.

These are initial relevance heuristics checked against the retained bilingual corpus, not confidence
percentages or a guarantee of semantic correctness. Text and image similarities have different
distributions. Model/pipeline replacement requires recalibration; do not silently reuse these values.
The evidence owner records known misses and unrelated-query results alongside retained target ranks.

## Consequences, validation and replacement

Discovery uses additional disk space to retain features that an LRU cache could discard. Its budget
and partial coverage must remain visible. Existing analysis can continue at the ordinary cache
boundary when the discovery budget is exhausted. Other storage errors retain their actual cause.

Verify real SQLite reopen, compatibility separation, quota retention, transaction rollback and
corruption rejection. Reuse existing classification and source-admission regressions for unchanged
policy. A replacement storage adapter preserves feature identity, precision and publication semantics;
a replacement retrieval engine can rebuild from these features without repeating inference.
