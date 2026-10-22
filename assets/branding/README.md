# Ame marks

`ame.svg` and `ame_light.svg` are the supplied Revaea artwork.
The colored mark supplies the Windows application and installer icon. The title bar uses a tonal
fill and a two-pixel theme-colored contour derived from the supplied vector path. Its 28-pixel size
and horizontal center match the primary navigation rail through shared workbench metrics.

Regenerate the checked-in PNG density variants and multi-resolution ICO with:

```powershell
uv run tool/branding_generate_icons.py
```

Flutter loads the two 28-pixel PNG layers and their 2x, 3x and 4x variants through the built-in asset
bundle. Both layers include the same padding so the contour stays inside the icon bounds.
No SVG renderer ships with the application. The generation tool uses
[resvg-py](https://github.com/baseplate-admin/resvg-py) (MIT) and
[Pillow](https://python-pillow.org/) (MIT-CMU) only during asset preparation.
