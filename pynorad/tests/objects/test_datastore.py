from pathlib import Path

import pytest

from ufoLib2 import Font


def test_imageset(tmp_path: Path) -> None:
    font = Font()
    font.images["test.png"] = b"\x89PNG\r\n\x1a\n123"
    font.images["test2.png"] = b"\x89PNG\r\n\x1a\n456"
    font_path = tmp_path / "a.ufo"
    font.save(font_path)

    font = Font.open(font_path)
    assert font.images["test.png"] == b"\x89PNG\r\n\x1a\n123"
    assert font.images["test2.png"] == b"\x89PNG\r\n\x1a\n456"

    with pytest.raises(ValueError, match=r".*subdirectories.*"):
        font.images["directory/test2.png"] = b"\x89PNG\r\n\x1a\n456"
    with pytest.raises(KeyError):
        font.images["directory/test2.png"]


def test_dataset(tmp_path: Path) -> None:
    font = Font()
    font.data["test.png"] = b"123"
    font.data["directory/test2.png"] = b"456"
    font_path = tmp_path / "a.ufo"
    font.save(font_path)

    font = Font.open(font_path)
    assert font.data["test.png"] == b"123"
    assert font.data["directory/test2.png"] == b"456"
