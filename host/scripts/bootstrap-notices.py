"""Bundle the bootstrap port and AOSP dependency license notices with the host."""
from pathlib import Path
import sys
root, output = map(Path, sys.argv[1:])
parts = ["AIC M6 bootstrap dependencies\nSource: https://github.com/ReVanced/aapt2/tree/adb1d7acef11a67849b8a6204a824e271c4c1952\nLocal adaptations: host/scripts/prepare-aapt2-source.py\n"]
for base in [root, *sorted((root / "submodules").iterdir())]:
    for pattern in ("LICENSE*", "COPYING*", "NOTICE*", "src/LICENSE*", "expat/COPYING*"):
        for path in sorted(base.glob(pattern)):
            if path.is_file():
                parts.append("\n--- " + str(path.relative_to(root)) + " ---\n" + path.read_text(encoding="utf-8", errors="replace"))
parts.append("\nAOSP apksig 8.10.1: Apache License 2.0\nhttps://android.googlesource.com/platform/tools/apksig/\nAndroid platform android.jar: Android SDK terms and included AOSP notices.\n")
(output / "NOTICE.txt").write_text("\n".join(parts), encoding="utf-8")
