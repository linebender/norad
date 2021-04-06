from ufoLib2.objects import Anchor, Font, Guideline


def test_object_lib_roundtrip(tmp_path):
    ufo = Font()

    ufo.info.guidelines = [Guideline(x=100), Guideline(y=200)]
    guideline_lib = ufo.objectLib(ufo.info.guidelines[1])
    guideline_lib["com.test.foo"] = 1234

    ufo.newGlyph("component")
    glyph = ufo.newGlyph("test")

    glyph.guidelines = [Guideline(x=300), Guideline(y=400)]
    glyph_guideline_lib = glyph.objectLib(glyph.guidelines[1])
    glyph_guideline_lib["com.test.foo"] = 4321

    glyph.anchors = [Anchor(x=1, y=2, name="top"), Anchor(x=3, y=4, name="bottom")]
    anchor_lib = glyph.objectLib(glyph.anchors[1])
    anchor_lib["com.test.anchorTool"] = True

    pen = glyph.getPen()
    pen.moveTo((0, 0))
    pen.lineTo((100, 200))
    pen.lineTo((200, 400))
    pen.closePath()
    pen.moveTo((1000, 1000))
    pen.lineTo((1000, 2000))
    pen.lineTo((2000, 4000))
    pen.closePath()
    pen.addComponent("component", (1, 0, 0, 1, 0, 0))
    pen.addComponent("component", (1, 0, 0, 1, 0, 0))

    contour_lib = glyph.objectLib(glyph.contours[0])
    contour_lib["com.test.foo"] = "abc"
    point_lib = glyph.objectLib(glyph.contours[1].points[0])
    point_lib["com.test.foo"] = "abc"
    component_lib = glyph.objectLib(glyph.components[0])
    component_lib["com.test.foo"] = "abc"

    ufo.save(tmp_path / "test.ufo")

    # Roundtrip
    ufo_reload = Font.open(tmp_path / "test.ufo")

    reload_guideline_lib = ufo_reload.objectLib(ufo_reload.info.guidelines[1])
    reload_glyph = ufo_reload["test"]
    reload_glyph_guideline_lib = reload_glyph.objectLib(reload_glyph.guidelines[1])
    reload_anchor_lib = reload_glyph.objectLib(reload_glyph.anchors[1])
    reload_contour_lib = reload_glyph.objectLib(reload_glyph.contours[0])
    reload_point_lib = reload_glyph.objectLib(reload_glyph.contours[1].points[0])
    reload_component_lib = reload_glyph.objectLib(reload_glyph.components[0])

    assert reload_guideline_lib == guideline_lib
    assert reload_glyph_guideline_lib == glyph_guideline_lib
    assert reload_anchor_lib == anchor_lib
    assert reload_contour_lib == contour_lib
    assert reload_point_lib == point_lib
    assert reload_component_lib == component_lib


def test_object_lib_prune(tmp_path):
    ufo = Font()

    ufo.info.guidelines = [Guideline(x=100), Guideline(y=200)]
    _ = ufo.objectLib(ufo.info.guidelines[0])
    guideline_lib = ufo.objectLib(ufo.info.guidelines[1])
    guideline_lib["com.test.foo"] = 1234
    ufo.lib["public.objectLibs"]["aaaa"] = {"1": 1}

    ufo.newGlyph("component")
    glyph = ufo.newGlyph("test")

    glyph.guidelines = [Guideline(x=300), Guideline(y=400)]
    _ = glyph.objectLib(glyph.guidelines[0])
    glyph_guideline_lib = glyph.objectLib(glyph.guidelines[1])
    glyph_guideline_lib["com.test.foo"] = 4321
    glyph.lib["public.objectLibs"]["aaaa"] = {"1": 1}

    ufo.save(tmp_path / "test.ufo")

    # Roundtrip
    ufo_reload = Font.open(tmp_path / "test.ufo")
    assert set(ufo_reload.lib["public.objectLibs"].keys()) == {
        ufo.info.guidelines[1].identifier
    }
    assert set(ufo_reload["test"].lib["public.objectLibs"].keys()) == {
        glyph.guidelines[1].identifier
    }

    # Empty object libs are pruned from objectLibs, but the identifiers stay.
    assert ufo.info.guidelines[0].identifier is not None
    assert glyph.guidelines[0].identifier is not None
