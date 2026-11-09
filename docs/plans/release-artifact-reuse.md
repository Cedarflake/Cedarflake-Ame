# Reuse verified Windows release artifacts

Scope: continue a release after changing release tooling without rebuilding unchanged application
payloads or repeating completed application gates. ADR 0015 retains publication authority and
immutable source, signature, package and tag checks. Existing public packages are not replaced.

## Implementation

1. Add a read-only preparation owner to the existing release entry. Automatically inspect recent
   main-dispatched release runs, or an explicitly selected run. Resolve the exact tag and compare
   workflow inputs before selecting fresh verification, candidate reuse or package reuse.
2. Admit candidates only after successful complete candidate gates. Admit packages through a
   successful packaging job and an immutable receipt containing source, channel and package hashes.
   Revalidate downloaded bytes, reject expired or mismatched artifacts, and retain artifacts for
   fourteen days. Missing eligible artifacts require fresh verification; explicit invalid reuse
   requests fail with an explanation.
3. Separate current trusted packaging tools from tagged application version and model metadata. Packaging
   changes repackage a verified candidate; publication/notes changes reuse the verified package
   pair. Unknown or candidate-verification changes retain complete candidate verification.
4. Add a read-only validation mode for exercising reuse selection and downloads without building,
   signing, installing, uploading release assets or publishing a release. Keep content-write
   authority only in the existing main-only publisher.
5. Route release-tooling-only pushes and pull requests through workflow syntax, packaging and
   publication fixtures. Mixed application changes and unknown owners retain normal full checks.

## Verification and delivery

Use focused passing/rejecting fixtures for changed-stage selection, run/artifact provenance,
receipts and immutable file admission, plus the existing workflow and publication checks. Exercise
the read-only path against retained CI artifacts. Do not rerun a full candidate merely to validate
orchestration changes. Record execution evidence in the existing unsigned-release acceptance owner.
