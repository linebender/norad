import pathlib
import shutil

import pytest

import pynorad as ufoLib2


@pytest.fixture
def datadir(request):
    return pathlib.Path(__file__).parent / "data"


@pytest.fixture
def ufo_UbuTestData(tmp_path, datadir):
    ufo_path = tmp_path / "UbuTestData.ufo"
    shutil.copytree(datadir / "UbuTestData.ufo", ufo_path)
    return ufoLib2.Font.open(ufo_path)
