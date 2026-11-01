import hashlib
import os
import tarfile
import time
import urllib.request
from urllib.parse import urlsplit
import uuid


MAX_XML_BYTES = 128 * 1024 * 1024


def verify_download_url(url):
    parsed = urlsplit(url)
    host = parsed.hostname or ""
    if (
        parsed.scheme != "https"
        or parsed.port not in (None, 443)
        or parsed.username
        or parsed.password
        or not (host == "github.com" or host.endswith(".githubusercontent.com"))
    ):
        raise ValueError(
            "Vocabulary downloads require the admitted HTTPS hosting domains"
        )


class VocabularyRedirects(urllib.request.HTTPRedirectHandler):
    max_redirections = 5

    def redirect_request(self, request, response, code, message, headers, new_url):
        verify_download_url(new_url)
        return super().redirect_request(
            request, response, code, message, headers, new_url
        )


def validate_local_path(path):
    for ancestor in (path, *path.parents):
        if not ancestor.exists() and not ancestor.is_symlink():
            continue
        metadata = ancestor.lstat()
        unavailable = 0x400 | 0x1000 | 0x40000 | 0x400000
        if (
            ancestor.is_symlink()
            or getattr(metadata, "st_file_attributes", 0) & unavailable
        ):
            raise ValueError(
                "Vocabulary paths cannot traverse links or cloud placeholders"
            )


def verify_bytes(payload, expected):
    if len(payload) != expected["length"]:
        raise ValueError("Vocabulary artifact length differs from its manifest")
    if hashlib.sha256(payload).hexdigest() != expected["sha256"]:
        raise ValueError("Vocabulary artifact digest differs from its manifest")


def verify_file(path, expected):
    validate_local_path(path)
    if not path.is_file() or path.stat().st_size != expected["length"]:
        raise ValueError("Vocabulary artifact length differs from its manifest")
    with path.open("rb") as source:
        if hashlib.file_digest(source, "sha256").hexdigest() != expected["sha256"]:
            raise ValueError("Vocabulary artifact digest differs from its manifest")


def verify_bundle(directory, manifest):
    validate_local_path(directory)
    expected_names = {entry["name"] for entry in manifest["files"]}
    if {entry.name for entry in directory.iterdir()} != expected_names:
        raise ValueError(
            "Vocabulary directory must contain exactly the admitted payload"
        )
    for expected in manifest["files"]:
        verify_file(directory / expected["name"], expected)


def download_source(directory, expected):
    validate_local_path(directory)
    directory.mkdir(parents=True, exist_ok=True)
    destination = directory / expected["file"]
    if destination.exists():
        verify_file(destination, expected)
        return
    temporary = directory / (uuid.uuid4().hex + ".partial")
    started = time.monotonic()
    verify_download_url(expected["url"])
    request = urllib.request.Request(
        expected["url"], headers={"User-Agent": "Cedarflake-Ame-build/1.0"}
    )
    try:
        opener = urllib.request.build_opener(VocabularyRedirects())
        with opener.open(request, timeout=10) as source, temporary.open("xb") as target:
            total = 0
            while chunk := source.read(65536):
                total += len(chunk)
                if total > expected["length"] or time.monotonic() - started > 300:
                    raise ValueError(
                        "Vocabulary download exceeded its size or time bound"
                    )
                target.write(chunk)
        verify_file(temporary, expected)
        try:
            os.link(temporary, destination)
        except FileExistsError:
            verify_file(destination, expected)
    finally:
        temporary.unlink(missing_ok=True)


def publish_bundle(destination, payloads, manifest):
    validate_local_path(destination)
    if destination.exists():
        raise ValueError("Vocabulary preparation requires a new directory")
    if set(payloads) != {entry["name"] for entry in manifest["files"]}:
        raise ValueError("Vocabulary payload differs from its manifest")
    for entry in manifest["files"]:
        verify_bytes(payloads[entry["name"]], entry)
    destination.parent.mkdir(parents=True, exist_ok=True)
    scratch = destination.parent / (".classification-vocabulary-" + uuid.uuid4().hex)
    scratch.mkdir()
    try:
        for name, payload in payloads.items():
            with (scratch / name).open("xb") as target:
                target.write(payload)
        verify_bundle(scratch, manifest)
        scratch.rename(destination)
    finally:
        if scratch.exists():
            for name in payloads:
                path = scratch / name
                validate_local_path(path)
                path.unlink(missing_ok=True)
            scratch.rmdir()


def read_archive(path, expected):
    validate_local_path(path)
    if path.stat().st_size != expected["length"]:
        raise ValueError("Vocabulary archive length differs from its manifest")
    with path.open("rb") as source:
        digest = hashlib.file_digest(source, "sha256").hexdigest()
        if digest != expected["sha256"]:
            raise ValueError("Vocabulary archive digest differs from its manifest")
        source.seek(0)
        with tarfile.open(fileobj=source, mode="r:xz") as archive:
            members = archive.getmembers()
            if len(members) > 16:
                raise ValueError("Vocabulary archive has too many members")
            payloads = {}
            for wanted in expected["members"]:
                matches = [entry for entry in members if entry.name == wanted["name"]]
                if len(matches) != 1 or not matches[0].isfile():
                    raise ValueError(
                        "Vocabulary archive member is missing or not a regular file"
                    )
                entry = matches[0]
                if entry.size != wanted["length"] or entry.size > MAX_XML_BYTES:
                    raise ValueError(
                        "Vocabulary archive member exceeds its admitted size"
                    )
                with archive.extractfile(entry) as stream:
                    payload = stream.read(entry.size + 1)
                verify_bytes(payload, wanted)
                payloads[wanted["role"]] = payload
    return payloads
