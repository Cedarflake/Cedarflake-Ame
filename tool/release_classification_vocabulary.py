import json
import os
from pathlib import Path

from release_vocabulary_artifacts import (
    download_source,
    publish_bundle,
    read_archive,
    validate_local_path,
    verify_bundle,
    verify_file,
)
from release_vocabulary_lexicon import compose_resource, read_nouns


MAX_RESOURCE_BYTES = 64 * 1024 * 1024


def prepare_vocabulary(
    source_directory, destination, manifest, context_file, notice_file
):
    validate_local_path(destination)
    if destination.exists():
        raise ValueError("Vocabulary preparation requires a new directory")
    verify_file(context_file, manifest["contexts"])
    context_bytes = context_file.read_bytes()
    contexts = json.loads(context_bytes)
    notice = next(entry for entry in manifest["files"] if entry["name"] == "NOTICE.txt")
    verify_file(notice_file, notice)
    english = read_archive(
        source_directory / manifest["sources"][0]["file"], manifest["sources"][0]
    )
    chinese = read_archive(
        source_directory / manifest["sources"][1]["file"], manifest["sources"][1]
    )
    resource = compose_resource(
        read_nouns(english.pop("xml")), read_nouns(chinese.pop("xml")), contexts
    )
    encoded = json.dumps(resource, ensure_ascii=False, separators=(",", ":")).encode(
        "utf-8"
    )
    if len(encoded) > MAX_RESOURCE_BYTES:
        raise ValueError("Vocabulary exceeds the resource byte bound")
    payloads = {
        "vocabulary.json": encoded,
        "WORDNET-LICENSE": english["license"],
        "WORDNET-citation.bib": english["citation"],
        "COW-LICENSE": chinese["license"],
        "COW-citation.bib": chinese["citation"],
        "NOTICE.txt": notice_file.read_bytes(),
    }
    publish_bundle(destination, payloads, manifest)


if __name__ == "__main__":
    import argparse

    parser = argparse.ArgumentParser()
    parser.add_argument("--source-directory", type=Path)
    parser.add_argument("--destination", type=Path)
    parser.add_argument("--verify-bundle", type=Path)
    parser.add_argument("--download", action="store_true")
    arguments = parser.parse_args()
    resources = Path(__file__).resolve().parent.parent / "resources/classification"
    manifest = json.loads(
        (resources / "vocabulary-distribution.json").read_text(encoding="utf-8")
    )
    if arguments.verify_bundle:
        verify_bundle(arguments.verify_bundle, manifest)
        print("Verified classification vocabulary")
    else:
        if not arguments.destination or not arguments.source_directory:
            parser.error("Preparation requires source and destination directories")
        build = resources.parent.parent / "build"
        destination = Path(os.path.abspath(arguments.destination))
        if not destination.is_relative_to(build) or destination == build:
            parser.error("Vocabulary output must remain below repository build storage")
        if arguments.download:
            source_directory = Path(os.path.abspath(arguments.source_directory))
            if not source_directory.is_relative_to(build) or source_directory == build:
                parser.error(
                    "Vocabulary downloads must remain below repository build storage"
                )
            for source in manifest["sources"]:
                download_source(source_directory, source)
        prepare_vocabulary(
            arguments.source_directory,
            destination,
            manifest,
            resources / "naming-contexts.json",
            resources / "NOTICE.txt",
        )
        print("Prepared verified classification vocabulary")
