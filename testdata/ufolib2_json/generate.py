"""Regenerate the ufoLib2 JSON fixtures in this directory.

Run from anywhere, in an environment with `ufoLib2[json]` installed:

    python testdata/ufolib2_json/generate.py

Writes `<stem>.json` for each existing norad test UFO, plus `kitchen_sink` as a
`.ufo`/`.json` pair. The sink exercises every feature of the format.
"""

import shutil
from pathlib import Path

import ufoLib2
from ufoLib2.objects import (
    Anchor,
    Component,
    Contour,
    Glyph,
    Guideline,
    Image,
    Point,
)

HERE = Path(__file__).resolve().parent
TESTDATA = HERE.parent

EXISTING_UFOS = [
    "MutatorSansLightWide.ufo",
    "dataimagetest.ufo",
    "fontinfotest.ufo",
    "fontinfotest_v1.ufo",
    "fontinfotest_v2.ufo",
    "fontinfotest_v3.ufo",
    "identifiers.ufo",
    "lineendings/Tester-LineEndings.ufo",
    "upconversion_kerning/glyphname_groupname_UFOv1.ufo",
    "upconversion_kerning/glyphname_groupname_UFOv2.ufo",
]

DATA_WRAPPER_TEST = bytes([0, 1, 2, 253, 254, 255])


def dump_existing() -> None:
    for rel in EXISTING_UFOS:
        path = TESTDATA / rel
        font = ufoLib2.Font.open(path, lazy=False)
        # fontTools' kerning upconversion builds groups in hash-seed dependent
        # order; sort them to keep the output deterministic.
        groups = sorted(font.groups.items())
        font.groups.clear()
        font.groups.update(groups)
        out = HERE / f"{path.stem}.json"
        out.write_bytes(font.json_dumps(indent=2) + b"\n")


def rich_lib() -> dict:
    """A lib with every plist value kind we can serialize."""
    return {
        "com.example.int": 42,
        "com.example.float": 1.5,
        "com.example.integral-float": 2.0,
        "com.example.bool": True,
        "com.example.string": "héllo",
        "com.example.list": [1, 2.5, "three", False, [4, 5]],
        "com.example.dict": {"a": 1, "nested": {"b": [True, {"c": "deep"}]}},
        "com.example.data": DATA_WRAPPER_TEST,
        "com.example.datalist": {"items": [1, b"\x00\xffabc", {"inner": b"xyz"}]},
    }


def rich_layer_lib() -> dict:
    """As `rich_lib` but without bytes: ufoLib2 crashes on bytes in a layer lib."""
    lib = rich_lib()
    del lib["com.example.data"], lib["com.example.datalist"]
    return lib


def make_glyph_a() -> Glyph:
    g = Glyph("A", width=600, height=800, unicodes=[0x41, 0xC0, 0x391])
    # Surrounding whitespace is stripped by fontTools when writing the .ufo.
    g.note = "  A glyph with everything.\n "
    g.lib.update(rich_lib())

    g.contours.append(
        Contour(
            points=[
                Point(0, 0, "line", identifier="p-line"),
                Point(0, 500, "line", smooth=True, name="top-left"),
                Point(100, 700, None),
                Point(200, 700, None),
                Point(300, 700, "curve", smooth=True, identifier="p-curve"),
                Point(400, 700, None),
                Point(500, 600, "qcurve"),
                Point(500, 0, "line"),
            ],
            identifier="c-closed",
        )
    )
    # An open contour, starting with a move point.
    g.contours.append(
        Contour(
            points=[
                Point(50, 50, "move", identifier="p-move"),
                Point(100, 100, "line"),
                Point(150, 60, "qcurve", name="qc"),
            ],
            identifier="c-open",
        )
    )
    g.anchors.append(Anchor(300, 800, name="top", color="1,0,0,0.5", identifier="a-top"))
    g.anchors.append(Anchor(300, -50, name="bottom", color="0,0.5,1,1"))
    g.anchors.append(Anchor(1.5, 2.5))

    g.components.append(Component("B"))
    g.components.append(
        Component(
            "B",
            transformation=(0.5, 0.1, -0.1, 0.75, 20, 30),
            identifier="comp-xf",
        )
    )

    g.guidelines.append(Guideline(x=250, name="vertical", color="0,1,0,1", identifier="g-x"))
    g.guidelines.append(Guideline(y=-20, identifier="g-y"))
    g.guidelines.append(Guideline(x=10, y=20, angle=45.5, name="angled", identifier="g-angle"))

    g.image = Image(
        fileName="image1.png",
        transformation=(0.5, 0, 0, 0.5, 10, 20),
        color="0.5,0.5,0.5,1",
    )

    # Object libs live in the glyph lib, keyed by identifier.
    g.objectLib(g.contours[0].points[0])["com.example.point"] = "on a point"
    g.objectLib(g.anchors[0])["com.example.anchor"] = [1, 2, 3]
    g.objectLib(g.guidelines[2])["com.example.guideline"] = {"k": True}
    g.objectLib(g.components[1])["com.example.component"] = 7
    g.objectLib(g.contours[0])["com.example.contour"] = "on a contour"
    return g


def make_glyph_b() -> Glyph:
    g = Glyph("B", width=500)
    g.unicodes = [0x42]
    g.contours.append(
        Contour(points=[Point(0, 0, "line"), Point(0, 100, "line"), Point(100, 100, "line")])
    )
    g.lib["com.example.b"] = "plain"
    return g


def make_offcurve_glyph() -> Glyph:
    """A glyph with a contour made only of off-curve points, which is valid."""
    g = Glyph("offcurves", width=100)
    g.contours.append(
        Contour(points=[Point(0, 0, None), Point(20, 20, None), Point(40, 0, None)])
    )
    return g


def make_kitchen_sink() -> ufoLib2.Font:
    font = ufoLib2.Font()

    # Layers. The default layer has a non-default name, so `"default": true`
    # is written.
    fg = font.layers.defaultLayer
    font.layers.renameLayer(fg.name, "foreground")
    font.layers.defaultLayer = font.layers["foreground"]
    fg = font.layers["foreground"]
    fg.insertGlyph(make_glyph_a())
    fg.insertGlyph(make_glyph_b())
    fg.insertGlyph(Glyph("space", width=250))
    fg.insertGlyph(make_offcurve_glyph())
    fg.lib.update(rich_layer_lib())
    fg.lib["com.example.layer-note"] = "default layer lib"

    bg = font.layers.newLayer("background", color="0.1,0.2,0.3,0.4")
    bg.lib["com.example.background"] = {"answer": 42, "pi": 3.25}
    bg.insertGlyph(Glyph("A", width=600))
    bg.insertGlyph(Glyph("onlybg", width=10))

    # Font-level lib, groups, kerning, features, data, images.
    font.lib.update(rich_lib())
    font.lib["public.glyphOrder"] = ["A", "B", "space"]
    font.groups["public.kern1.A"] = ["A"]
    font.groups["public.kern2.B"] = ["B"]
    font.groups["public.kern2.round"] = ["space"]
    font.groups["com.example.plain"] = ["A", "B"]
    font.kerning[("public.kern1.A", "public.kern2.B")] = -30
    font.kerning[("A", "B")] = 10.5
    font.kerning[("A", "public.kern2.round")] = -5
    font.kerning[("public.kern1.A", "space")] = 2
    font.features.text = "feature liga {\n    sub A B by space;\n} liga;\n"
    font.data["com.example.foo/bar.txt"] = b"hello\nworld\n"
    font.data["top.bin"] = bytes(range(256))
    font.images["image1.png"] = (
        TESTDATA / "dataimagetest.ufo/images/image1.png"
    ).read_bytes()

    info = font.info
    info.familyName = "Kitchen Sink"
    info.styleName = "Regular"
    info.styleMapFamilyName = "Kitchen Sink Reg"
    info.styleMapStyleName = "regular"
    info.versionMajor = 1
    info.versionMinor = 2
    info.copyright = "Copyright (c) nobody"
    info.trademark = "Sink (tm)"
    info.note = "a note"
    info.unitsPerEm = 1000.0  # integral float
    info.ascender = 750.5  # fractional float
    info.descender = -250
    info.capHeight = 700
    info.xHeight = 500
    info.italicAngle = -12.5
    info.openTypeHeadCreated = "2024/01/02 03:04:05"
    info.openTypeHeadLowestRecPPEM = 8
    info.openTypeHeadFlags = [0, 1, 4]
    info.openTypeHheaAscender = 800
    info.openTypeHheaDescender = -200
    info.openTypeHheaLineGap = 90
    info.openTypeHheaCaretSlopeRise = 1
    info.openTypeHheaCaretSlopeRun = 0
    info.openTypeNameDesigner = "Designer"
    info.openTypeNameLicense = "License text"
    info.openTypeNamePreferredFamilyName = "Sink Pref"
    info.openTypeNameRecords = [
        {"nameID": 1, "platformID": 3, "encodingID": 1, "languageID": 1033, "string": "Sink"},
        {"nameID": 0, "platformID": 1, "encodingID": 0, "languageID": 0, "string": "(c)"},
    ]
    info.openTypeOS2WidthClass = 5
    info.openTypeOS2WeightClass = 400
    info.openTypeOS2Selection = [1, 7]
    info.openTypeOS2VendorID = "NONE"
    info.openTypeOS2Panose = [2, 0, 5, 3, 0, 0, 0, 0, 0, 0]
    info.openTypeOS2FamilyClass = [1, 2]
    info.openTypeOS2UnicodeRanges = [0, 1, 2]
    info.openTypeOS2CodePageRanges = [0, 1]
    info.openTypeOS2TypoAscender = 750
    info.openTypeOS2TypoDescender = -250
    info.openTypeOS2TypoLineGap = 100
    info.openTypeOS2WinAscent = 900
    info.openTypeOS2WinDescent = 300
    info.openTypeOS2Type = [2]
    info.openTypeOS2SubscriptXSize = 650
    info.openTypeOS2SuperscriptYOffset = 480
    info.openTypeOS2StrikeoutSize = 50
    info.openTypeGaspRangeRecords = [
        {"rangeMaxPPEM": 8, "rangeGaspBehavior": [1, 3]},
        {"rangeMaxPPEM": 65535, "rangeGaspBehavior": [0, 1, 2, 3]},
    ]
    info.openTypeVheaVertTypoAscender = 400
    info.postscriptFontName = "KitchenSink-Regular"
    info.postscriptFullName = "Kitchen Sink Regular"
    info.postscriptUnderlineThickness = 50.5
    info.postscriptUnderlinePosition = -100
    info.postscriptIsFixedPitch = False
    info.postscriptBlueValues = [-10, 0, 700, 710]
    info.postscriptBlueScale = 0.039625
    info.postscriptForceBold = True
    info.postscriptDefaultWidthX = 500
    info.postscriptWindowsCharacterSet = 1
    info.macintoshFONDName = "Sink"
    info.macintoshFONDFamilyID = 15000
    info.woffMajorVersion = 1
    info.woffMinorVersion = 0
    info.woffMetadataUniqueID = {"id": "com.example.sink"}
    info.woffMetadataVendor = {"name": "Vendor", "url": "https://example.com", "dir": "ltr", "class": "vend"}
    info.woffMetadataCredits = {"credits": [{"name": "Someone", "role": "Design"}]}
    info.woffMetadataDescription = {
        "url": "https://example.com/d",
        "text": [{"text": "Hello", "language": "en"}, {"text": "Bonjour"}],
    }
    info.woffMetadataLicense = {"url": "https://example.com/l", "id": "lic", "text": [{"text": "MIT"}]}
    info.woffMetadataCopyright = {"text": [{"text": "(c)", "dir": "rtl"}]}
    info.woffMetadataTrademark = {"text": [{"text": "tm"}]}
    info.woffMetadataLicensee = {"name": "Licensee"}
    info.woffMetadataExtensions = [
        {
            "id": None,  # required by ufoLib2 and written as `"id": null`
            "names": [{"text": "Ext"}],
            "items": [
                {
                    "id": "item1",
                    "names": [{"text": "Item"}],
                    "values": [{"text": "Value", "language": "en"}],
                },
                {"names": [{"text": "No id"}], "values": [{"text": "v"}]},
            ],
        },
        {"id": "ext2", "names": [{"text": "Second"}], "items": [{"names": [{"text": "n"}], "values": [{"text": "v"}]}]},
    ]

    info.guidelines = [
        Guideline(x=100, name="fx", color="1,1,0,1", identifier="fg-x"),
        Guideline(y=200, identifier="fg-y"),
        Guideline(x=1, y=2, angle=90, identifier="fg-angle"),
    ]
    font.objectLib(info.guidelines[0])["com.example.fontguide"] = "font-level"
    return font


def dump_pair(name: str, font: ufoLib2.Font) -> None:
    shutil.rmtree(HERE / f"{name}.ufo", ignore_errors=True)
    font.save(HERE / f"{name}.ufo", overwrite=True)
    json = font.json_dumps(indent=2)
    (HERE / f"{name}.json").write_bytes(json + b"\n")
    # Sanity check: the output is valid ufoLib2 JSON.
    ufoLib2.Font.json_loads(json)


if __name__ == "__main__":
    dump_existing()
    dump_pair("kitchen_sink", make_kitchen_sink())
