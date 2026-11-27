# Microsoft Store package verification

The [Store distribution plan](../plans/store-msix.md) owns implementation steps; the
[roadmap](../roadmap.md#microsoft-store-distribution) owns disposition;
[ADR 0045](../architecture/0045-microsoft-store-distribution.md) owns package and data behavior.

The direct-directory startup passed the unsigned Release, catalog-free bridge smoke and installed
client checks below. The MSI lifecycle matrix and visible Setup maintenance also passed below.

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
The MSI lifecycle results are recorded below.

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
native policy and filesystem cases. Installed-package verification follows below.

The first MSI fixture attempts exposed harness errors: a released process handle hid the observer's
exit code, an obsolete runtime entry remained in its file inventory, and the alternate-account
assertion expected English action text in a Chinese MSI log. The real alternate-account removal
returned 1603 from `AmePrepareUserDataRemoval` with the expected account-identity denial. The next
assertion incorrectly required a continuously running broker: its production host exits successfully
after ten idle seconds. The fixture now records the service state and exit codes while retaining
registration, identity and data-hash checks. Failed attempts and their retired guest receipts remain
in `build/test/derived/msi-data-cleanup-*`; they are not counted as complete lifecycle runs.

The retained `msi-data-cleanup-lifecycle` guest completed fresh installation, alternate-administrator
cleanup rejection, repair of a missing installed file, a real `0.1.4` to `0.1.5` major upgrade,
default uninstall with data retained, full-UI cancellation after selecting data removal, and explicit
default-data removal. Each retaining case verified eight file hashes under an ordinary observer
token; deletion retained all four external/cache/source/sibling files. Cancellation returned 1602;
alternate-account removal returned 1603. The combined evidence is `remaining/result.json`, linked to
the original case logs. MSI inputs were `B8AE0E0A8DC5F238413237A9A31A0E0BCDFF8076566EE9A3631510399068CE5C`
and successor `1856506DB2C84C6F41202A45F35CFA32E92420823A2FCA2410AF001707D8D0C7`.

The first cancellation invocation used `msiexec /x` without `/qf`; Windows selected basic UI and
completed an ordinary uninstall with data retained. That result establishes default retention,
not cancellation. The full-UI continuation separately established cancellation. Setup delegates
maintenance to the MSI's full interface; direct basic/silent MSI removal retains data unless the
explicit removal property is supplied.

Full UI exposed missing Chinese glyphs with the previous Segoe UI TextStyle. The installer now
uses Microsoft's Simplified Chinese UI font, Microsoft YaHei UI. The refreshed MSI digest is
`B1E6B72086D85F6FCC1B0245EBFA106B83D753F47774ACC53675B1AB00C1CFBF`.
Its Setup bundle installed successfully from the guest's local filesystem. Actual maintenance
displayed readable Chinese, the initially unchecked option and complete data-scope text; selected
removal returned zero and removed the disposable default marker. The native 99 cases, 42 rejecting
MSI policy cases and 16 checkbox transitions also passed for this build. A first launch directly
from the Sandbox mapped directory returned `0x8007139F` before producing a Burn log; the identical
binary ran successfully after copying into the guest. `readable-local.json`, `readable-Remove.json`
and the associated Burn/MSI logs retain these results. The owned guest was retired after verification.

## Hosted verification

[Quality run 45947420779](https://github.com/Cedarflake/Cedarflake-Ame/actions/runs/45947420779)
targets `eb54aa3a`. Its Remaining component rejected four stale production-factory entries and the
new preferences-directory query missing from the dependency inventory. The retained composition
policy now names the exact shared-directory and distribution-adapter factories under ADR 0045;
the immutable legacy inventory and dependency rules are unchanged. The focused boundary suite
passed all 26 cases, including rejecting parser fixtures. Final-head hosted verification remains
pending with the compatibility cleanup.

## Review boundaries

### Store publication tooling — 2026-11-27

The new Store workflow and local retrieval entrypoint passed PowerShell parsing and pinned
actionlint. Package fixtures passed exact identity/version/build admission and foreign executable
rejection. Controlled HTTP transport exercised the submission entrypoint: one upload and commit,
read-only resumption after commit, rejection of an unrelated draft without mutation, and rejection
of a mismatched submission ID. Initial fixture runs exposed PowerShell dynamic-scope collisions
with the scratch helper; fixture state now uses one named object, and the corrected cases passed.
The existing MSIX payload checks and CI selection fixtures also passed.

Production owners are separated into package/signature admission (144 lines), submission API and
payload preparation (119), artifact provenance/publication (86), submission entrypoint (61),
candidate preparation (27) and authenticated retrieval (132). The workflow is 106 lines and the
dedicated contract suite is 200 lines. No product runtime or existing MSI/portable publisher is
changed by this follow-up.

No Partner Center submission, WAM-authenticated download, Microsoft-signed Ame package installation
or Release attachment upload ran. These require the first published version and account setup;
the [Store procedure](quality/store-release.md) defines that remaining validation. Local fixtures
do not establish Store certification or offline-license compatibility.

The changed update controller contains 327 production lines, with no inline tests; the new Store
delivery suite contains 124 dedicated test lines. Distribution selection is a separate owner from
download orchestration. MSI cleanup is split by account identity (134 lines), filesystem cleanup
(127 lines) and MSI action lifecycle (93 lines), with 200 dedicated filesystem/policy test lines.
Package assembly contains 118 lines and icon/resource generation 57 lines. These counts describe
the reviewed handwritten owners, not additional acceptance gates.
