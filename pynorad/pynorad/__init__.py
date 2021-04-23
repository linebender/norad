from typing import Iterable, OrderedDict, Optional, Any, Tuple
from fontTools.pens.pointPen import PointToSegmentPen, SegmentToPointPen
from fontTools.misc.transform import Transform
from .pynorad import PyFont, PyGuideline, PyPointPen, PyLayer, PyGlyph, PyFontInfo

# I acknowledge that this is not the right way to do this
__version__ = '0.1'

DEFAULT_LAYER_NAME = "public.default"
# this is something that exists in ufoLib2; we bring it across so that we
# can modify tests as little as possible.
class Placeholder:
    """Represents a sentinel value to signal a "lazy" object hasn't been loaded yet."""

_NOT_LOADED = Placeholder()


class Font(object):
    """A fontfile"""
    def __init__(self, path = None, **kwargs):
        self._path = path
        self._reader = None
        self._lazy = False
        self._validate = True

        if path is None:
            self._font = PyFont()
        else:
            self._font = PyFont.load(str(path))

    def __eq__(self, other):
        if other.__class__ is not self.__class__:
            return NotImplemented
        return self._font.py_eq(other._font)

    def __len__(self):
        return self._font.default_layer().len()

    def __deepcopy__(self, memo):
        result = Font(None)
        result._font = self._font.deep_copy()
        return result

    def __getitem__(self, name):
        return Glyph.proxy(self._font.default_layer().glyph(name))

    def __setitem__(self, name: str, glyph):
        self._font.default_layer().set_glyph(glyph._glyph)

    def __delitem__(self, name: str):
        self._font.default_layer().remove_glyph(name)

    def __contains__(self, glyphName: str):
        return self._font.default_layer().contains(glyphName)

    def newLayer(self, layerName: str):
        return Layer.proxy(self._font.new_layer(layerName))

    def addGlyph(self, glyph):
        Layer.proxy(self._font.default_layer()).addGlyph(glyph)

    def appendGuideline(self, guideline):
        if guideline.__class__ is not Guideline:
            guideline = Guideline(**guideline)
        self._font.append_guideline(guideline._guideline)


    def newGlyph(self, name: str):
        return self._font.default_layer().new_glyph(name)

    def renameGlyph(self, old: str, new: str, overwrite: bool = False):
        Layer.proxy(self._font.default_layer()).renameGlyph(old, new, overwrite=overwrite)

    def __iter__(self):
        return IterWrapper(Glyph, self._font.default_layer().iter_glyphs())


    @classmethod
    def open(cls, path, lazy=True, validate=True):
        if not validate:
            print("Pynorad always validates input")
        return cls(path)

    @classmethod
    def read(cls, reader, **kwargs):
        """API compat with ufoLib2"""
        return cls.open(reader._path)


    def save(self, path):
        self._font.save(str(path))

    @property
    def layers(self):
        return LayerSet.proxy(self._font)

    @property
    def info(self):
        return FontInfo.proxy(self._font.fontinfo())

    @property
    def guidelines(self):
        return [Guideline.proxy(g) for g in self._font.guidelines()]

    @guidelines.setter
    def guidelines(self, value):
        self._font.replace_guidelines([Guideline.normalize(g)._guideline for g in value])


    #FIXME: norad doesn't impl data yet
    @property
    def data(self):
        return dict()

    #FIXME: norad doesn't impl images yet
    @property
    def images(self):
        return dict()

    @property
    def path(self):
        return self._path

    def unlazify(self):
        pass

class Layer:
    def __init__(self, name: str = 'public.default', glyphs = None, color = None, lib = None, proxy = None):
        if proxy is not None:
            assert proxy.__class__ == PyLayer
            self._layer = proxy
        else:
            self._layer = PyLayer.concrete(name)
            if glyphs is not None:
                if not isinstance(glyphs, dict):
                    # check for dupe names
                    names = set()
                    for glyph in glyphs:
                        if not isinstance(glyph, Glyph):
                            raise TypeError(f"Expected Glyph, found {type(glyph).__name__}")
                        name = glyph.name
                        if name in names:
                            raise KeyError(f"glyph named '{name}' already exists")
                        names.add(name)

                    # convert to a dict
                    glyphs = { g.name: g for g in glyphs }
                for name, glyph in glyphs.items():
                    if not isinstance(glyph, Glyph):
                        raise TypeError(f"Expected Glyph, found {type(glyph).__name__}")
                    currentName = glyph.name
                    if currentName is None or currentName == "":
                        glyph._name = name or ""
                    elif currentName != name:
                        raise ValueError(
                            "glyph has incorrect name: "
                            f"expected '{name}', found '{glyph.name}'"
                        )
                    self.addGlyph(glyph)

    def renameGlyph(self, old: str, new: str, overwrite: bool = False):
        if old != new:
            self._layer.rename_glyph(old, new, overwrite=overwrite)

    @classmethod
    def proxy(cls, obj):
        if obj is not None:
            return cls(proxy=obj)

    @property
    def name(self):
        return self._layer.name


    def __eq__(self, other):
        if other.__class__ is not self.__class__:
            return NotImplemented
        return self._layer.py_eq(other._layer)

    def __len__(self):
        return self._layer.len()

    def __iter__(self):
        return IterWrapper(Glyph, self._layer.iter_glyphs())

    def __getitem__(self, name):
        return Glyph.proxy(self._layer.glyph(name))

    def __contains__(self, name: str):
        return self._layer.contains(name)

    def get(self, name):
        return self[name]

    def newGlyph(self, name):
        return Glyph.proxy(self._layer.new_glyph(name))

    def addGlyph(self, glyph):
        print(glyph.name)
        self.insertGlyph(glyph, overwrite=False, copy=False)

    def insertGlyph(
        self,
        glyph,
        name: Optional[str] = None,
        overwrite: bool = True,
        copy: bool = True,
    ) -> None:
        if copy:
            pass
        if name is not None:
            glyph._name = name
            # print("pynorad insertGlyph doesn't respect provided names yet")
        if glyph.name is None:
            raise ValueError(f"{glyph!r} has no name; can't add it to Layer")
        if not overwrite and glyph.name in self:
            raise KeyError(f"glyph named '{glyph.name}' already exists")

        newProxyGlyph = self._layer.set_glyph(glyph._glyph)
        glyph._glyph = newProxyGlyph

class LayerSet:
    def __init__(self, layers = None, defaultLayer = None, proxy: PyFont = None):
        if proxy is not None:
            self._font = proxy
        else:
            if layers is None or len(layers) == 0:
                raise ValueError("Expected some layers or something")
            if not any(layer is defaultLayer for layer in layers.values()):
                raise ValueError(
                    f"Default layer {repr(defaultLayer)} must be in layer set."
                )
            del layers[defaultLayer.name]
            layers = [defaultLayer._layer] + [layer._layer for (name, layer) in layers]
            self._font = PyFont.from_layers(layers)


    @classmethod
    def default(cls):
        return LayerSet.proxy(PyFont())

    @classmethod
    def proxy(cls, font: PyFont):
        if font is not None:
            return LayerSet(proxy=font)

    @classmethod
    def from_iterable(
        cls, value: Iterable[Layer], defaultLayerName: str = DEFAULT_LAYER_NAME
    ) -> "LayerSet":
        """Instantiates a LayerSet from an iterable of :class:`.Layer` objects.

        Args:
            value: an iterable of :class:`.Layer` objects.
            defaultLayerName: the name of the default layer of the ones in ``value``.
        """
        layers: OrderedDict[str, Layer] = OrderedDict()
        defaultLayer = None
        for layer in value:
            if not isinstance(layer, Layer):
                raise TypeError(f"expected 'Layer', found '{type(layer).__name__}'")
            if layer.name in layers:
                raise KeyError(f"duplicate layer name: '{layer.name}'")
            if layer.name == defaultLayerName:
                defaultLayer = layer
            layers[layer.name] = layer

        if defaultLayerName not in layers:
            raise ValueError(f"expected one layer named '{defaultLayerName}'.")
        assert defaultLayer is not None

        this = cls(layers=layers, defaultLayer=defaultLayer)
        assert this._font is not None

        return this

    def __iter__(self):
        return IterWrapper(Layer, self._font.iter_layers())

    def __len__(self):
        return self._font.layer_count()

    def __eq__(self, other):
        if other.__class__ is not self.__class__:
            return NotImplemented
        return self._font.layer_eq(other._font)

    def __contains__(self, layer):
        return self._font.contains(layer)


    def __getitem__(self, name):
        return Layer.proxy(self._font.get_layer(name))

    def newLayer(self, name, **kwargs):
        return Layer.proxy(self._font.new_layer(name))

    def renameLayer(self, old, new, overwrite = False):
        self._font.rename_layer(old, new, overwrite)

    def keys(self):
        return self._font.layer_names()

    @property
    def defaultLayer(self):
        return Layer.proxy(self._font.default_layer())

    @property
    def layerOrder(self):
        return self._font.layer_order()

class IterWrapper:
    def __init__(self, typ, inner):
        self.inner = inner
        self.typ = typ

    def __iter__(self):
        return self

    def __next__(self):
        nxt = next(self.inner)
        if nxt is not None:
            return self.typ.proxy(nxt)
        else:
            return None

# class ufoLib2.objects.Glyph(name: Optional[str] = None, width: float = 0, height: float = 0, unicodes: List[int] = NOTHING, image: ufoLib2.objects.image.Image = NOTHING, lib: Dict[str, Any] = NOTHING, note: Optional[str] = None, anchors: List[ufoLib2.objects.anchor.Anchor] = NOTHING, components: List[ufoLib2.objects.component.Component] = NOTHING, contours: List[ufoLib2.objects.contour.Contour] = NOTHING, guidelines: List[ufoLib2.objects.guideline.Guideline] = NOTHING)[source]
class Glyph:
    def __init__(self, name: str = "", proxy: PyGlyph = None, **kwargs):
        self.lib = {}
        if proxy is not None:
            self._glyph = proxy
        else:
            self._glyph = PyGlyph.concrete(name)
        # self._glyph = obj

    @classmethod
    def proxy(cls, obj: PyGlyph):
        if obj is not None:
            return cls(proxy = obj)

    def __eq__(self, other):
        if other.__class__ is not self.__class__:
            return NotImplemented
        return self._glyph.py_eq(other._glyph)

    @property
    def width(self):
        return self._glyph.width

    @property
    def height(self):
        return self._glyph.height

    @property
    def contours(self):
        return self._glyph.contours

    @property
    def components(self):
        return []

    @property
    def name(self):
        return self._glyph.name

    # FIXME: delete these two, use a proxy object ancestor or something
    @property
    def _name(self):
        return self._glyph.name

    @_name.setter
    def _name(self, value: str):
        self._glyph._name = value

    def draw(self, pen):
        pointPen = PointToSegmentPen(pen)
        self._glyph.drawPoints(pointPen)

    def getPointPen(self):
        """Returns a point pen for others to draw points into self."""
        pointPen = GlyphPointPen(self._glyph.point_pen())
        return pointPen

    def getPen(self):
        pen = SegmentToPointPen(self.getPointPen())
        return pen


class GlyphPointPen:
    def __init__(self, proxy: PyPointPen):
        self._obj = proxy

    def beginPath(self, identifier: Optional[str] = None, **kwargs: Any) -> None:
        self._obj.begin_path(identifier)


    def endPath(self) -> None:
        self._obj.end_path()


    def addPoint(
        self,
        pt: Tuple[float, float],
        segmentType: Optional[str] = None,
        smooth: bool = False,
        name: Optional[str] = None,
        identifier: Optional[str] = None,
        **kwargs: Any,
    ) -> None:
        segmentType = encodeSegmentType(segmentType)
        self._obj.add_point(pt, segmentType, smooth, name, identifier)


    def addComponent(
        self,
        baseGlyph: str,
        transformation: Transform,
        identifier: Optional[str] = None,
        **kwargs: Any,
    ) -> None:
        tx = transformation
        transform = (tx.xx, tx.xy, tx.yx, tx.yy, tx.dx, tx.dy)
        self._obj.add_component(baseGlyph, transform, identifier)


def encodeSegmentType(segmentType: Optional[str]) -> int:
    """
    Jumping through hoops to avoid sending a string across the FFI
    boundary. The ordering of points is the ordering in the spec.
    """
    if segmentType == "move":
        return 0
    if segmentType == "line":
        return 1
    if segmentType is None:
        return 2
    if segmentType == "curve":
        return 3
    if segmentType == "qcurve":
        return 4
    raise ValueError(f"Unknown segment type {segmentType}")

class Guideline:
    """I'll do something at some point"""
    def __init__(self, x=None, y=None, angle=None, name=None, color=None, identifier=None, proxy=None):
        if proxy is not None:
            self._guideline = proxy
        else:
            self._guideline = PyGuideline.concrete(x, y, angle, name, color, identifier)

    @classmethod
    def proxy(cls, obj: PyGuideline):
        return cls(proxy=obj)

    @classmethod
    def normalize(cls, obj):
        """Given a Guideline or a dict that looks like a Guideline,
        return a Guideline."""
        if obj.__class__ is Guideline:
            return obj
        else:
            return Guideline(**obj)

    def __eq__(self, other):
        if other.__class__ is not self.__class__:
            return NotImplemented
        return self._guideline.py_eq(other._guideline)

# class Contours:
    # @classmethod
    # def proxy(cls, obj):
        # assert obj.__class__ == GlyphProxy
        # self._proxy = obj

class FontInfo(object):
    """I'll do something at some point"""
    def __init__(self, proxy=None):
        if proxy is not None:
            object.__setattr__(self, "_info", proxy)
        else:
            pass

    @classmethod
    def proxy(cls, obj: PyFontInfo):
        return cls(proxy=obj)

    def __getattr__(self, item):
        real = object.__getattribute__(self, "_info")
        if hasattr(real, item):
            return getattr(real, item)
        raise AttributeError(item)

    def __setattr__(self, name, item):
        real = object.__getattribute__(self, "_info")
        if hasattr(real, name):
            return setattr(real, name, item)
        raise AttributeError(item)

    # def __eq__(self, other):
        # if other.__class__ is not self.__class__:
            # return NotImplemented
        # return self._guideline.py_eq(other._guideline)

