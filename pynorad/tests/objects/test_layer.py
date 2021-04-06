import pytest

from ufoLib2.objects import Glyph, Layer


def test_init_layer_with_glyphs_dict():
    a = Glyph()
    b = Glyph()

    layer = Layer("My Layer", {"a": a, "b": b})

    assert layer.name == "My Layer"
    assert "a" in layer
    assert layer["a"] is a
    assert a.name == "a"
    assert "b" in layer
    assert layer["b"] is b
    assert b.name == "b"

    with pytest.raises(
        ValueError, match="glyph has incorrect name: expected 'a', found 'b'"
    ):
        Layer(glyphs={"a": b})

    with pytest.raises(KeyError, match=".*Glyph .* can't be added twice"):
        Layer(glyphs={"a": a, "b": a})

    with pytest.raises(TypeError, match="Expected Glyph, found int"):
        Layer(glyphs={"a": 1})


def test_init_layer_with_glyphs_list():
    a = Glyph("a")
    b = Glyph("b")
    layer = Layer(glyphs=[a, b])

    assert layer["a"] is a
    assert layer["b"] is b

    with pytest.raises(KeyError, match=".*Glyph .* can't be added twice"):
        Layer(glyphs=[a, a])

    c = Glyph()
    with pytest.raises(ValueError, match=".*Glyph .* has no name"):
        Layer(glyphs=[c])

    with pytest.raises(KeyError, match="glyph named 'b' already exists"):
        Layer(glyphs=[a, b, Glyph("b")])

    with pytest.raises(TypeError, match="Expected Glyph, found int"):
        Layer(glyphs=[1])


def test_addGlyph():
    a = Glyph("a")

    layer = Layer()

    layer.addGlyph(a)

    assert "a" in layer
    assert layer["a"] is a

    with pytest.raises(KeyError, match="glyph named 'a' already exists"):
        layer.addGlyph(a)


def test_insertGlyph():
    g = Glyph()
    pen = g.getPen()
    pen.moveTo((0, 0))
    pen.lineTo((1, 1))
    pen.lineTo((0, 1))
    pen.closePath()

    layer = Layer()
    layer.insertGlyph(g, "a")

    assert "a" in layer
    assert layer["a"].name == "a"
    assert layer["a"].contours == g.contours
    assert layer["a"] is not g

    layer.insertGlyph(g, "b")
    assert "b" in layer
    assert layer["b"].name == "b"
    assert layer["b"].contours == layer["a"].contours
    assert layer["b"] is not layer["a"]
    assert layer["b"] is not g

    assert g.name is None

    with pytest.raises(KeyError, match="glyph named 'a' already exists"):
        layer.insertGlyph(g, "a", overwrite=False)

    with pytest.raises(ValueError, match=".*Glyph .* has no name; can't add it"):
        layer.insertGlyph(g)


def test_newGlyph():
    layer = Layer()
    a = layer.newGlyph("a")

    assert "a" in layer
    assert layer["a"] is a

    with pytest.raises(KeyError, match="glyph named 'a' already exists"):
        layer.newGlyph("a")


def test_renameGlyph():
    g = Glyph()

    layer = Layer(glyphs={"a": g})
    assert g.name == "a"

    layer.renameGlyph("a", "a")  # no-op
    assert g.name == "a"

    layer.renameGlyph("a", "b")
    assert g.name == "b"

    layer.insertGlyph(g, "a")

    with pytest.raises(KeyError, match="target glyph named 'a' already exists"):
        layer.renameGlyph("b", "a")
