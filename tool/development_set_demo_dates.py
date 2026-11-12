# /// script
# requires-python = ">=3.12"
# dependencies = ["piexif==1.1.3"]
# ///
"""Set capture, creation and modification dates on manifest-listed demo JPEGs."""

import argparse
import ctypes
import io
import json
import math
import os
import sys
import tempfile
from ctypes import wintypes
from datetime import UTC, datetime, timedelta
from pathlib import Path

import piexif

DEFAULT_LIBRARY = Path(__file__).resolve().parents[1] / "build/test/demo-library"


def date_value(value: str) -> datetime:
    parsed = datetime.fromisoformat(value)
    return parsed.astimezone() if parsed.tzinfo is None else parsed


def set_file_times(path: Path, created: datetime, modified: datetime) -> None:
    kernel = ctypes.WinDLL("kernel32", use_last_error=True)
    kernel.CreateFileW.argtypes = [
        wintypes.LPCWSTR,
        wintypes.DWORD,
        wintypes.DWORD,
        wintypes.LPVOID,
        wintypes.DWORD,
        wintypes.DWORD,
        wintypes.HANDLE,
    ]
    kernel.CreateFileW.restype = wintypes.HANDLE
    kernel.SetFileTime.argtypes = [
        wintypes.HANDLE,
        ctypes.POINTER(wintypes.FILETIME),
        ctypes.POINTER(wintypes.FILETIME),
        ctypes.POINTER(wintypes.FILETIME),
    ]
    kernel.SetFileTime.restype = wintypes.BOOL
    kernel.CloseHandle.argtypes = [wintypes.HANDLE]
    kernel.CloseHandle.restype = wintypes.BOOL

    def file_time(value: datetime) -> wintypes.FILETIME:
        ticks = int(value.timestamp() * 10_000_000) + 116444736000000000
        return wintypes.FILETIME(ticks & 0xFFFFFFFF, ticks >> 32)

    handle = kernel.CreateFileW(str(path), 0x100, 7, None, 3, 0x80, None)
    if handle == ctypes.c_void_p(-1).value:
        raise ctypes.WinError(ctypes.get_last_error())
    try:
        creation_time = file_time(created)
        modification_time = file_time(modified)
        if not kernel.SetFileTime(
            handle, ctypes.byref(creation_time), None, ctypes.byref(modification_time)
        ):
            raise ctypes.WinError(ctypes.get_last_error())
    finally:
        kernel.CloseHandle(handle)


def write_capture_date(path: Path, captured: datetime) -> None:
    original = path.read_bytes()
    metadata = piexif.load(original)
    stamp = captured.strftime("%Y:%m:%d %H:%M:%S").encode("ascii")
    offset = captured.strftime("%z")
    offset = f"{offset[:3]}:{offset[3:]}".encode("ascii")
    metadata["0th"][piexif.ImageIFD.DateTime] = stamp
    metadata["Exif"][piexif.ExifIFD.DateTimeOriginal] = stamp
    metadata["Exif"][piexif.ExifIFD.DateTimeDigitized] = stamp
    for tag in (0x9010, 0x9011, 0x9012):
        metadata["Exif"][tag] = offset
    for tag in (
        piexif.ExifIFD.SubSecTime,
        piexif.ExifIFD.SubSecTimeOriginal,
        piexif.ExifIFD.SubSecTimeDigitized,
    ):
        metadata["Exif"].pop(tag, None)
    output = io.BytesIO()
    piexif.insert(piexif.dump(metadata), original, output)
    with tempfile.NamedTemporaryFile(
        dir=path.parent, suffix=".date-tmp", delete=False
    ) as target:
        temporary = Path(target.name)
        target.write(output.getvalue())
    try:
        temporary.replace(path)
    finally:
        temporary.unlink(missing_ok=True)


def selected_images(library: Path, filenames: list[str]) -> list[Path]:
    manifest = json.loads((library / "sources.json").read_text(encoding="utf-8"))
    if manifest.get("kind") != "ame-demo-library-v1":
        raise ValueError("Only a manifest-created demonstration library is supported")
    images = []
    for item in manifest["images"]:
        relative = Path(item["file"])
        if (
            relative.is_absolute()
            or relative.parent != Path("images")
            or relative.suffix.lower() != ".jpg"
        ):
            raise ValueError(f"Invalid demonstration image path: {relative}")
        path = library / relative
        if (
            not path.resolve().is_relative_to(library)
            or path.is_symlink()
            or path.stat().st_nlink != 1
        ):
            raise ValueError(
                f"Image must be a regular file inside this demonstration library: {relative}"
            )
        if not filenames or path.name in filenames:
            images.append(path)
    missing = set(filenames) - {path.name for path in images}
    if missing:
        raise ValueError(
            f"Images are not in the demonstration manifest: {sorted(missing)}"
        )
    if not images:
        raise ValueError("No demonstration images selected")
    return sorted(images)


def main() -> None:
    sys.stdout.reconfigure(encoding="utf-8")
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--library", type=Path, default=DEFAULT_LIBRARY)
    parser.add_argument(
        "--start", type=date_value, default=date_value("2021-01-01T12:00:00")
    )
    parser.add_argument(
        "--end", type=date_value, default=date_value("2026-10-31T12:00:00")
    )
    parser.add_argument(
        "--per-day", type=int, default=5, help="Images sharing one capture date"
    )
    parser.add_argument(
        "--created-days",
        type=float,
        default=1,
        help="Creation offset from capture, in days",
    )
    parser.add_argument(
        "--modified-days",
        type=float,
        default=2,
        help="Modification offset from capture, in days",
    )
    parser.add_argument(
        "--created-at",
        type=date_value,
        help="Fixed creation time, overriding --created-days",
    )
    parser.add_argument(
        "--modified-at",
        type=date_value,
        help="Fixed modification time, overriding --modified-days",
    )
    parser.add_argument(
        "--file",
        action="append",
        default=[],
        help="One manifest filename; repeat to select several",
    )
    parser.add_argument(
        "--preview", action="store_true", help="Print dates without changing images"
    )
    args = parser.parse_args()
    if os.name != "nt":
        parser.error("Windows is required to set filesystem creation time")
    if args.per_day < 1 or args.end < args.start:
        parser.error(
            "Use a positive --per-day and an end date on or after the start date"
        )
    library = args.library.resolve()
    images = selected_images(library, args.file)
    groups = math.ceil(len(images) / args.per_day)
    span_days = (args.end.date() - args.start.date()).days
    report = []
    for index, path in enumerate(images):
        day_offset = round(span_days * (index // args.per_day) / max(1, groups - 1))
        captured = args.start + timedelta(days=day_offset)
        created = args.created_at or captured + timedelta(days=args.created_days)
        modified = args.modified_at or captured + timedelta(days=args.modified_days)
        if not args.preview:
            write_capture_date(path, captured)
            set_file_times(path, created, modified)
        report.append(
            {
                "file": path.name,
                "captured": captured.isoformat(),
                "created": created.isoformat(),
                "modified": modified.isoformat(),
            }
        )
        print(
            f"{path.name}: capture={captured.isoformat()} created={created.isoformat()} modified={modified.isoformat()}"
        )
    if not args.preview:
        record = {
            "written_at": datetime.now(UTC).isoformat(),
            "images": report,
        }
        (library / "dates.json").write_text(
            json.dumps(record, indent=2) + "\n", encoding="utf-8"
        )
    print(
        f"{'Previewed' if args.preview else 'Updated'} {len(images)} demonstration images"
    )


if __name__ == "__main__":
    main()
