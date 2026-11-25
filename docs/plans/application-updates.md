# Application updates

The [roadmap](../roadmap.md#application-updates) owns priority and delivery status.
This plan implements the accepted Settings entry and Windows update workflow together.

## Outcome

Keep one installation entry and a portable package. Check for a newer
release from Settings, read its notes, download it and explicitly restart into that version.
Automatic checks and optional automatic downloads share the same state and notification entry.
The installer retains the protected journal service's installation and rollback guarantees.

## Accepted boundaries

- ADR 0043 selects WiX for the installed application/service transaction and Velopack 1.2.161 for
  portable replacement. Both use the same Ame update workflow.
- Keep one install/update entry. Explain any required elevation in the selected installation flow.
- Use HTTPS and SHA-256 integrity checks. Neither code signing nor feed signing is included.
- Old Alpha installations may be uninstalled and replaced manually. Automatic migration of the
  former application-and-service MSI is outside this change.
- Preserve catalogs, preferences, organization decisions, operation history and original media.
- Keep application updates outside catalog work. Checks and downloads must not delay startup,
  scans or browsing. Joining a safe shutdown precedes replacement of running application files.
- Release publication and merging remain separate from this implementation.

## Selection checkpoint

| Candidate | Existing capability to reuse | New responsibility | User-visible tradeoff |
| --- | --- | --- | --- |
| Keep WiX | The per-machine MSI already handles application/service upgrades, protected paths and transactional rollback. | Release discovery/download, a coordinated installer handoff, and a separately verified portable replacement helper. | Installed updates request elevation; the existing installation layout remains. |
| Velopack | A common application package/update engine for installed and portable use. | Admit the Rust SDK, change release artifacts, separate broker ownership, integrate startup hooks and cover the pinned SDK's cache/path validation gaps. | Ordinary per-user application updates avoid elevation; service changes still need it. |

Select for a complete installation lifecycle and maintainable update behavior, not minimum diff
size. Validate first installation, subsequent upgrades, repair, removal, portable replacement,
service upgrades and failure recovery as one supported product. Reusing WiX must connect the full
upgrade path; adopting Velopack must also resolve service ownership and maintenance. Do not make the
user manage disconnected installation components. Settings, notification, preference and channel
contracts do not depend on the engine choice.

The selected division preserves the existing complete installed product and uses a mature portable
replacement engine. Velopack 1.2.161's
[cached-package handling](https://github.com/velopack/velopack/issues/1175) and
[feed filename handling](https://github.com/velopack/velopack/issues/1135) require adapter-level
admission in the native adapter. Full packages receive SHA-256 verification before engine use.

## Implementation order

| Slice | Change | Evidence needed |
| --- | --- | --- |
| 1. Selection and contracts | Compare retaining WiX with Velopack for installed and portable updates, service ownership, integrity, recovery and maintenance; record the selected approach. Define engine-independent channel, release, progress, failure and restart contracts. | Existing installer capabilities, actual dependency behavior and a concrete end-to-end update path without unnecessary installation changes. |
| 2. Native update engine | Discover published releases, admit HTTPS assets and hashes, download to managed update storage and prepare a verified update through the selected adapter. | Real package inspection, corrupt/cached/traversal rejection, cancellation, interrupted download and replacement fixtures. |
| 3. Application coordination | Persist preferences, schedule checks after startup, retain one update session and coordinate restart with the existing shutdown owners. | No concurrent duplicate download, retained state, failure/retry, preference/channel changes and joined file-operation shutdown. |
| 4. Settings and feedback | Add the version-row action, child update settings and compact update dialog; connect existing notifications and all three languages. | Actual entry, download, dismissal, retry and restart paths; keyboard, narrow layout and enlarged text. |
| 5. Distribution | Produce installation and portable artifacts plus update metadata from the same verified candidate; retain or revise the installer only as required by the selected approach. | Exact payload manifests, service install/repair/upgrade/rollback/removal and portable update behavior. |
| 6. Delivery | Connect hosted gates, exercise two consecutive disposable versions, document the supported installation and update paths, commit and push the complete feature. | Fresh install and A-to-B update, failed update recovery, preserved user data, exact-commit CI and affected Windows client checks. |

The slices are dependencies within one feature branch, not separate unfinished UI releases.
When a pinned dependency cannot meet an accepted boundary, isolate or repair that adapter behavior
before connecting it to the application. Do not relax integrity checks or substitute a success state.

## Verification scope

Use temporary application versions, controlled feeds and disposable user profiles. No real image
root or existing installed service is a mutation fixture. Use Windows Sandbox, the authorized cloud
test computer with normal UAC, or the existing hosted installer lifecycle owner for elevated service
changes. On a retained test computer, verify that no Ame installation or service exists before
installing the fixture; remove the owned product and disposable account/profile afterward. Reuse
the installed Flutter SDK, repository locks, existing notification templates and release-candidate
manifest verification.

Require SHA-256 verification for both newly downloaded and retained packages before use. Evaluate
incremental packages only with evidence for their reconstructed payload; they are not a prerequisite
for the accepted update experience.

## Rollback

Preserve the selected installer's transaction and recovery boundaries. A failed check or
download retains the running version. A failed service transaction restores its prior registered
service. Never roll back catalogs automatically or install an older application when switching channels.
Undo implementation through an ordinary scoped commit; do not rewrite published release assets.
