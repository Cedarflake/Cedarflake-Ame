# Ame marks

`ame.svg` and `ame_light.svg` are the supplied Revaea artwork.
The colored mark supplies the Windows application and installer icon. The title bar keeps the outer
silhouette, with a two-pixel theme-colored contour and an inset tonal fill separated by a 1.5-pixel
transparent gap. The original central cutout remains without an inner stroke. Its 28-pixel size
and horizontal center match the primary navigation rail through shared workbench metrics.

Regenerate the checked-in PNG density variants and multi-resolution ICO with:

```powershell
uv run tool/branding_generate_icons.py
```

Flutter loads the 28-pixel fill and outline PNGs and their 2x, 3x and 4x variants through the built-in
asset bundle. Their shared padding keeps the contour inside the icon bounds. The fill uses a vector
mask to keep its gap from the contour uniform around the curved silhouette.
No SVG renderer ships with the application. The generation tool uses
[resvg-py](https://github.com/baseplate-admin/resvg-py) (MIT) and
[Pillow](https://python-pillow.org/) (MIT-CMU) only during asset preparation.
