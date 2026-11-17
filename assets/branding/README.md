# Ame marks

`ame.svg` and `ame_light.svg` are the supplied Revaea artwork.
The colored mark supplies the Windows application and installer icon. The workbench uses the
theme-colored fill beside the `Ame` wordmark, without an added contour. The shared workbench bar
owns its display size, alignment and spacing, accounting for the artwork's transparent padding.

Regenerate the checked-in PNG density variants and multi-resolution ICO with:

```powershell
uv run tool/branding_generate_icons.py
```

The generator produces 28-pixel fill and outline PNGs with 2x, 3x and 4x variants. Flutter loads the
fill through its built-in asset bundle and resolves a variant for the display density.
The generated artwork retains the original central cutout and the shared transparent inset.
No SVG renderer ships with the application. The generation tool uses
[resvg-py](https://github.com/baseplate-admin/resvg-py) (MIT) and
[Pillow](https://python-pillow.org/) (MIT-CMU) only during asset preparation.
