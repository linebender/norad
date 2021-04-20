from collections import OrderedDict
from copy import deepcopy

import pytest
from fontTools import ufoLib

import pynorad as ufoLib2
from pynorad import _NOT_LOADED, LayerSet, Layer
# import pynorad.objects
# from pynorad.objects import Layer, LayerSet
# from pynorad.objects.misc import _NOT_LOADED


def test_import_version():
    assert hasattr(ufoLib2, "__version__")
    assert isinstance(ufoLib2.__version__, str)


def test_LayerSet_load_layers_on_iteration(tmp_path):
    ufo = ufoLib2.Font()
    ufo.layers.newLayer("test")
    ufo_save_path = tmp_path / "test.ufo"
    ufo.save(ufo_save_path)
    ufo = ufoLib2.Font.open(ufo_save_path)
    keys = ufo.layers.keys()
    assert set(keys) == {"public.default", "test"}
    for layer in ufo.layers:
        # assert layer is not _NOT_LOADED
        assert layer.name in keys


# def test_lazy_data_loading_saveas(ufo_UbuTestData, tmp_path):
    # ufo = ufo_UbuTestData
    # ufo_path = tmp_path / "UbuTestData2.ufo"
    # ufo.save(ufo_path)
    # assert all(v is not _NOT_LOADED for v in ufo.data._data.values())


# def test_lazy_data_loading_inplace_no_load(ufo_UbuTestData):
    # ufo = ufo_UbuTestData
    # ufo.save()
    # assert all(v is _NOT_LOADED for v in ufo.data._data.values())


# def test_lazy_data_loading_inplace_load_some(ufo_UbuTestData):
    # ufo = ufo_UbuTestData
    # some_data = b"abc"
    # ufo.data["com.github.fonttools.ttx/T_S_I__0.ttx"] = some_data
    # ufo.save()
    # assert all(
        # v is _NOT_LOADED for k, v in ufo.data._data.items() if "T_S_I__0" not in k
    # )
    # assert ufo.data["com.github.fonttools.ttx/T_S_I__0.ttx"] == some_data


def test_constructor_from_path(datadir):
    path = datadir / "UbuTestData.ufo"
    font = ufoLib2.Font(path)

    assert font._path == path
    # assert font._lazy is True
    assert font._validate is True
    # assert font._reader is not None

    font2 = ufoLib2.Font(path, lazy=False, validate=False)

    assert font2._path == path
    assert font2._lazy is False
    # assert font2._validate is False
    # assert font2._reader is None

    assert font == font2


def test_deepcopy_lazy_object(datadir):
    path = datadir / "UbuTestData.ufo"
    font1 = ufoLib2.Font.open(path, lazy=True)

    font2 = deepcopy(font1)

    assert font1 is not font2
    assert font1 == font2

    assert font1.layers is not font2.layers
    assert font1.layers == font2.layers

    assert font1.layers.defaultLayer is not font2.layers.defaultLayer
    assert font1.layers.defaultLayer == font2.layers.defaultLayer

    assert font1.data is not font2.data
    assert font1.data == font2.data

    assert font1.images is not font2.images
    assert font1.images == font2.images

    # assert font1.reader is not None
    # assert not font1.reader.fs.isclosed()
    # assert not font1._lazy

    # assert font2.reader is None
    # assert not font2._lazy

    assert font1.path == path
    assert font2.path is None


def test_unlazify(datadir):
    reader = ufoLib.UFOReader(datadir / "UbuTestData.ufo")
    font = ufoLib2.Font.read(reader, lazy=True)

    # assert font._reader is reader
    # assert not reader.fs.isclosed()

    font.unlazify()

    assert font._lazy is False


def test_font_eq_and_ne(ufo_UbuTestData):
    font1 = ufo_UbuTestData
    font2 = deepcopy(font1)

    assert font1 == font2

    font1["a"].contours[0].points[0].x = 0

    assert font1 != font2


def test_empty_layerset():
    with pytest.raises(ValueError):
        LayerSet(layers={}, defaultLayer=None)


def test_default_layerset():
    layers = LayerSet.default()
    assert len(layers) == 1
    assert "public.default" in layers
    assert len(layers["public.default"]) == 0


def test_custom_layerset():
    default = Layer()
    ls1 = LayerSet.from_iterable([default])
    assert next(iter(ls1)) == ls1.defaultLayer

    with pytest.raises(ValueError):
        ls1 = LayerSet.from_iterable([Layer(name="abc")])

    ls2 = LayerSet.from_iterable([Layer(name="abc")], defaultLayerName="abc")
    assert ls2["abc"] == ls2.defaultLayer

    layers2 = OrderedDict()
    layers2["public.default"] = default
    LayerSet(layers=layers2, defaultLayer=default)


def test_guidelines():
    font = ufoLib2.Font()

    # accept either a mapping or a Guideline object
    font.appendGuideline({"x": 100, "y": 50, "angle": 315})
    font.appendGuideline(ufoLib2.objects.Guideline(x=30))

    assert len(font.guidelines) == 2
    assert font.guidelines == [
        ufoLib2.objects.Guideline(x=100, y=50, angle=315),
        ufoLib2.objects.Guideline(x=30),
    ]

    # setter should clear existing guidelines
    font.guidelines = [{"x": 100}, ufoLib2.objects.Guideline(y=20)]

    assert len(font.guidelines) == 2
    assert font.guidelines == [
        ufoLib2.objects.Guideline(x=100),
        ufoLib2.objects.Guideline(y=20),
    ]

def test_point_order_change(ufo_UbuTestData):
    font = ufo_UbuTestData
    glyph = font["a"]
    point = glyph.contours[0].points[5]
    assert point.x == 347.0
    del glyph.contours[0].points[0]
    # deleting this earlier point invalidates our previous point; currently
    # this raises an exception but we'd prefer it to just work
    assert point.x == 347.0
    point2 = glyph.contours[0].points[4]
    assert point == point2

