import io
import xml.etree.ElementTree as ET
from collections import Counter, defaultdict
from dataclasses import dataclass


@dataclass(frozen=True)
class Noun:
    ili: str | None
    lemmas: tuple[str, ...]
    definition: str
    domain: str
    parents: tuple[str, ...]
    wholes: tuple[str, ...]


def read_nouns(payload):
    senses = {}
    nouns = {}
    for _, element in ET.iterparse(io.BytesIO(payload), events=("end",)):
        if element.tag == "LexicalEntry":
            lemma = element.find("Lemma")
            if lemma is not None and lemma.get("partOfSpeech") == "n":
                for sense in element.findall("Sense"):
                    identity = sense.get("id")
                    if not identity or identity in senses:
                        raise ValueError(
                            "Vocabulary contains a duplicate or empty sense"
                        )
                    senses[identity] = lemma.get("writtenForm")
            element.clear()
        elif element.tag == "Synset":
            if element.get("partOfSpeech") == "n":
                identity = element.get("id")
                if not identity or identity in nouns:
                    raise ValueError("Vocabulary contains a duplicate or empty noun")
                members = element.get("members", "").split()
                lemmas = tuple(
                    dict.fromkeys(senses[key] for key in members if key in senses)
                )
                nouns[identity] = Noun(
                    element.get("ili"),
                    lemmas,
                    element.findtext("Definition", default=""),
                    element.get("lexfile", ""),
                    tuple(
                        relation.get("target")
                        for relation in element.findall("SynsetRelation")
                        if relation.get("relType") == "hypernym"
                    ),
                    tuple(
                        relation.get("target")
                        for relation in element.findall("SynsetRelation")
                        if relation.get("relType")
                        in ("holo_part", "holo_member", "holo_substance")
                    ),
                )
                if len(nouns) > 100_000:
                    raise ValueError("Vocabulary exceeds the noun bound")
            element.clear()
    return nouns


def compose_resource(english, chinese, contexts):
    translations = {
        noun.ili: noun for noun in chinese.values() if noun.ili and noun.lemmas
    }
    translated = {
        identity: translations[noun.ili]
        for identity, noun in english.items()
        if noun.ili in translations and noun.lemmas
    }
    children = defaultdict(list)
    for identity, noun in english.items():
        for parent in noun.parents:
            if parent in english:
                children[parent].append(identity)
    translated_children = Counter(
        parent for identity in translated for parent in english[identity].parents
    )
    entries = set()
    concepts = []
    context_links = {context["id"]: [] for context in contexts}
    for identity, noun in sorted(english.items()):
        if not noun.lemmas:
            raise ValueError("Vocabulary noun lacks a canonical lemma")
        translation = translated.get(identity)
        if translation and (
            translated_children[identity] >= 4 or noun.domain == "noun.communication"
        ):
            entries.add(identity)
        if noun.domain == "noun.act" and (translation or len(children[identity]) >= 4):
            entries.add(identity)
        if noun.domain == "noun.location" and len(children[identity]) >= 4:
            entries.add(identity)
        parents = sorted(set(parent for parent in noun.parents if parent in english))
        related = []
        if noun.domain != "noun.Tops":
            related = sorted(set(children[identity]) | set(parents))
        headword = noun.lemmas[0]
        label = translation.lemmas[0] if translation else headword
        if translation:
            for candidate in translation.lemmas:
                if 2 <= len(candidate) <= 80 and any(
                    "\u4e00" <= char <= "\u9fff" for char in candidate
                ):
                    label = candidate
                    break
        concepts.append(
            {
                "id": identity,
                "definition": {
                    "label": label[:80],
                    "description": f"An image of {headword}. {noun.definition}.",
                    "subject": headword.replace("_", " "),
                },
                "related": related,
                "broader": sorted(
                    set(parents) | {whole for whole in noun.wholes if whole in english}
                ),
            }
        )
        lemmas = {lemma.replace("_", " ").lower() for lemma in noun.lemmas}
        for context in contexts:
            if lemmas.intersection(context["cues"]):
                context_links[context["id"]].append(identity)
    if len(entries) > 4096:
        raise ValueError("Vocabulary exceeds the entry-point bound")
    naming = [
        {
            "id": "topic-" + context["id"],
            "definition": {
                "label": context["label"],
                "description": "An image of " + context["subject"] + ".",
                "subject": context["subject"],
            },
            "cues": context_links[context["id"]],
            "broader": [],
        }
        for context in contexts
    ]
    return {
        "format_version": 1,
        "source_namespace": "omw-en-nouns",
        "entries": sorted(entries),
        "concepts": concepts,
        "contexts": naming,
    }
