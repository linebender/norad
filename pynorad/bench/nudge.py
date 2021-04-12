
import copy
# from ufoLib2 import Font
from pynorad import Font


# TEST_FILE_PATH = "/Users/rofls/dev/projects/fontville/norad/pynorad/tests/data/UbuTestData.ufo"
TEST_FILE_PATH = "/Users/rofls/dev/projects/fontville/fontfiles/twenty-first/twenty-first-bold.ufo"
WRITE_PATH = "./test-out.ufo"

def main():
    font = Font.open(TEST_FILE_PATH)
    font_clone = copy.deepcopy(font)
    for glyph in font:
        if not len(glyph.contours):
            continue
        contour = glyph.contours[0]
        for point in contour.points:
            # print(point)
            # print(point.x, point.y)
            point.x += 5
            point.y -= 5

    font.save(WRITE_PATH)
    font = Font.open(WRITE_PATH)

    for glyph in font:
        if not len(glyph.contours):
            continue
        contour1 = glyph.contours[0]
        # print(glyph.name)
        contour2 = font_clone[glyph.name].contours[0]
        for (i, point1) in enumerate(contour1.points):
            point2 = contour2.points[i]
            assert point2.x < point1.x
            assert point2.y > point1.y



if __name__ == "__main__":
    main()
