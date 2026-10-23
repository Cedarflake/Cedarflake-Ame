# /// script
# requires-python = ">=3.11"
# dependencies = ["resvg-py==0.5.0", "pillow==13.0.0"]
# ///

from io import BytesIO
from pathlib import Path
import xml.etree.ElementTree as ET

import resvg_py
from PIL import Image


REPOSITORY = Path(__file__).resolve().parent.parent
BRANDING = REPOSITORY / "assets" / "branding"
ICON_SIZES = (16, 20, 24, 32, 40, 48, 64, 128, 256)
TITLE_BAR_SIZE = 28
TITLE_BAR_VIEW_BOX = "-50 -50 2100 2100"
# SVG units give a two-pixel contour and a 1.5-pixel transparent gap at 28 pixels.
TITLE_BAR_STROKE_WIDTH = 150
TITLE_BAR_FILL_GAP = 112.5


def render(source: Path, size: int) -> bytes:
    return resvg_py.svg_to_bytes(
        svg_path=str(source),
        width=size,
        height=size,
        skip_system_fonts=True,
    )


def build_title_bar_layers() -> dict[str, ET.Element]:
    source = ET.parse(BRANDING / "ame_light.svg").getroot()
    shape = source.find(
        ".//{http://www.w3.org/2000/svg}mask/{http://www.w3.org/2000/svg}path"
    )
    if shape is None:
        raise ValueError("The supplied mark must contain its shape mask.")

    # The first closed subpath is the silhouette; the second cuts out its center.
    outer_path, terminator, _ = shape.attrib["d"].partition("Z")
    if not terminator:
        raise ValueError("The supplied mark must contain a closed outer contour.")
    outer_path += terminator
    attributes = {
        "xmlns": "http://www.w3.org/2000/svg",
        "viewBox": TITLE_BAR_VIEW_BOX,
    }
    outline = ET.Element("svg", attributes)
    ET.SubElement(
        outline,
        "path",
        {
            "d": outer_path,
            "fill": "none",
            "stroke": "white",
            "stroke-width": str(TITLE_BAR_STROKE_WIDTH),
            "stroke-linejoin": "round",
        },
    )

    fill = ET.Element("svg", attributes)
    mask = ET.SubElement(
        ET.SubElement(fill, "defs"),
        "mask",
        {
            "id": "inset-fill",
            "maskUnits": "userSpaceOnUse",
            "x": "-50",
            "y": "-50",
            "width": "2100",
            "height": "2100",
        },
    )
    # Masking the edge keeps the gap uniform around the curved silhouette.
    ET.SubElement(
        mask,
        "path",
        {
            "d": outer_path,
            "fill": "white",
            "stroke": "black",
            "stroke-width": str(TITLE_BAR_STROKE_WIDTH + 2 * TITLE_BAR_FILL_GAP),
            "stroke-linejoin": "round",
        },
    )
    ET.SubElement(
        fill,
        "path",
        {
            "d": shape.attrib["d"],
            "fill": "white",
            "fill-rule": "evenodd",
            "mask": "url(#inset-fill)",
        },
    )
    return {"ame_light": fill, "ame_outline": outline}


def main() -> None:
    layers = build_title_bar_layers()
    for density in (1, 2, 3, 4):
        destination = BRANDING if density == 1 else BRANDING / f"{density}.0x"
        destination.mkdir(exist_ok=True)
        for name, layer in layers.items():
            (destination / f"{name}.png").write_bytes(
                resvg_py.svg_to_bytes(
                    svg_string=ET.tostring(layer, encoding="unicode"),
                    width=TITLE_BAR_SIZE * density,
                    height=TITLE_BAR_SIZE * density,
                    skip_system_fonts=True,
                )
            )

    frames = [
        Image.open(BytesIO(render(BRANDING / "ame.svg", size))).convert("RGBA")
        for size in ICON_SIZES
    ]
    icon = REPOSITORY / "windows" / "runner" / "resources" / "app_icon.ico"
    frames[-1].save(
        icon,
        format="ICO",
        sizes=[(size, size) for size in ICON_SIZES],
        append_images=frames[:-1],
    )
    for frame in frames:
        frame.close()
    print("Generated title-bar PNG variants and the Windows application icon.")


if __name__ == "__main__":
    main()
