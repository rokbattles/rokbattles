"""Download two immutable test files; never load, install or execute either file."""

import hashlib
import io
from pathlib import Path
import sys
import urllib.request
import zipfile

URL = "https://github.com/basil00/WinDivert/releases/download/v2.2.2/WinDivert-2.2.2-A.zip"
ARCHIVE_SIZE = 405137
ARCHIVE_SHA256 = "63cb41763bb4b20f600b6de04e991a9c2be73279e317d4d82f237b150c5f3f15"
FILES = {
    "WinDivert.dll": (47616, "c1e060ee19444a259b2162f8af0f3fe8c4428a1c6f694dce20de194ac8d7d9a2"),
    "WinDivert64.sys": (94144, "8da085332782708d8767bcace5327a6ec7283c17cfb85e40b03cd2323a90ddc2"),
}


def checked(data, size, digest):
    if len(data) != size or hashlib.sha256(data).hexdigest() != digest:
        raise ValueError("Official signature test fixture does not match immutable pin")
    return data


def stage(destination):
    with urllib.request.urlopen(URL, timeout=60) as response:
        archive = checked(response.read(ARCHIVE_SIZE + 1), ARCHIVE_SIZE, ARCHIVE_SHA256)
    destination.mkdir(parents=True, exist_ok=False)
    with zipfile.ZipFile(io.BytesIO(archive)) as zipped:
        for name, (size, digest) in FILES.items():
            member = "WinDivert-2.2.2-A/x64/" + name
            if zipped.namelist().count(member) != 1:
                raise ValueError("Missing or duplicate test fixture")
            with zipped.open(member) as source:
                data = checked(source.read(size + 1), size, digest)
            (destination / name).write_bytes(data)


if __name__ == "__main__":
    if len(sys.argv) != 2:
        raise SystemExit("Expected one disposable signature-test fixture directory")
    stage(Path(sys.argv[1]))
