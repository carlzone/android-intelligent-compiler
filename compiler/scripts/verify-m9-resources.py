"""Independent M9.4 resource-table oracle; never used by production builds."""
from __future__ import annotations

import argparse
import base64
import hashlib
import os
import pathlib
import shutil
import subprocess
import zipfile


PNG = base64.b64decode(
    "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNk+A8AAQUBAScY42YAAAAASUVORK5CYII="
)


def run(command: list[str], cwd: pathlib.Path | None = None) -> str:
    result = subprocess.run(command, cwd=cwd, text=True, encoding="utf-8", errors="replace", capture_output=True)
    if result.returncode:
        raise SystemExit(f"command failed ({result.returncode}): {' '.join(command)}\n{result.stdout}{result.stderr}")
    return result.stdout + result.stderr


def sdk_tools(sdk: pathlib.Path) -> tuple[pathlib.Path, pathlib.Path]:
    aapt2 = sdk / "build-tools" / "35.0.0" / ("aapt2.exe" if os.name == "nt" else "aapt2")
    android = sdk / "platforms" / "android-35" / "android.jar"
    if not aapt2.is_file() or not android.is_file():
        raise SystemExit("M9.4 AAPT2 oracle not run: Android build-tools/platform 35 is unavailable")
    return aapt2, android


def write_inputs(root: pathlib.Path) -> tuple[pathlib.Path, pathlib.Path]:
    source = root / "resources.aic"
    source.write_text(
        '''aic_version 0.2 app "Resources" package "dev.aic.resources" {
  resources {
    string greeting = "Hello"
    string greeting locale "zh-TW" = "你好"
    color accent = "#FF445566"
    color primary = "#112233"
    image launcher project_asset "launcher.png"
    theme app primary primary accent accent
    launcher_icon launcher
  }
  activity MainActivity { on_create {
    let text = android.text_view(text: resource.string(greeting))
    android.set_text_color(view: text, color: resource.color(primary))
    let image = android.image_view(resource: resource.image(launcher))
    android.set_decorative(view: image)
    android.set_content_view(text)
  } }
}''',
        encoding="utf-8",
    )
    assets = root / "assets"
    assets.mkdir()
    (assets / "launcher.png").write_bytes(PNG)
    return source, assets


def write_oracle_resources(root: pathlib.Path) -> pathlib.Path:
    res = root / "oracle-res"
    for directory in ["values", "values-b+zh+TW", "mipmap"]:
        (res / directory).mkdir(parents=True)
    (res / "values" / "resources.xml").write_text(
        '''<resources>
<string name="greeting">Hello</string>
<color name="accent">#FF445566</color><color name="primary">#112233</color>
<style name="app" parent="android:style/Theme.Material.Light.NoActionBar">
<item name="android:colorAccent">@color/accent</item>
<item name="android:colorPrimary">@color/primary</item>
</style></resources>''', encoding="utf-8"
    )
    (res / "values-b+zh+TW" / "strings.xml").write_text(
        '<resources><string name="greeting">你好</string></resources>', encoding="utf-8"
    )
    (res / "mipmap" / "launcher.png").write_bytes(PNG)
    return res


def semantic_lines(dump: str) -> set[str]:
    wanted = ("greeting", "accent", "primary", "launcher", "style/app", "Hello", "你好")
    return {line.strip() for line in dump.splitlines() if any(value in line for value in wanted)}


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--sdk-root", default=os.environ.get("ANDROID_SDK_ROOT") or os.environ.get("ANDROID_HOME"))
    args = parser.parse_args()
    if not args.sdk_root:
        raise SystemExit("M9.4 AAPT2 oracle not run: pass --sdk-root")
    repo = pathlib.Path(__file__).resolve().parents[2]
    aapt2, android = sdk_tools(pathlib.Path(args.sdk_root))
    temporary_parent = (repo / "compiler" / "target").resolve()
    root = (temporary_parent / "m9-resource-oracle").resolve()
    if root.parent != temporary_parent:
        raise SystemExit("invalid M9.4 oracle work directory")
    if root.exists():
        shutil.rmtree(root)
    root.mkdir(parents=True)
    try:
        source, assets = write_inputs(root)
        custom = root / "custom"
        run([
            "cargo", "run", "--quiet", "--manifest-path", str(repo / "compiler" / "Cargo.toml"),
            "-p", "aic-cli", "--", "compile", "--input", str(source), "--output-dir", str(custom),
            "--profile", "android-35", "--opt-level", "1", "--assets-dir", str(assets),
        ], repo)
        res = write_oracle_resources(root)
        compiled = root / "compiled.zip"
        oracle = root / "oracle.apk"
        stable_ids = root / "stable-ids.txt"
        stable_ids.write_text(
            "dev.aic.resources:string/greeting = 0x7f010000\n"
            "dev.aic.resources:color/accent = 0x7f020000\n"
            "dev.aic.resources:color/primary = 0x7f020001\n"
            "dev.aic.resources:mipmap/launcher = 0x7f040000\n"
            "dev.aic.resources:style/app = 0x7f050000\n",
            encoding="ascii",
        )
        manifest = root / "AndroidManifest.xml"
        manifest.write_text('<manifest xmlns:android="http://schemas.android.com/apk/res/android" package="dev.aic.resources"><application android:theme="@style/app" android:icon="@mipmap/launcher" /></manifest>', encoding="utf-8")
        run([str(aapt2), "compile", "--no-crunch", "--dir", str(res), "-o", str(compiled)])
        run([str(aapt2), "link", "-o", str(oracle), "-I", str(android), "--stable-ids", str(stable_ids), "--manifest", str(manifest), str(compiled)])
        custom_dump = run([str(aapt2), "dump", "resources", str(custom / "unsigned.apk")])
        oracle_dump = run([str(aapt2), "dump", "resources", str(oracle)])
        custom_lines = semantic_lines(custom_dump)
        oracle_lines = semantic_lines(oracle_dump)
        required = ("greeting", "accent", "primary", "launcher", "app", "Hello", "你好")
        for value in required:
            if not any(value in line for line in custom_lines):
                raise SystemExit(f"custom resource dump is missing {value!r}\n{custom_dump}")
            if not any(value in line for line in oracle_lines):
                raise SystemExit(f"AAPT2 oracle dump is missing {value!r}\n{oracle_dump}")
        for fragment in [
            "0x7f010000 string/greeting", "0x7f020000 color/accent",
            "0x7f020001 color/primary", "0x7f040000 mipmap/launcher",
            "0x7f050000 style/app", '(zh-rTW) "你好"', "#ff112233",
            "0x01010433=@color/primary", "0x01010435=@color/accent",
        ]:
            if fragment not in custom_dump or fragment not in oracle_dump:
                raise SystemExit(f"resource semantic mismatch for {fragment!r}\nCUSTOM\n{custom_dump}\nORACLE\n{oracle_dump}")
        with zipfile.ZipFile(custom / "unsigned.apk") as custom_zip, zipfile.ZipFile(oracle) as oracle_zip:
            custom_hash = hashlib.sha256(custom_zip.read("res/mipmap/launcher.png")).hexdigest()
            oracle_hash = hashlib.sha256(oracle_zip.read("res/mipmap/launcher.png")).hexdigest()
            source_hash = hashlib.sha256(PNG).hexdigest()
            if custom_hash != source_hash or oracle_hash != source_hash:
                raise SystemExit("resource image payload hash mismatch")
        print("M9.4 AAPT2 semantic resource oracle passed")
    finally:
        shutil.rmtree(root, ignore_errors=True)


if __name__ == "__main__":
    main()
