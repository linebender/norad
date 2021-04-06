import pytest

from ufoLib2.objects import Glyph


@pytest.fixture
def contour():
    g = Glyph("a")
    pen = g.getPen()
    pen.moveTo((0, 0))
    pen.curveTo((10, 10), (10, 20), (0, 20))
    pen.closePath()
    return g.contours[0]


def test_contour_getBounds(contour):
    assert contour.getBounds() == (0, 0, 7.5, 20)
    assert contour.getBounds(layer={}) == (0, 0, 7.5, 20)
    assert contour.bounds == (0, 0, 7.5, 20)


def test_contour_getControlBounds(contour):
    assert contour.getControlBounds() == (0, 0, 10, 20)
    assert contour.getControlBounds(layer={}) == (0, 0, 10, 20)
