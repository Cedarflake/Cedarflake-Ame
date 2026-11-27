# Microsoft Store package verification

The [Store distribution plan](../plans/store-msix.md) owns implementation order and disposition;
[ADR 0045](../architecture/0045-microsoft-store-distribution.md) owns package and data behavior.

The direct-directory startup passed the unsigned Release, catalog-free bridge smoke and installed
client checks below. MSI lifecycle verification remains in progress.

## Shared data lifecycle probe — 2026-11-27

An isolated Windows 11 build 26100 guest deployed two temporary, signed MSIX versions using the
reserved identity. The fixture included an executable only to satisfy manifest admission; it did
not launch a complete Ame application. Certificate trust existed only inside the disposable guest.

A newly created standard user performed direct filesystem access, package installation, upgrade,
uninstall and reinstall without an elevated token. Five scenarios passed:

- Data created beneath the package family's `LocalState/Ame` before first installation survived.
- Upgrade preserved data written after installation.
- Uninstall removed the shared data.
- Unpackaged writes after uninstall survived a later installation.
- Uninstall after reinstallation removed those data again.

The ordinary-user report is `build/test/derived/msix-data-probe-standard/result.json`; its owner
receipt records retirement of the exact guest. An earlier elevated deployment probe observed the
same data behavior, but did not satisfy the ordinary-user requirement and its harness rejected that
evidence. These probes cover directory lifecycle only; installed-client observations are recorded
below. Native worker qualification and Store certification retain their separate boundaries.

## Installed client observations — 2026-11-27

An earlier full-client package was installed and activated with the reserved package identity under
the disposable guest's ordinary interactive account. Portable and MSIX clients opened the same
unified data, including the dark theme, favorite `01.png` and the `msix` album containing `01.png`,
and enforced one shared application instance. Both clients caught up images added while Ame was
closed.

The packaged UI recycled `04.png` into the Windows Recycle Bin. The remaining source-image hashes
were unchanged. Settings showed Store-managed updates in Chinese, English and Japanese, and the
Open Store action invoked the fixed Store handler. Microsoft Store was absent from the Sandbox, so
the destination product page was not opened there.

Evidence is retained beneath `build/test/derived/msix-client`, including `install_current.json`,
`portable_shared.json`, `singleton-observation.json`, `store_catchup.json`,
`packaged_offline_sync.json` and `file-operation-observation.json`, alongside the shared-data
snapshots. The first snapshot failed in the harness's `Copy-Item` operation on a long preview path,
as recorded in `migrated_snapshot.json`. A subsequent
`robocopy` snapshot succeeded at `migrated_complete`; the failure belonged to the snapshot operation.

The final icon resource index uses the existing Windows icon artwork, scaled assets and unplated
target-size variants. A real package upgrade retained the existing library, and the taskbar showed
the transparent gradient Ame icon. The resource map resolves the manifest's `Files/Assets` paths.

The direct-directory build was deployed as isolated test version `1.1.7.0`. It started with a fresh
library without importing earlier defaults. Through Settings, the ordinary user selected an external
catalog directory, restarted, imported five fixture images and favorited one image. Uninstall
removed the shared default root; all 14 external catalog/cache/source files retained their SHA-256
hashes, including the five originals. The earlier external-preview-only uninstall also retained its
three cache artifacts and five originals. Reports are `direct-upgrade.json`, `direct-uninstall.json`,
`direct-external-before.json`, `direct-external-after.json` and `external-cache-uninstall.json`.

The final harness initially read a JSON array as one nested array under Windows PowerShell 5;
correcting that read completed the per-file comparison without another uninstall. The test account's
membership was restored, temporary certificate trust removed, and the owned client guest retired.
The seven-case MSI lifecycle matrix remains pending.

## Store update selection

The targeted application and presentation run passed 15 checks covering direct delivery and Store
selection, exclusion of direct preferences/results/downloads, coalesced Store launches, shutdown
joining and launch-failure recovery. The direct-directory unsigned Windows Release build, three
native runner lifecycle cases, two engine-retirement cases and catalog-free bridge smoke passed.
Packaged settings and Store-handler observations are recorded above.

## Package and removal boundaries

The Windows SDK built a 424,145,664-byte x64 MSIX from the admitted direct-directory Release payload.
Its SHA-256 is `e19df6858f5e6dc451878c015cb411e6cfc3e7b1f9fca3ad695633e3b679b457`.
Archive verification matched 72 payload files and 62 generated files by length and SHA-256.
The package version is `1.1.4.0`; it declares only `runFullTrust`. Package admission checks cover Store version mapping,
required worker retention, deployment-helper exclusion, duplicate and escaping archive paths,
missing entries and changed bytes. This is an intermediate dirty-worktree build, not a publishable
release candidate. It includes the same pinned models and license notices as the existing
distributions. The earlier model-free packaging probe is superseded by this complete payload.

The native MSI policy executable passed 99 cases, including the new current-account removal
boundaries. Its temporary files were constrained to leased scratch storage. This establishes the
native policy and filesystem cases; interactive MSI cancellation, upgrade and credential-elevation
behavior still require installed-package verification.

The first MSI fixture attempts exposed harness errors: a released process handle hid the observer's
exit code, an obsolete runtime entry remained in its file inventory, and the alternate-account
assertion expected English action text in a Chinese MSI log. The real alternate-account removal
returned 1603 from `AmePrepareUserDataRemoval` with the expected account-identity denial. The next
assertion incorrectly required a continuously running broker: its production host exits successfully
after ten idle seconds. The fixture now records the service state and exit codes while retaining
registration, identity and data-hash checks. Failed attempts and their retired guest receipts remain
in `build/test/derived/msi-data-cleanup-*`; they are not counted as complete lifecycle runs.

## Review boundaries

The changed update controller contains 327 production lines, with no inline tests; the new Store
delivery suite contains 124 dedicated test lines. Distribution selection is a separate owner from
download orchestration. MSI cleanup is split by account identity (134 lines), filesystem cleanup
(127 lines) and MSI action lifecycle (93 lines), with 200 dedicated filesystem/policy test lines.
Package assembly contains 118 lines and icon/resource generation 57 lines. These counts describe
the reviewed handwritten owners, not additional acceptance gates.
