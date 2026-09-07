"""Apply pinned upstream patches and make AOSP's symlink includes work on Windows."""
from pathlib import Path
import shutil
import subprocess
import sys

root = Path(sys.argv[1]).resolve()
assert (root / "patch.sh").is_file(), "Expected the pinned ReVanced AAPT2 checkout"

def git(*args, check=True):
    return subprocess.run(["git", "-C", str(root), *args], check=check, capture_output=True, text=True)

assert git("rev-parse", "HEAD").stdout.strip() == "adb1d7acef11a67849b8a6204a824e271c4c1952"
for patch in ("apktool_ibotpeaches.patch", "protobuf.patch", "32bsystem_on_armv8.patch"):
    path = str(root / "patches" / patch)
    if git("apply", "--reverse", "--check", path, check=False).returncode != 0:
        git("apply", "--check", path)
        git("apply", path)

sysprop = root / "submodules/incremental_delivery/sysprop"
(sysprop / "include").mkdir(parents=True, exist_ok=True)
shutil.copy(root / "misc/IncrementalProperties.sysprop.h", sysprop / "include")
shutil.copy(root / "misc/IncrementalProperties.sysprop.cpp", sysprop)
shutil.copy(root / "misc/platform_tools_version.h", root / "submodules/soong/cc/libbuildversion/include")
for name in ("Resources.proto", "ResourcesInternal.proto", "ApkInfo.proto"):
    path = root / "submodules/base/tools/aapt2" / name
    text = path.read_text(encoding="utf-8")
    for proto in ("Configuration.proto", "Resources.proto"):
        text = text.replace("frameworks/base/tools/aapt2/" + proto, proto)
    path.write_text(text, encoding="utf-8")

# Git on Windows can check symlinks out as files containing the target name.
# Use their pinned Git blobs to resolve targets, never arbitrary workspace paths.
links = []
for repo in (root / "submodules").iterdir():
    if not (repo / ".git").exists():
        continue
    listing = subprocess.check_output(["git", "-C", str(repo), "ls-files", "-s"], text=True)
    for line in listing.splitlines():
        if not line.startswith("120000 "):
            continue
        meta, name = line.split("\t", 1)
        path = repo / name
        target = subprocess.check_output(["git", "-C", str(repo), "cat-file", "blob", meta.split()[1]], text=True).strip()
        resolved = (path.parent / target).resolve()
        if root in resolved.parents:
            links.append((path, resolved))
for path, target in links:
    if not path.is_symlink() and target.is_file() and path.suffix in (".h", ".c", ".cpp", ".cc"):
        path.write_bytes(target.read_bytes())
for path, target in links:
    if not path.is_symlink() and path.is_file() and target.is_dir():
        path.unlink()  # Verified single placeholder file within the bootstrap checkout.
        shutil.copytree(target, path)

cmake = root / "cmake/aapt2.cmake"
cmake.write_text(cmake.read_text().replace("${SRC}/expat/lib", "${SRC}/expat/expat/lib"))
print("Pinned AAPT2 source prepared")
