import hashlib
import io
import json
import tarfile
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

from release_vocabulary_artifacts import (
    download_source,
    publish_bundle,
    read_archive,
    validate_local_path,
    verify_bundle,
    verify_download_url,
)
from release_vocabulary_lexicon import compose_resource, read_nouns


def identity(payload):
    return {"length": len(payload), "sha256": hashlib.sha256(payload).hexdigest()}


class VocabularyDistributionTests(unittest.TestCase):
    def setUp(self):
        storage = (
            Path(__file__).resolve().parent.parent
            / "build/classification-vocabulary-fixtures"
        )
        validate_local_path(storage)
        storage.mkdir(parents=True, exist_ok=True)
        self.temporary = tempfile.TemporaryDirectory(dir=storage)
        self.root = Path(self.temporary.name)
        self.addCleanup(self.temporary.cleanup)

    def test_lexicon_keeps_sense_order_wholes_and_useful_chinese_names(self):
        english = read_nouns(b"""<Lexicon>
          <LexicalEntry><Lemma partOfSpeech="n" writtenForm="dog"/>
            <Sense id="dog-sense"/></LexicalEntry>
          <LexicalEntry><Lemma partOfSpeech="n" writtenForm="animal"/>
            <Sense id="animal-sense"/></LexicalEntry>
          <LexicalEntry><Lemma partOfSpeech="n" writtenForm="pack"/>
            <Sense id="pack-sense"/></LexicalEntry>
          <Synset id="animal" partOfSpeech="n" members="animal-sense" lexfile="noun.Tops"/>
          <Synset id="pack" partOfSpeech="n" members="pack-sense"/>
          <Synset id="dog" ili="i-dog" partOfSpeech="n" members="dog-sense" lexfile="noun.communication">
            <Definition>a domestic animal</Definition>
            <SynsetRelation relType="hypernym" target="animal"/>
            <SynsetRelation relType="holo_member" target="pack"/>
          </Synset></Lexicon>""")
        chinese = read_nouns(
            """<Lexicon>
          <LexicalEntry><Lemma partOfSpeech="n" writtenForm="狗"/>
            <Sense id="short"/></LexicalEntry>
          <LexicalEntry><Lemma partOfSpeech="n" writtenForm="狗狗"/>
            <Sense id="long"/></LexicalEntry>
          <Synset id="chinese-dog" ili="i-dog" partOfSpeech="n" members="short long"/>
          </Lexicon>""".encode()
        )
        result = compose_resource(
            english,
            chinese,
            [{"id": "dogs", "label": "狗狗", "subject": "dogs", "cues": ["dog"]}],
        )
        dog = next(row for row in result["concepts"] if row["id"] == "dog")
        self.assertEqual(dog["definition"]["label"], "狗狗")
        self.assertEqual(
            dog["definition"]["description"], "An image of dog. a domestic animal."
        )
        self.assertEqual(dog["related"], ["animal"])
        self.assertEqual(dog["broader"], ["animal", "pack"])
        self.assertEqual(result["entries"], ["dog"])
        self.assertEqual(result["contexts"][0]["cues"], ["dog"])
        duplicate = b'<Lexicon><Synset id="same" partOfSpeech="n"/><Synset id="same" partOfSpeech="n"/></Lexicon>'
        with self.assertRaisesRegex(ValueError, "duplicate"):
            read_nouns(duplicate)

    def archive_fixture(self, mode="regular"):
        path = self.root / "source.tar.xz"
        payload = b"<Lexicon/>"
        with tarfile.open(path, "w:xz") as archive:
            entry = tarfile.TarInfo("lexicon/data.xml")
            entry.size = len(payload)
            if mode == "link":
                entry.type = tarfile.SYMTYPE
                entry.linkname = "../outside.xml"
            archive.addfile(entry, io.BytesIO(payload))
            if mode == "duplicate":
                archive.addfile(entry, io.BytesIO(payload))
        expected = {
            **identity(path.read_bytes()),
            "members": [{"name": entry.name, "role": "xml", **identity(payload)}],
        }
        return path, expected

    def test_archive_requires_digest_unique_regular_members_and_exact_sizes(self):
        path, expected = self.archive_fixture()
        self.assertEqual(read_archive(path, expected)["xml"], b"<Lexicon/>")
        with self.assertRaisesRegex(ValueError, "digest"):
            read_archive(path, {**expected, "sha256": "0" * 64})
        expected["members"][0]["length"] += 1
        with self.assertRaisesRegex(ValueError, "size"):
            read_archive(path, expected)
        for mode in ("link", "duplicate"):
            path, expected = self.archive_fixture(mode)
            with self.assertRaisesRegex(ValueError, "regular file"):
                read_archive(path, expected)

    def test_bundle_publication_is_atomic_and_preserves_existing_output(self):
        payloads = {"vocabulary.json": b'{"format_version":1}', "LICENSE": b"terms"}
        manifest = {
            "files": [
                {"name": name, **identity(data)} for name, data in payloads.items()
            ]
        }
        destination = self.root / "bundle"
        with patch.object(Path, "rename", side_effect=OSError("publication rejected")):
            with self.assertRaises(OSError):
                publish_bundle(destination, payloads, manifest)
        self.assertEqual(list(self.root.iterdir()), [])
        publish_bundle(destination, payloads, manifest)
        verify_bundle(destination, manifest)
        with self.assertRaisesRegex(ValueError, "new directory"):
            publish_bundle(destination, payloads, manifest)
        self.assertEqual((destination / "LICENSE").read_bytes(), b"terms")
        (destination / "LICENSE").write_bytes(b"wrong")
        with self.assertRaisesRegex(ValueError, "digest"):
            verify_bundle(destination, manifest)
        (destination / "extra").write_bytes(b"x")
        with self.assertRaisesRegex(ValueError, "exactly"):
            verify_bundle(destination, manifest)

    def test_download_is_bounded_reusable_and_removes_partial_files(self):
        expected = {
            "file": "source.tar.xz",
            "url": "https://github.com/example/resource",
            **identity(b"good"),
        }
        with patch(
            "release_vocabulary_artifacts.urllib.request.build_opener"
        ) as opener:
            opener.return_value.open.return_value = io.BytesIO(b"too much")
            with self.assertRaisesRegex(ValueError, "size or time"):
                download_source(self.root, expected)
            self.assertEqual(list(self.root.iterdir()), [])
            opener.return_value.open.return_value = io.BytesIO(b"good")
            download_source(self.root, expected)
            self.assertEqual((self.root / expected["file"]).read_bytes(), b"good")
            opener.reset_mock()
            download_source(self.root, expected)
            opener.assert_not_called()
        for url in (
            "http://github.com/file",
            "https://github.com.evil.test/file",
            "https://github.com:444/file",
            "https://user@github.com/file",
        ):
            with self.assertRaisesRegex(ValueError, "HTTPS"):
                verify_download_url(url)

    def test_manifest_pins_the_retained_resource_and_notice_payload(self):
        resources = Path(__file__).resolve().parent.parent / "resources/classification"
        manifest = json.loads(
            (resources / "vocabulary-distribution.json").read_text(encoding="utf-8")
        )
        resource = next(
            row for row in manifest["files"] if row["name"] == "vocabulary.json"
        )
        self.assertEqual(resource["length"], 23249187)
        self.assertEqual(
            resource["sha256"],
            "2cea3ceca629faac3d51271c2928871548ddc709cb769cc50f45a771ae68835c",
        )
        self.assertEqual(len(bytes.fromhex(resource["blake3"])), 32)
        self.assertEqual(
            manifest["contexts"],
            identity((resources / "naming-contexts.json").read_bytes()),
        )
        notice = next(row for row in manifest["files"] if row["name"] == "NOTICE.txt")
        self.assertEqual(
            identity((resources / "NOTICE.txt").read_bytes()),
            {key: notice[key] for key in ("length", "sha256")},
        )


if __name__ == "__main__":
    unittest.main()
