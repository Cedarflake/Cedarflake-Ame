# Demonstration images

The reusable screenshot collection lives in `build/test/demo-library/images`.
`build/test/sources/mixed` is the older synthetic test corpus. Images and local manifests remain
outside Git; do not clean up the demonstration collection while it is registered in Ame.

## Download and import

From the repository root, run:

```powershell
uv run tool/development_download_demo_images.py
```

The downloader retains up to 100 distinct JPEGs from The Metropolitan Museum of Art. Every object
must have `isPublicDomain: true`; images must have a long edge of at least 2000 pixels and a short
edge of at least 1200 pixels. Downloads are sequential and resumable. `sources.json` records the
title, artist, object page, image URL, CC0 designation, dimensions and original SHA-256 for each file.
See the museum's [Open Access policy](https://www.metmuseum.org/policies/frequently-asked-questions-image-and-data-resources)
and [Collection API](https://metmuseum.github.io/).

In the regular Ame client, use **Add folder to library** and select
`build/test/demo-library/images`. Select that root in the sidebar for screenshots containing only
the demonstration collection. This adds the images to the existing catalog.

## Arrange dates

The Windows script changes EXIF capture time, filesystem creation time and filesystem modification
time. It operates only on JPEGs listed in this collection's manifest. JPEG image data is not
re-encoded. The museum's source manifest remains separate from these fictional demonstration dates.

```powershell
# Five images per date, distributed from 2021 through October 2026.
uv run tool/development_set_demo_dates.py

# Preview a different arrangement without writing files.
uv run tool/development_set_demo_dates.py --start 2023-01-01T12:00:00 --end 2026-10-31T12:00:00 --per-day 10 --preview

# Apply that arrangement; creation is one day after capture, modification two days after capture.
uv run tool/development_set_demo_dates.py --start 2023-01-01T12:00:00 --end 2026-10-31T12:00:00 --per-day 10

# Set all three times independently for one image.
uv run tool/development_set_demo_dates.py --file 001-met-438031.jpg --start 2024-06-01T12:00:00 --end 2024-06-01T12:00:00 --created-at 2024-06-02T14:00:00 --modified-at 2025-01-15T09:00:00
```

Use `--created-days` and `--modified-days` to change the relative offsets, or `--created-at` and
`--modified-at` for fixed timestamps. Dates without an offset use the local Windows time zone;
an explicit offset such as `2024-06-01T12:00:00+08:00` is also accepted. `dates.json` records the
latest applied batch. Ame's folder synchronization picks up changes to already imported samples;
capture metadata can appear after the initial filesystem update.

Both tools accept `--library <directory>` for a separate demonstration collection. Point this to
the directory containing `sources.json`, not its `images` subdirectory.
