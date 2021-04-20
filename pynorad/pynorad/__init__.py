from typing import Iterable, OrderedDict
from .pynorad import PyFont, PyLayer, GlyphProxy

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
        # result._path = self._path
        result._font = self._font.deep_copy()
        return result

    def __getitem__(self, name):
        return self._font.default_layer().glyph(name)

    def __iter__(self):
        return self._font.default_layer().iter_glyphs()


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

    @classmethod
    def proxy(cls, obj):
        return Layer(proxy=obj)

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
        return self._layer.iter_glyphs()

    def __getitem__(self, name):
        return self._layer.glyph(name)



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
        return LayerIter(self._font.iter_layers())

    def __len__(self):
        return self._font.layer_count()

    def __eq__(self, other):
        if other.__class__ is not self.__class__:
            return NotImplemented
        return self._font.layer_eq(other._font)

    def __contains__(self, layer):
        print("contains", layer)
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

class LayerIter:
    def __init__(self, inner):
        self.inner = inner

    def __iter__(self):
        return self

    def __next__(self):
        nxt = next(self.inner)
        if nxt is not None:
            return Layer.proxy(nxt)
        else:
            return None

class Glyph:
    def __init__(self, obj):
        assert obj.__class__ == GlyphProxy
        self._glyph = obj

    # def __eq__(self, other):
        # if other.__class__ is not self.__class__:
            # return NotImplemented
        # return self._layer.py_eq(other._layer)

    @property
    def contours(self):
        return self._glyph.contours

    @property
    def name(self):
        return self._glyph.name

class Guideline:
    """I'll do something at some point"""

# class Contours:
    # @classmethod
    # def proxy(cls, obj):
        # assert obj.__class__ == GlyphProxy
        # self._proxy = obj

