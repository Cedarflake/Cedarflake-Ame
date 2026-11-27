# Using Ame

[简体中文](user-guide.zh-Hans.md) · [日本語](user-guide.ja.md)

## Start with your folders

In All photos, use the add button beside Folders and choose an existing image folder. Images appear
as discovery progresses. Open a folder menu to update it or change its synchronization settings.
If a drive or cloud folder is unavailable, reconnect it before updating. Cloud-only images need to
be available locally before analysis; Ame does not download originals automatically.

Smart albums and natural-language search need the image-analysis models. Follow the model-preparation
prompt; a failed download can be resumed from its feedback. Classification continues in the background.
Settings → Smart albums controls GPU acceleration. Ordinary browsing remains available during analysis.

## Installed and portable versions

The MSI includes models and a background library-update service. Installation and MSI updates request
administrator permission. The portable ZIP runs after extraction, downloads models when needed and
does not install a Windows service. Its data locations are shown in Settings → Storage; keeping the
program in a portable folder does not make every data location part of that folder.

Settings → About checks for updates and offers Download, then Restart and update. Update settings
choose automatic behavior and the release or preview channel. The preview channel includes Alpha,
Beta and RC releases. If the update fails, use the action in its feedback or download a package from
the [release page](https://github.com/Cedarflake/Cedarflake-Ame/releases). For manual updates, close Ame,
run the newer MSI or extract the entire portable ZIP into a new folder and launch it.

The Microsoft Store version uses Store-managed updates. Settings → About opens its Store page.
MSI, portable and Store versions share their default library data. Removing the Store version clears
those shared data; MSI removal offers an initially unchecked data-removal option. Deleting portable
program files leaves the data in place. Catalog and cache folders selected outside the default
location remain, as do original images. Choose custom locations in Settings → Storage.

Earlier versions' data folders remain untouched and are not loaded automatically. This Alpha
version starts directly with the common data location; normal future updates retain those shared
data and your configured custom locations.

## Back up and recover organization

Settings → Organization records → Export saves favorites, albums and review decisions. Keep that file
outside the image and application-data folders. Restore organization records lets you inspect and
confirm restoration. This backup contains organization records; keep a separate backup of original
images. Storage settings show where the catalog and thumbnails are stored.

If a copy or move is interrupted, open File operation records and inspect the operation before
continuing it. Use its available recovery action. Deleted files normally go to the Windows Recycle
Bin, where Windows can restore them. Permanent deletion has no Recycle Bin recovery.

## Share diagnostics

Open Settings → About → Diagnostics → Export and choose where to save the ZIP. The completion
message can open its folder. If Ame cannot start, use Export diagnostics on the failure page; Copy
diagnostics copies a shorter safe summary. If saving fails, choose another location or use Retry
when offered.

The ZIP contains a readable summary and structured runtime information: application/system/Flutter
versions, CPU and memory, graphics adapters and drivers, display scaling, available task/model state
and recent error codes. It excludes images, catalog records, credentials
and personal paths. It remains on your computer until you share it. Attach it to a
[problem report](https://github.com/Cedarflake/Cedarflake-Ame/issues), together with the steps that
triggered the issue and the approximate time. Check any screenshots separately for personal content.
