from typing import Iterable, OrderedDict, Optional, Any, Tuple, Mapping, NamedTuple, List
from copy import deepcopy

from fontTools.pens.pointPen import PointToSegmentPen, SegmentToPointPen
from fontTools.pens.boundsPen import BoundsPen, ControlBoundsPen
from fontTools.misc.transform import Transform
from fontTools.misc.arrayTools import unionRect
from .pynorad import PyFont, PyGuideline, PyPointPen, PyLayer, PyGlyph, PyPoint, PyContour, PyComponent, PyFontInfo, PyAnchor, PyImage

# I acknowledge that this is not the right way to do this
__version__ = '0.1'

DEFAULT_LAYER_NAME = "public.default"
# this is something that exists in ufoLib2; we bring it across so that we
# can modify tests as little as possible.
class Placeholder:
    """Represents a sentinel value to signal a "lazy" object hasn't been loaded yet."""

_NOT_LOADED = Placeholder()

class BoundingBox(NamedTuple):
    """Represents a bounding box as a tuple of (xMin, yMin, xMax, yMax)."""

    xMin: float
    yMin: float
    xMax: float
    yMax: float

def unionBounds(bounds1, bounds2):
    if bounds1 is None:
        return bounds2
    if bounds2 is None:
        return bounds1
    return BoundingBox(*unionRect(bounds1, bounds2))

class Bounded:
    def getBounds(self, layer=None):
        pen = BoundsPen(layer)
        pen.skipMissingComponents = False
        self.draw(pen)
        return None if pen.bounds is None else BoundingBox(*pen.bounds)

    def getControlBounds(self, layer=None):
        pen = ControlBoundsPen(layer)
        # raise 'KeyError' when a referenced component is missing from glyph set
        pen.skipMissingComponents = False
        self.draw(pen)
        return None if pen.bounds is None else BoundingBox(*pen.bounds)

    @property
    def bounds(self):
        return self.getBounds()

    @property
    def controlPointBounds(self):
        return self.getControlBounds()

class Proxy(object):
    __slots__ = ["_obj", "__weakref__"]
    def __init__(self, obj):
        object.__setattr__(self, "_obj", obj)

    def __getattr__(self, item):
        real = object.__getattribute__(self, "_obj")
        if hasattr(real, item):
            return getattr(real, item)
        raise AttributeError(item)

    # I'm not sure why I need this to be explicit but apparently I do?
    def __len__(self):
        return len(self._obj)

class ProxySetter(Proxy):
    def __init__(self, obj, ignoreItems={}):
        super().__init__(obj)
        object.__setattr__(self, "_ignore", ignoreItems)

    def __setattr__(self, name, item):
        if name == "_obj":
            object.__setattr__(self, name, item)
            return

        ignore = object.__getattribute__(self, "_ignore")
        if name in ignore:
            return super().__setattr__(name, item)

        real = object.__getattribute__(self, "_obj")
        if hasattr(real, name):
            return setattr(real, name, item)
        raise AttributeError(name)

class Anchor(ProxySetter):
    def __init__(self, x: float, y: float, name: Optional[str] = None, color: Optional[str] = None, identifier: Optional[str] = None, proxy=None):
        if proxy is None:
            proxy = PyAnchor.concrete(x, y, name, color, identifier)
        super().__init__(proxy)

    @classmethod
    def proxy(cls, obj: PyAnchor):
        return cls(0, 0, proxy=obj)

    def __eq__(self, other):
        return self._obj == other._obj

class Image(Proxy):
    def __init__(self, fileName: Optional[str] = None, transformation=None, color: Optional[str] = None, proxy=None):
        if proxy is None:
            proxy = PyImage.concrete(fileName, transformation, color)
        super().__init__(proxy)

    @classmethod
    def proxy(cls, obj):
        if obj is not None:
            return cls(proxy=obj)


class Component(Proxy, Bounded):
    def __init__(self, baseGlyph: str, transformation=None, identifier=None, proxy=None):
        if proxy is None:
            proxy = PyComponent.concrete(baseGlyph, transformation, identifier)
        super().__init__(proxy)

    @classmethod
    def proxy(cls, obj: PyComponent):
        return cls("", proxy=obj)

    @property
    def baseGlyph(self):
        return self._obj.base

    @baseGlyph.setter
    def baseGlyph(self, name: str):
        self._obj.set_base(name)

    def getBounds(self, layer=None):
        if layer is None:
            raise TypeError("layer is required to compute bounds of components")
        return super().getBounds(layer)

    def getControlBounds(self, layer=None):
        if layer is None:
            raise TypeError("layer is required to compute bounds of components")
        return super().getControlBounds(layer)

    def draw(self, pen) -> None:
        """Draws component with given pen."""
        pointPen = PointToSegmentPen(pen)
        self.drawPoints(pointPen)

    def drawPoints(self, pointPen) -> None:
        """Draws points of component with given point pen."""
        try:
            pointPen.addComponent(
                self.baseGlyph, self.transformation, identifier=self.identifier
            )
        except TypeError:
            pointPen.addComponent(self.baseGlyph, self.transformation)

class Contour(Bounded):
    def __init__(self, points=None, identifier=None, proxy=None):
        if proxy is not None:
            self._obj = proxy
        else:
            self._obj = PyContour.concrete([p._obj for p in points], identifier)

    def __eq__(self, other):
        return self._obj == other

    def __len__(self):
        return len(self.points)

    def __getitem__(self, idx):
        return self.points[idx]

    def __setitem__(self, idx, value):
        self.points[idx] = value

    def __delitem__(self, idx):
        del self.points[idx]

    def __iter__(self):
        return iter(self.points)

    @classmethod
    def proxy(cls, obj: PyGuideline):
        return cls(proxy=obj)

    @property
    def points(self):
        return  ProxySequence(Point, self._obj.points)

    def draw(self, pen):
        pointPen = PointToSegmentPen(pen)
        self._obj.drawPoints(pointPen)

class Point(ProxySetter):
    def __init__(self, x: float, y: float, segmentType: Optional[str] = None, smooth: bool = False, name: Optional[str] = None, identifier: Optional[str] = None, proxy = None):
        if proxy is None:
            typ = encodeSegmentType(segmentType)
            proxy = PyPoint.concrete(x, y, typ, smooth, name, identifier)

        super().__init__(proxy)

    @classmethod
    def proxy(cls, obj: PyPoint):
        return cls(0, 0, proxy=obj)

    def __eq__(self, other):
        return self._obj == other

class Guideline(Proxy):
    """I'll do something at some point"""
    def __init__(self, x=None, y=None, angle=None, name=None, color=None, identifier=None, proxy=None):
        if proxy is None:
            proxy = PyGuideline.concrete(x, y, angle, name, color, identifier)
        super().__init__(proxy)

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
        return self._obj.py_eq(other._obj)

class FakeDataSet(object):
    def __init__(self, data = None):
        self._data = data or dict()

    def __len__(self) -> int:
        return len(self._data)

    def __iter__(self):
        return iter(self._data)

    def __getitem__(self, fileName: str) -> bytes:
        data_object = self._data[fileName]
        # if isinstance(data_object, Placeholder):
            # data_object = self._data[fileName] = self.read_data(self._reader, fileName)
        return data_object

    def __setitem__(self, fileName: str, data: bytes) -> None:
        # should we forbid overwrite?
        self._data[fileName] = data
        # if fileName in self._scheduledForDeletion:
            # self._scheduledForDeletion.remove(fileName)

    def __delitem__(self, fileName: str) -> None:
        del self._data[fileName]
        self._scheduledForDeletion.add(fileName)

    def __eq__(self, other):
        return self._data == other._data

    @property
    def fileNames(self) -> List[str]:
        """Returns a list of filenames in the data store."""
        return list(self._data.keys())

class Features:
    """A data class representing UFO features.

    See http://unifiedfontobject.org/versions/ufo3/features.fea/.
    """

    text: str = ""
    """Holds the content of the features.fea file."""

    def __bool__(self) -> bool:
        return bool(self.text)

    def __str__(self) -> str:
        return self.text


class Font(Proxy):
    """A fontfile"""
    def __init__(self, path = None, **kwargs):
        self._path = path
        self._data = FakeDataSet()
        self._features = Features()
        self._reader = None
        self._lazy = False
        self._validate = True

        if path is None:
            super().__init__(PyFont())
        else:
            super().__init__(PyFont.load(str(path)))

    def __eq__(self, other):
        if other.__class__ is not self.__class__:
            return NotImplemented
        return self._obj.py_eq(other._obj)

    def __len__(self):
        return len(self._obj.default_layer())

    def __deepcopy__(self, memo):
        result = Font(None)
        object.__setattr__(result, "_obj", self._obj.deep_copy())
        result._data = deepcopy(self._data, memo)
        result._features = deepcopy(self._features, memo)
        return result

    def __getitem__(self, name):
        return self.layers.defaultLayer.__getitem__(name)

    def __setitem__(self, name: str, glyph):
       self.layers.defaultLayer.__setitem__(name, glyph)

    def __delitem__(self, name: str):
        self.layers.defaultLayer.__delitem__(name)

    def __contains__(self, name: str):
        return self.layers.defaultLayer.__contains__(name)


    @property
    def bounds(self) -> Optional[BoundingBox]:
        """Returns the (xMin, yMin, xMax, yMax) bounding box of the default
        layer, taking the actual contours into account.

        |defcon_compat|
        """
        return self.layers.defaultLayer.bounds

    @property
    def controlPointBounds(self) -> Optional[BoundingBox]:
        """Returns the (xMin, yMin, xMax, yMax) bounding box of the layer,
        taking only the control points into account.

        |defcon_compat|
        """
        return self.layers.defaultLayer.controlPointBounds

    @property
    def glyphOrder(self):
        return self._obj.glyph_order()

    @glyphOrder.setter
    def glyphOrder(self, value: Optional[List[str]]):
        return self._obj.set_glyph_order(value)

    @property
    def features(self):
        return self._features

    #FIXME: stub
    # @property
    # def groups(self):
        # return dict()

    # @property
    # def kerning(self):
        # return dict()

    def objectLib(self, obj):
        return self._obj.objectLib(obj._obj)

    def newLayer(self, layerName: str):
        return Layer.proxy(self._obj.new_layer(layerName))

    def renameLayer(self, old, new, overwrite = False):
        self.layers.renameLayer(old, new, overwrite)

    def keys(self):
        return self.layers.defaultLayer.keys()

    def addGlyph(self, glyph):
        Layer.proxy(self._obj.default_layer()).addGlyph(glyph)

    def appendGuideline(self, guideline):
        if guideline.__class__ is not Guideline:
            guideline = Guideline(**guideline)
        self._obj.append_guideline(guideline._obj)

    def newGlyph(self, name: str):
        return self.layers.defaultLayer.newGlyph(name)

    def renameGlyph(self, old: str, new: str, overwrite: bool = False):
        self.layers.defaultLayer.renameGlyph(old, new, overwrite=overwrite)

    def __iter__(self):
        return IterWrapper(Glyph, self._obj.default_layer().iter_glyphs())

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
        self._obj.save(str(path))

    @property
    def layers(self):
        return LayerSet.proxy(self._obj)

    @property
    def info(self):
        return FontInfo.proxy(self._obj.fontinfo())

    @property
    def guidelines(self):
        return ProxySequence(Guideline, self._obj.guidelines())

    @guidelines.setter
    def guidelines(self, value):
        self.replace_guidelines([Guideline.normalize(g)._obj for g in value])

    #FIXME: norad doesn't impl data yet
    @property
    def data(self):
        return self._data

    #FIXME: norad doesn't impl images yet
    @property
    def images(self):
        return dict()

    @property
    def path(self):
        return self._path

    def unlazify(self):
        pass


class Layer(Proxy):
    def __init__(self, name: str = 'public.default', glyphs = None, color = None, lib = None, proxy = None):
        if proxy is not None:
            assert proxy.__class__ == PyLayer
            super().__init__(proxy)
        else:
            super().__init__(PyLayer.concrete(name))
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
                        glyph.set_name(name or "")
                    elif currentName != name:
                        raise ValueError(
                            "glyph has incorrect name: "
                            f"expected '{name}', found '{glyph.name}'"
                        )
                    self.addGlyph(glyph)

    def renameGlyph(self, old: str, new: str, overwrite: bool = False):
        if old != new:
            self.rename_glyph(old, new, overwrite=overwrite)

    @classmethod
    def proxy(cls, obj):
        if obj is not None:
            return cls(proxy=obj)

    def __eq__(self, other):
        if other.__class__ is not self.__class__:
            return NotImplemented
        return self._obj.py_eq(other._obj)

    def __iter__(self):
        return IterWrapper(Glyph, self.iter_glyphs())

    def __getitem__(self, name):
        rawGlyph = self._obj.glyph(name)
        if rawGlyph is None:
            raise KeyError(f"No glyph named '{name}' in layer.")
        return Glyph.proxy(rawGlyph)

    def __contains__(self, name: str):
        return self._obj.contains(name)

    def __setitem__(self, name: str, glyph) -> None:
        if not isinstance(glyph, Glyph):
            raise TypeError(f"Expected Glyph, found {type(glyph).__name__}")
        self.insertGlyph(glyph, name, True, False)

    def get(self, name):
        return self[name]

    @property
    def bounds(self) -> Optional[BoundingBox]:
        """Returns the (xMin, yMin, xMax, yMax) bounding box of the layer,
        taking the actual contours into account.

        |defcon_compat|
        """
        bounds = None
        for glyph in self:
            bounds = unionBounds(bounds, glyph.getBounds(self))
        return bounds

    @property
    def controlPointBounds(self) -> Optional[BoundingBox]:
        """Returns the (xMin, yMin, xMax, yMax) bounding box of the layer,
        taking only the control points into account.

        |defcon_compat|
        """
        bounds = None
        for glyph in self:
            bounds = unionBounds(bounds, glyph.getControlBounds(self))
        return bounds

    def newGlyph(self, name):
        return Glyph.proxy(self.new_glyph(name))

    def addGlyph(self, glyph):
        self.insertGlyph(glyph, overwrite=False, copy=False)

    def insertGlyph(
        self,
        glyph,
        name: Optional[str] = None,
        overwrite: bool = True,
        copy: bool = True,
    ) -> None:
        self._obj.insert_glyph(glyph._obj, name, overwrite, copy)

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
            layers = [defaultLayer._obj] + [layer._obj for (name, layer) in layers]
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
        layer = self._font.get_layer(name)
        return Layer.proxy(layer)

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

class ProxySequence:
    def __init__(self, typ, inner):
        self.inner = inner
        self.typ = typ

    def __getitem__(self, idx):
        return self.typ.proxy(self.inner.__getitem__(idx))

    def __delitem__(self, idx):
        self.inner.__delitem__(idx)

    def __len__(self):
        return len(self.inner)

    def __iter__(self):
        return IterWrapper(self.typ, iter(self.inner))

    def __eq__(self, other):
        return len(self) == len(other) and all(x == y for x, y in zip(self, other))

class Glyph(Proxy, Bounded):
    def __init__(self, name: str = "", width: float = 0, height: float = 0, unicodes: List[int] = [], contours: List[Contour] = [], components: List[Component] = [], anchors: List[Anchor] = [], guidelines: List[Guideline] = [],  proxy: PyGlyph = None, **kwargs):
        if proxy is None:
            contours = [c._obj for c in contours]
            components = [c._obj for c in components]
            anchors = [c._obj for c in anchors]
            guides = [c._obj for c in guidelines]
            proxy = PyGlyph.concrete(name, width, height, unicodes, contours, components, anchors, guides)
        super().__init__(proxy)

    @classmethod
    def proxy(cls, obj: PyGlyph):
        if obj is not None:
            return cls(proxy = obj)

    def __eq__(self, other):
        if other.__class__ is not self.__class__:
            return NotImplemented
        return self._obj.py_eq(other._obj)

    def __repr__(self) -> str:
        return "<{}.{} {}at {}>".format(
            self.__class__.__module__,
            self.__class__.__name__,
            f"'{self.name}' " if self.name is not None else "",
            hex(id(self)),
        )

    def __iter__(self):
        return iter(self.contours)

    @property
    def contours(self):
        return ProxySequence(Contour, self._obj.contours)

    @contours.setter
    def contours(self, contours: List[Contour]):
        self._obj.contours = [g._obj for g in contours]

    @property
    def components(self):
        return ProxySequence(Component, self._obj.components)

    @components.setter
    def components(self, components: List[Component]):
        self._obj.components = [g._obj for g in components]

    @property
    def anchors(self):
        return ProxySequence(Anchor, self._obj.anchors)

    @anchors.setter
    def anchors(self, anchors: List[Anchor]):
        self._obj.anchors = [g._obj for g in anchors]

    @property
    def guidelines(self):
        return ProxySequence(Guideline, self._obj.guidelines)

    @guidelines.setter
    def guidelines(self, guidelines: List[Guideline]):
        self._obj.guidelines = [g._obj for g in guidelines]

    def objectLib(self, obj):
        return self._obj.objectLib(obj._obj)

    @property
    def width(self):
        return self._obj.width

    @width.setter
    def width(self, val):
        self._obj.width = val

    @property
    def height(self):
        return self._obj.height

    @height.setter
    def height(self, val):
        self._obj.height = val

    @property
    def note(self):
        return self._obj.note

    @note.setter
    def note(self, val):
        self._obj.note = val

    @property
    def image(self):
        return Image.proxy(self._obj.image)

    @image.setter
    def image(self, image):
        if image is not None:
            image = image._obj
        self._obj.image = image


    def appendAnchor(self, anchor):
        if not isinstance(anchor, Anchor):
            if not isinstance(anchor, Mapping):
                raise TypeError(
                    "Expected Anchor object or a Mapping for the ",
                    f"Anchor constructor, found {type(anchor).__name__}",
                )
            anchor = Anchor(**anchor)
        self._obj.append_anchor(anchor._obj)

    def appendContour(self, contour):
        if not isinstance(contour, Contour):
            raise TypeError(f"Expected Contour, found {type(contour).__name__}")
        self._obj.append_contour(contour._obj)

    def appendGuideline(self, guideline):
        if not isinstance(guideline, Guideline):
            if not isinstance(guideline, Mapping):
                raise TypeError(
                    "Expected Guideline object or a Mapping for the ",
                    f"Guideline constructor, found {type(guideline).__name__}",
                )
            guideline = Guideline(**guideline)
        self._obj.append_guideline(guideline._obj)

    def draw(self, pen):
        pointPen = PointToSegmentPen(pen)
        self._obj.drawPoints(pointPen)

    def getPointPen(self):
        """Returns a point pen for others to draw points into self."""
        pointPen = GlyphPointPen(self._obj.point_pen())
        return pointPen

    def getPen(self):
        pen = SegmentToPointPen(self.getPointPen())
        return pen

    @property
    def verticalOrigin(self) -> Optional[float]:
        return self._obj.verticalOrigin

    @verticalOrigin.setter
    def verticalOrigin(self, value: Optional[float]) -> None:
        self._obj.verticalOrigin = value

    def getBounds(self, layer: Optional[Layer] = None) -> Optional[BoundingBox]:
        if layer is None and self.components:
            raise TypeError("layer is required to compute bounds of components")

        return super().getBounds(layer)

    def getControlBounds(
        self, layer: Optional[Layer] = None
    ) -> Optional[BoundingBox]:
        if layer is None and self.components:
            raise TypeError("layer is required to compute bounds of components")

        return super().getControlBounds(layer)

    def getLeftMargin(self, layer: Optional[Layer] = None) -> Optional[float]:
        bounds = self.getBounds(layer)
        if bounds is None:
            return None
        return bounds.xMin

    def setLeftMargin(self, value: float, layer: Optional[Layer] = None) -> None:
        bounds = self.getBounds(layer)
        if bounds is None:
            return None
        diff = value - bounds.xMin
        if diff:
            self.width += diff
            self.move((diff, 0))

    def getRightMargin(self, layer: Optional[Layer] = None) -> Optional[float]:
        bounds = self.getBounds(layer)
        if bounds is None:
            return None
        return self.width - bounds.xMax

    def setRightMargin(self, value: float, layer: Optional[Layer] = None) -> None:
        bounds = self.getBounds(layer)
        if bounds is None:
            return None
        self.width = bounds.xMax + value

    def getBottomMargin(self, layer: Optional[Layer] = None) -> Optional[float]:
        bounds = self.getBounds(layer)
        if bounds is None:
            return None
        if self.verticalOrigin is None:
            return bounds.yMin
        else:
            return bounds.yMin - (self.verticalOrigin - self.height)

    def setBottomMargin(self, value: float, layer: Optional[Layer] = None) -> None:
        bounds = self.getBounds(layer)
        if bounds is None:
            return None
        # blindly copied from defcon Glyph._set_bottomMargin; not sure it's correct
        if self.verticalOrigin is None:
            oldValue = bounds.yMin
            self.verticalOrigin = self.height
        else:
            oldValue = bounds.yMin - (self.verticalOrigin - self.height)
        diff = value - oldValue
        if diff:
            self.height += diff

    def getTopMargin(self, layer: Optional[Layer] = None) -> Optional[float]:
        bounds = self.getBounds(layer)
        if bounds is None:
            return None
        if self.verticalOrigin is None:
            return self.height - bounds.yMax
        else:
            return self.verticalOrigin - bounds.yMax

    def setTopMargin(self, value: float, layer: Optional[Layer] = None) -> None:
        bounds = self.getBounds(layer)
        if bounds is None:
            return
        if self.verticalOrigin is None:
            oldValue = self.height - bounds.yMax
        else:
            oldValue = self.verticalOrigin - bounds.yMax
        diff = value - oldValue
        if oldValue != value:
            # Is this still correct when verticalOrigin was not previously set?
            self.verticalOrigin = bounds.yMax + value
            self.height += diff

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
        self._obj.add_component(baseGlyph, transformation, identifier)

class FontInfo(ProxySetter):
    """I'll do something at some point"""
    def __init__(self, proxy=None):
        if proxy is None:
            proxy = PyFontInfo.concrete()
        super().__init__(proxy, {"guidelines"})

    def __getattr__(self, item):
        real = object.__getattribute__(self, "_obj")
        if hasattr(real, item):
            return getattr(real, item)
        return None# AttributeError(item)

    @classmethod
    def proxy(cls, obj: PyFontInfo):
        return cls(proxy=obj)

    @property
    def guidelines(self):
        return ProxySequence(Guideline, self._obj.guidelines)

    @guidelines.setter
    def guidelines(self, value):
        self._obj.guidelines = [Guideline.normalize(g)._obj for g in value]


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

def getBounds(drawable, layer: Optional[Layer]) -> Optional[BoundingBox]:
    pen = BoundsPen(layer)
    # raise 'KeyError' when a referenced component is missing from glyph set
    pen.skipMissingComponents = False
    drawable.draw(pen)
    return None if pen.bounds is None else BoundingBox(*pen.bounds)

def getControlBounds(
    drawable, layer: Optional[Layer]
) -> Optional[BoundingBox]:
    pen = ControlBoundsPen(layer)
    # raise 'KeyError' when a referenced component is missing from glyph set
    pen.skipMissingComponents = False
    drawable.draw(pen)
    return None if pen.bounds is None else BoundingBox(*pen.bounds)
