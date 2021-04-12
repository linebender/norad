from .pynorad import PyFont, LayerProxy, GlyphProxy

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
        return LayerSet(self._font)

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

class LayerSet:
    def __init__(self, font: PyFont):
        self._font = font

    def __iter__(self):
        return self._font.iter_layers()

    def __len__(self):
        return self._font.layer_count()

    def __eq__(self, other):
        if other.__class__ is not self.__class__:
            return NotImplemented
        return self._font.layer_eq(other._font)

    def __getitem__(self, name):
        Layer(self._font.get_layer(name))

    def newLayer(self, name, **kwargs):
        return Layer(self._font.new_layer(name))

    def keys(self):
        return self._font.layer_names()

    @property
    def defaultLayer(self):
        return Layer(self._font.default_layer())

    @property
    def layerOrder(self):
        return self._font.layer_order()


class Layer:
    def __init__(self, obj):
        assert obj.__class__ == LayerProxy
        self._layer = obj

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

