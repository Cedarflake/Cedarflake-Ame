# Cedarflake Ame Privacy Statement

Last updated: 2026-11-28

This statement describes how Cedarflake Ame ("Ame"), published by Cedarflake and maintained by
the Cedarflake Ame contributors, handles information. It covers the Microsoft Store, MSI and
portable editions. Ame is a local image library application and does not require an Ame account.

## Information processed on your device

Ame reads images and folders you open or add to the library. Images, filenames, paths and embedded
metadata may contain personal information. To browse, search and organize your library, Ame
processes image contents, file locations, sizes and timestamps, image dimensions, and supported
metadata such as capture time and camera make/model.

Ame stores its catalog, thumbnails and previews, image fingerprints and analysis features,
classification results, favorites, albums, organization decisions, file-operation history,
preferences and task records on your device. Search input is processed locally; search features
and search/filter state used to resume organization sessions may also be saved locally.

Duplicate detection, similar-image comparison, smart-album classification and semantic search run
on your device. Ame does not upload your images, image metadata, search input or library records
to the project maintainers or a remote AI service, and does not use them to train an online model.
Ame does not include advertising or usage-analytics collection.

## Network connections and third parties

Ame uses HTTPS for its own update and model downloads:

- The Microsoft Store edition uses Microsoft Store for application updates. Windows and the Store
  manage those downloads; this edition does not use Ame's direct GitHub updater.
- MSI and portable editions contact GitHub to check releases and download updates, according to
  the update settings you choose. You can select manual checks instead of automatic checking or
  downloading.
- When model installation or recovery requires a download, Ame retrieves model files from
  Hugging Face and its download infrastructure. Image analysis then runs locally.

These services receive normal connection and request information, such as your IP address,
request time, requested release/model resource and application request headers. These requests
do not include your images, local file paths, library records or search input. The services handle
their information under their own privacy policies:
[Microsoft](https://www.microsoft.com/en-us/privacy/privacystatement),
[GitHub](https://docs.github.com/en/site-policy/privacy-policies/github-general-privacy-statement)
and [Hugging Face](https://huggingface.co/privacy).

If you place images, application data or exports in a OneDrive or other synchronized folder,
the corresponding provider may synchronize them under its own settings and privacy policy.
Opening external links also uses your browser or the relevant external application.

## Diagnostics and information you choose to share

Ame keeps a bounded local diagnostic event history to help investigate failures. Exporting
diagnostics saves a report to a location you choose; it does not upload or send that report.
Reports contain application and Windows versions, runtime information, CPU and graphics models,
memory capacity, graphics driver versions, display scaling, aggregate counts, task states and
structured error codes. They exclude image contents, catalog records, personal file paths,
file/folder/album names, credentials, computer names and hardware serial numbers.

If you choose to send a report or contact the maintainers, they receive the information you submit
and use it to answer your request or investigate the problem. GitHub processes issue submissions
under its own policy. Public issues and attachments are visible to others; do not include private
images or information you do not want to publish.

## Storage, security and retention

Application data is stored in your Windows user profile by default. Supported catalog and preview
locations can be changed in Settings. Ame does not apply its own encryption to local catalogs,
caches or exported files; their protection depends on Windows account permissions and any device
or storage encryption you enable. Local task and operation records may contain file paths.

Library and organization records remain until you remove the relevant data. Rebuildable caches
may be cleared or replaced, and the bounded diagnostic history replaces older events. Exports
are retained at the location you choose. Normal application updates retain local data.

The Microsoft Store, MSI and portable editions share the same default data directory for a Windows
user. Uninstalling the Microsoft Store/MSIX edition removes that shared default application data,
including data used by another edition. MSI uninstall removes default application data only if
you select its data-removal option, which is unchecked by default. Deleting portable program files
leaves the application data in place. Custom data directories and exported files located outside
the shared default directory are not removed by these uninstall operations.

## Your choices and control

You choose which images and folders Ame opens or catalogs. You can remove folders from the library,
change organization records, configure direct-update checks where available, and choose whether
to export or share diagnostics. To remove retained local data, close Ame and remove its data from
the storage locations shown in Settings, including any custom
locations or exports you want to delete. Clearing a cache does not remove favorites or albums.

Browsing, cataloging and analysis preserve original images. Moving, copying, renaming, recycling
and deleting original files are separate actions you initiate. Uninstall cleanup does not delete
your original image folders. The maintainers cannot remotely access, correct or delete a library
that remains on your device.

## Changes and contact

We publish revisions of this statement in the
[project repository](https://github.com/Cedarflake/Cedarflake-Ame) with an updated date.
For privacy questions or requests about information you have shared with the project, contact
the maintainers through the [issue tracker](https://github.com/Cedarflake/Cedarflake-Ame/issues)
without including sensitive details in a public issue. Security vulnerabilities can be reported
privately through the process in [SECURITY.md](SECURITY.md).
