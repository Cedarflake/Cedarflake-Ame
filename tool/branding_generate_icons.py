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


def render(source: Path, size: int) -> bytes:
    return resvg_py.svg_to_bytes(
        svg_path=str(source),
        width=size,
        height=size,
        skip_system_fonts=True,
    )


def main() -> None:
    fill = ET.parse(BRANDING / "ame_light.svg").getroot()
    fill.set("viewBox", TITLE_BAR_VIEW_BOX)
    shape = fill.find(
        ".//{http://www.w3.org/2000/svg}mask/{http://www.w3.org/2000/svg}path"
    )
    if shape is None:
        raise ValueError("The supplied mark must contain its shape mask.")
    outline = ET.Element(
        "svg",
        {
            "xmlns": "http://www.w3.org/2000/svg",
            "viewBox": TITLE_BAR_VIEW_BOX,
        },
    )
    # At 28 pixels, 150 units in the padded view box produce a two-pixel contour.
    ET.SubElement(
        outline,
        "path",
        {
            "d": shape.attrib["d"],
            "fill": "none",
            "stroke": "white",
            "stroke-width": "150",
            "stroke-linejoin": "round",
        },
    )
    for density in (1, 2, 3, 4):
        destination = BRANDING if density == 1 else BRANDING / f"{density}.0x"
        destination.mkdir(exist_ok=True)
        for name, layer in (("ame_light", fill), ("ame_outline", outline)):
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
