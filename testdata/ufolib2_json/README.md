# ufoLib2 JSON fixtures

JSON dumps (`Font.json_dumps(indent=2)`) of norad's test UFOs, plus `kitchen_sink.ufo` and
`kitchen_sink.json`, a synthetic font that exercises every feature of the format. They are
used by `tests/ufolib2_json.rs`, which loads each `.json` and the matching `.ufo` and
compares the results.

Regenerate with `python testdata/ufolib2_json/generate.py` (requires `ufoLib2[json]`).
