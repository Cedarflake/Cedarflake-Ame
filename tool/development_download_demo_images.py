# /// script
# requires-python = ">=3.12"
# dependencies = ["pillow==12.2.0"]
# ///
"""Download a provenance-tracked public-domain image set for desktop demonstrations."""

import argparse
import hashlib
import io
import json
import sys
import time
import urllib.error
import urllib.parse
import urllib.request
from datetime import UTC, datetime
from itertools import zip_longest
from pathlib import Path

from PIL import Image

API = "https://collectionapi.metmuseum.org/public/collection"
POLICY = "https://www.metmuseum.org/policies/frequently-asked-questions-image-and-data-resources"
LICENSE = "https://creativecommons.org/publicdomain/zero/1.0/"
DEFAULT_LIBRARY = Path(__file__).resolve().parents[1] / "build/test/demo-library"
TOPICS = (
    "landscape",
    "flowers",
    "garden",
    "sea",
    "mountain",
    "still life",
    "river",
    "village",
)


def fetch(url: str) -> bytes:
    request = urllib.request.Request(
        url, headers={"User-Agent": "Cedarflake-Ame-demo-images/1.0"}
    )
    for attempt in range(3):
        try:
            with urllib.request.urlopen(request, timeout=45) as response:
                data = response.read(32 * 1024 * 1024 + 1)
                if len(data) > 32 * 1024 * 1024:
                    raise ValueError("Image exceeds the 32 MiB download limit")
                return data
        except (urllib.error.URLError, TimeoutError):
            if attempt == 2:
                raise
            time.sleep(2 * (attempt + 1))
    raise RuntimeError("Unreachable download state")


def save_manifest(path: Path, manifest: dict) -> None:
    temporary = path.with_suffix(".tmp")
    temporary.write_text(
        json.dumps(manifest, ensure_ascii=False, indent=2) + "\n", encoding="utf-8"
    )
    temporary.replace(path)


def download(library: Path, count: int) -> None:
    manifest_path = library / "sources.json"
    manifest = {"kind": "ame-demo-library-v1", "policy_url": POLICY, "images": []}
    if manifest_path.exists():
        manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
        if manifest.get("kind") != "ame-demo-library-v1":
            raise ValueError("Destination is not an Ame demonstration library")
    elif library.exists() and any(library.iterdir()):
        raise ValueError(
            "Use an empty destination or an existing demonstration library"
        )
    images = library / "images"
    images.mkdir(parents=True, exist_ok=True)
    for item in manifest["images"]:
        file = library / item["file"]
        if not file.is_file():
            raise ValueError(f"Missing retained demonstration image: {item['file']}")
    save_manifest(manifest_path, manifest)
    if len(manifest["images"]) >= count:
        print(f"Complete: {len(manifest['images'])} retained images in {images}")
        return
    known_ids = {item["object_id"] for item in manifest["images"]}
    known_hashes = {item["download_sha256"] for item in manifest["images"]}
    candidates = []
    for topic in TOPICS:
        query = urllib.parse.urlencode(
            {"hasImages": "true", "departmentId": 11, "q": topic, "limit": 100}
        )
        response = json.loads(fetch(f"{API}/v1.1/search?{query}"))
        candidates.append(response.get("objectIDs") or [])
        time.sleep(1)
    visited = set(known_ids)
    for row in zip_longest(*candidates):
        for object_id in row:
            if len(manifest["images"]) >= count:
                print(
                    f"Complete: {len(manifest['images'])} images in {images}",
                    flush=True,
                )
                return
            if object_id is None or object_id in visited:
                continue
            visited.add(object_id)
            try:
                item = json.loads(fetch(f"{API}/v1/objects/{object_id}"))
                image_url = item.get("primaryImage", "")
                if item.get("isPublicDomain") is not True or not image_url:
                    continue
                if urllib.parse.urlparse(image_url).hostname != "images.metmuseum.org":
                    continue
                data = fetch(image_url)
                digest = hashlib.sha256(data).hexdigest()
                if digest in known_hashes:
                    continue
                with Image.open(io.BytesIO(data)) as image:
                    width, height = image.size
                    if (
                        image.format != "JPEG"
                        or max(width, height) < 2000
                        or min(width, height) < 1200
                    ):
                        continue
                    image.verify()
                relative = (
                    f"images/{len(manifest['images']) + 1:03}-met-{object_id}.jpg"
                )
                target = library / relative
                with target.open("xb") as output:
                    output.write(data)
                manifest["images"].append(
                    {
                        "file": relative,
                        "object_id": object_id,
                        "title": item["title"],
                        "artist": item.get("artistDisplayName", ""),
                        "source_url": item["objectURL"],
                        "image_url": image_url,
                        "public_domain": True,
                        "license": "CC0-1.0",
                        "license_url": LICENSE,
                        "width": width,
                        "height": height,
                        "download_sha256": digest,
                        "downloaded_at": datetime.now(UTC).isoformat(),
                    }
                )
                known_hashes.add(digest)
                save_manifest(manifest_path, manifest)
                print(
                    f"{len(manifest['images']):03}/{count}: {width}x{height} {item['title']}",
                    flush=True,
                )
            except (urllib.error.URLError, TimeoutError, ValueError, OSError) as error:
                print(f"Skipped Met {object_id}: {error}", flush=True)
            finally:
                time.sleep(1)
    raise RuntimeError(
        f"Only {len(manifest['images'])}/{count} eligible images were available; rerun to resume"
    )


def main() -> None:
    sys.stdout.reconfigure(encoding="utf-8")
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--library", type=Path, default=DEFAULT_LIBRARY)
    parser.add_argument("--count", type=int, default=100)
    args = parser.parse_args()
    if not 1 <= args.count <= 100:
        parser.error("--count must be between 1 and 100")
    download(args.library.resolve(), args.count)


if __name__ == "__main__":
    main()
