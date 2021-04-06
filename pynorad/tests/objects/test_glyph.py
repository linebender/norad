from pathlib import Path

import pytest

from ufoLib2.objects import (
    Anchor,
    Component,
    Contour,
    Font,
    Glyph,
    Guideline,
    Image,
    Layer,
    Point,
)
from ufoLib2.objects.misc import BoundingBox


def test_glyph_defcon_behavior():
    glyph = Glyph()
    glyph.appendAnchor(Anchor(1, 2, "top"))
    glyph.appendAnchor({"x": 3, "y": 4, "name": "bottom"})
    assert glyph.anchors == [Anchor(1, 2, "top"), Anchor(3, 4, "bottom")]

    glyph = Glyph()
    glyph.appendContour(Contour([Point(1, 2)]))
    assert glyph.contours == [Contour([Point(1, 2)])]

    glyph = Glyph()
    glyph.appendGuideline(Guideline(x=1))
    glyph.appendGuideline({"x": 2})
    assert glyph.guidelines == [Guideline(x=1), Guideline(x=2)]


def test_copyDataFromGlyph(ufo_UbuTestData):
    font = ufo_UbuTestData

    a = font["a"]
    a.height = 500
    a.image = Image("a.png")
    a.note = "a note"
    a.lib = {"bar": [3, 2, 1]}
    a.anchors = [Anchor(250, 0, "bottom")]
    a.guidelines = [Guideline(y=500)]
    a.components = [Component("A")]

    b = Glyph("b")
    b.width = 350
    b.height = 1000
    b.image = Image("b.png")
    b.note = "b note"
    b.lib = {"foo": [1, 2, 3]}
    b.anchors = [Anchor(350, 800, "top")]
    b.guidelines = [Guideline(x=50)]

    assert b.name != a.name
    assert b.width != a.width
    assert b.height != a.height
    assert b.unicodes != a.unicodes
    assert b.image != a.image
    assert b.note != a.note
    assert b.lib != a.lib
    assert b.anchors != a.anchors
    assert b.guidelines != a.guidelines
    assert b.contours != a.contours
    assert b.components != a.components

    def _assert_equal_but_distinct_objects(glyph1, glyph2):
        assert glyph1.width == glyph2.width
        assert glyph1.height == glyph2.height
        assert glyph1.unicodes == glyph2.unicodes
        assert glyph1.unicodes is not glyph2.unicodes
        assert glyph1.image == glyph2.image
        assert glyph1.image is not glyph2.image
        assert glyph1.note == glyph2.note
        assert glyph1.lib == glyph2.lib
        assert glyph1.lib is not glyph2.lib
        assert glyph1.lib["bar"] == glyph2.lib["bar"]
        assert glyph1.lib["bar"] is not glyph2.lib["bar"]
        assert glyph1.anchors == glyph2.anchors
        assert glyph1.anchors is not glyph2.anchors
        assert glyph1.anchors[0] is not glyph2.anchors[0]
        assert glyph1.guidelines == glyph2.guidelines
        assert glyph1.guidelines is not glyph2.guidelines
        assert glyph1.guidelines[0] is not glyph2.guidelines[0]
        assert glyph1.contours == glyph2.contours
        assert glyph1.contours is not glyph2.contours
        assert glyph1.contours[0] is not glyph2.contours[0]
        assert glyph1.components == glyph2.components
        assert glyph1.components is not glyph2.components
        assert glyph1.components[0] is not glyph2.components[0]

    b.copyDataFromGlyph(a)
    assert b.name != a.name
    _assert_equal_but_distinct_objects(b, a)

    c = a.copy()
    assert c.name == a.name
    _assert_equal_but_distinct_objects(c, a)

    d = a.copy(name="d")
    assert d.name == "d"
    _assert_equal_but_distinct_objects(d, a)


def test_appendContour(ufo_UbuTestData):
    font = ufo_UbuTestData

    A = font["A"]
    n = len(A.contours)

    c = Contour(points=[Point(0, 0), Point(1, 1)])

    A.appendContour(c)

    assert len(A.contours) == n + 1
    assert A.contours[-1] is c

    with pytest.raises(TypeError, match="Expected Contour, found object"):
        A.appendContour(object())


def test_glyph_without_name():
    assert Glyph().name is None


def test_glyph_repr():
    g = Glyph()
    assert repr(g) == f"<ufoLib2.objects.glyph.Glyph at {hex(id(g))}>"

    g = Glyph("a")
    assert repr(g) == f"<ufoLib2.objects.glyph.Glyph 'a' at {hex(id(g))}>"


def test_glyph_get_bounds():
    a = Glyph("a")
    pen = a.getPen()
    pen.moveTo((0, 0))
    pen.curveTo((10, 10), (10, 20), (0, 20))
    pen.closePath()

    b = Glyph("b", components=[Component("a", (1, 0, 0, 1, -50, 100))])

    layer = Layer(glyphs=[a, b])

    assert a.getBounds(layer) == BoundingBox(xMin=0, yMin=0, xMax=7.5, yMax=20)

    assert a.getControlBounds(layer) == BoundingBox(xMin=0, yMin=0, xMax=10, yMax=20)

    with pytest.raises(
        TypeError, match="layer is required to compute bounds of components"
    ):
        b.getBounds()
    with pytest.raises(
        TypeError, match="layer is required to compute bounds of components"
    ):
        b.getControlBounds()

    assert b.getBounds(layer) == (-50, 100, -42.5, 120)  # namedtuple is a tuple
    assert b.getControlBounds(layer) == (-50, 100, -40, 120)


def test_glyph_get_bounds_empty():
    g = Glyph()
    assert g.getBounds() is None
    assert g.getControlBounds() is None


@pytest.fixture
def layer():
    a = Glyph("a")
    pen = a.getPen()
    pen.moveTo((8, 0))
    pen.lineTo((18, 0))
    pen.lineTo((18, 20))
    pen.lineTo((8, 20))
    pen.closePath()
    a.width = 30
    a.appendAnchor({"x": 10, "y": 30, "name": "top"})

    b = Glyph("b", width=a.width, components=[Component("a", (1, 0, 0, 1, 2, -5))])

    layer = Layer(glyphs=[a, b])
    return layer


def test_glyph_get_margins(layer):
    a = layer["a"]

    # for simple contour glyphs without components, layer is optional/unused
    assert a.getLeftMargin() == 8
    assert a.getLeftMargin(layer) == 8
    assert a.getRightMargin() == 12
    assert a.getRightMargin(layer) == 12
    assert a.getBottomMargin() == 0
    assert a.getBottomMargin(layer) == 0
    assert a.getTopMargin() == -20
    assert a.getTopMargin(layer) == -20

    a.verticalOrigin = 20
    assert a.getBottomMargin() == -20
    assert a.getBottomMargin(layer) == -20
    assert a.getTopMargin() == 0
    assert a.getTopMargin(layer) == 0

    b = layer["b"]
    # for composite glyphs, layer is required
    for m in ("Left", "Right", "Top", "Bottom"):
        with pytest.raises(TypeError, match="layer is required to compute bounds"):
            getattr(b, f"get{m}Margin")()

    assert b.getLeftMargin(layer) == 10
    assert b.getRightMargin(layer) == 10
    assert b.getBottomMargin(layer) == -5
    assert b.getTopMargin(layer) == -15

    b.verticalOrigin = 15
    assert b.getBottomMargin(layer) == -20
    assert b.getTopMargin(layer) == 0

    c = Glyph()  # empty glyph
    assert c.getLeftMargin() is None
    assert c.getRightMargin() is None
    assert c.getBottomMargin() is None
    assert c.getTopMargin() is None


def test_simple_glyph_set_left_margins(layer):
    a = layer["a"]
    b = layer["b"]  # same width, has component 'a' shifted +2 horizontally

    assert a.getLeftMargin() == 8
    assert b.getLeftMargin(layer) == 10
    assert a.width == 30
    assert b.width == 30
    assert a.anchors[0].x, a.anchors[0].y == (10, 20)

    a.setLeftMargin(8)  # no change
    assert a.getLeftMargin() == 8
    assert a.width == 30

    a.setLeftMargin(10)  # +2
    assert a.getLeftMargin() == 10
    assert a.width == 32
    # anchors were shifted
    assert a.anchors[0].x, a.anchors[0].y == (12, 20)
    # composite glyph "b" also shifts, but keeps original width
    assert b.getLeftMargin(layer) == 12
    assert b.width == 30

    a.setLeftMargin(-2)  # -12
    assert a.getLeftMargin(-2)
    assert a.width == 20


def test_composite_glyph_set_left_margins(layer):
    b = layer["b"]

    assert b.getLeftMargin(layer) == 10
    assert b.width == 30

    b.setLeftMargin(12, layer)  # +2
    assert b.getLeftMargin(layer) == 12
    assert b.width == 32


def test_simple_glyph_set_right_margins(layer):
    a = layer["a"]
    b = layer["b"]  # same width, has component 'a' shifted +2 horizontally

    assert a.getRightMargin() == 12
    assert b.getRightMargin(layer) == 10
    assert a.width == 30
    assert b.width == 30
    assert a.anchors[0].x, a.anchors[0].y == (10, 20)

    a.setRightMargin(12)  # no change
    assert a.getRightMargin() == 12
    assert a.width == 30

    a.setRightMargin(10)  # -2
    assert a.getRightMargin() == 10
    # only width changes, anchors stay same
    assert a.width == 28
    assert a.anchors[0].x, a.anchors[0].y == (10, 20)
    # composite glyph "b" does _not_ change when "a" RSB changes
    assert b.getRightMargin(layer) == 10
    assert b.width == 30

    a.setRightMargin(-2)  # -12
    assert a.getRightMargin() == -2
    assert a.width == 16


def test_composite_glyph_set_right_margins(layer):
    b = layer["b"]

    assert b.getRightMargin(layer) == 10
    assert b.width == 30

    b.setRightMargin(12, layer)  # +2
    assert b.getRightMargin(layer) == 12
    assert b.width == 32


def test_simple_glyph_set_bottom_margins(layer):
    a = layer["a"]
    b = layer["b"]  # same height/origin, has component 'a' shifted -5 vertically
    a.verticalOrigin = b.verticalOrigin = a.height = b.height = 30

    assert a.getBottomMargin() == 0
    assert b.getBottomMargin(layer) == -5

    a.setBottomMargin(-10)
    assert a.getBottomMargin(layer) == -10
    assert a.height == 20
    assert a.verticalOrigin == 30
    # composite glyph "b" does not change
    assert b.getBottomMargin(layer) == -5
    assert b.height == b.verticalOrigin == 30


def test_composite_glyph_set_bottom_margins(layer):
    b = layer["b"]
    b.verticalOrigin = b.height = 30

    assert b.getBottomMargin(layer) == -5
    assert b.height == 30

    b.setBottomMargin(0, layer)  # +5
    assert b.getBottomMargin(layer) == 0
    assert b.height == 35


def test_simple_glyph_set_top_margins(layer):
    a = layer["a"]
    b = layer["b"]  # same height/origin, has component 'a' shifted -5 vertically
    a.verticalOrigin = b.verticalOrigin = a.height = b.height = 30

    assert a.getTopMargin() == 10
    assert b.getTopMargin(layer) == 15

    a.setTopMargin(-10)
    assert a.getTopMargin() == -10
    assert a.height == 10
    assert a.verticalOrigin == 10
    # composite glyph "b" does not change
    assert b.getTopMargin(layer) == 15
    assert b.height == b.verticalOrigin == 30


def test_composite_glyph_set_top_margins(layer):
    b = layer["b"]
    b.verticalOrigin = b.height = 30

    assert b.getTopMargin(layer) == 15
    assert b.height == 30

    b.setTopMargin(10, layer)  # -5
    assert b.getTopMargin(layer) == 10
    assert b.height == 25


def test_composite_margin_roundtrip(datadir: Path) -> None:
    msans = Font.open(datadir / "MutatorSansBoldCondensed.ufo")

    comma = msans["comma"]

    assert comma.getLeftMargin(msans) == 30
    assert comma.getRightMargin(msans) == 30
    assert comma.width == 250

    # Quotedblleft consists of two inverted commas:
    quotedblleft = msans["quotedblleft"]

    assert quotedblleft.getLeftMargin(msans) == 30
    assert quotedblleft.getRightMargin(msans) == 30
    assert quotedblleft.width == 480

    # Now change comma and verify indirect change in quotedblleft.
    comma.setLeftMargin(27, msans)
    comma.setRightMargin(27, msans)

    assert comma.getLeftMargin(msans) == 27
    assert comma.getRightMargin(msans) == 27
    assert comma.width == 244

    assert quotedblleft.getLeftMargin(msans) == 33
    assert quotedblleft.getRightMargin(msans) == 27
    assert quotedblleft.width == 480

    # Changing margins of quotedblleft should not affect comma.
    quotedblleft.setLeftMargin(23, msans)
    quotedblleft.setRightMargin(22, msans)

    assert comma.getLeftMargin(msans) == 27
    assert comma.getRightMargin(msans) == 27
    assert comma.width == 244

    # Quotedblleft should however have the exact margins we gave above.
    assert quotedblleft.getLeftMargin(msans) == 23
    assert quotedblleft.getRightMargin(msans) == 22
    assert quotedblleft.width == 465
