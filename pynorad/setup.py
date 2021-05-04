from setuptools import setup

version, section = None, None
with open("Cargo.toml") as cargo_toml:
    for line in cargo_toml:
        if line.startswith('['):
            section = line[1:line.index(']')]
        elif section == "package" and line.startswith("version"):
            version = line.split("=")[1].lstrip().rstrip()
            version = version.replace("\"", "")
if not version:
    raise ValueError("No package version string in `Cargo.toml`.")

with open("README.md", "r") as readme_file:
    long_description = readme_file.read()

setup(
    name="pynorad",
    version=version,
    description="Python bindings for Norad, a Rust UFO library.",
    long_description=long_description,
    long_description_content_type="text/markdown",
    url="https://github.com/linebender/norad",
    packages=["pynorad"],
    classifiers=[
        "Programming Language :: Python :: 3",
        "Programming Language :: Rust",
        "Operating System :: POSIX :: Linux",
        "Operating System :: Microsoft :: Windows :: Windows 10",
        "Operating System :: MacOS",
    ],
    python_requires='>=3.6',
)
