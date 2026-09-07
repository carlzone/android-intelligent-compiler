"""Independent M8 desktop acceptance. SDK tools are oracles, not build dependencies."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import struct
import subprocess
import zipfile

ROOT = Path(__file__).resolve().parents[2]
DEFAULT_OUT = ROOT / "compiler/testdata/generated/m8"
FIXTURES = {n: ROOT / "compiler/testdata" / f"{n}.aic" for n in ("hello", "counter", "calculator", "notes")}
FIXTURES["optimizer"] = ROOT / "compiler/testdata/m5-optimizer.aic"


def build_fingerprint():
    """Bind acceptance to current compiler, host and test inputs, not stale rows."""
    paths = set()
    for base, patterns in [(ROOT / "compiler/crates", ("*.rs", "Cargo.toml")),
                           (ROOT / "host/app/src", ("*.kt", "*.xml", "*.aic", "*.json", "*.txt")),
                           (ROOT / "compiler/testdata", ("*.aic",)),
                           (ROOT / "compiler/scripts", ("*m8*",)),
                           (ROOT / "host/scripts", ("device-m6.py", "device-m8.py"))]:
        for pattern in patterns:
            paths.update(p for p in base.rglob(pattern) if p.is_file() and "generated" not in p.parts and "bootstrap" not in p.parts and "__pycache__" not in p.parts)
    paths.update([ROOT / "compiler/Cargo.toml", ROOT / "compiler/Cargo.lock", ROOT / "host/app/build.gradle.kts"])
    digest = hashlib.sha256()
    for path in sorted(paths):
        digest.update(path.relative_to(ROOT).as_posix().encode())
        digest.update(b"\0")
        digest.update(path.read_bytes())
        digest.update(b"\0")
    return digest.hexdigest()


def run(args, log=None, success=True):
    result = subprocess.run([str(a) for a in args], capture_output=True, encoding="utf-8", errors="replace")
    text = result.stdout + result.stderr
    if log:
        Path(log).write_text(text, encoding="utf-8")
    if success and result.returncode:
        raise RuntimeError(f"{args[0]} failed ({result.returncode}): {text}")
    if not success and result.returncode == 0:
        raise AssertionError("Tampered APK unexpectedly verified")
    return text


def normalized_tree(text):
    # AAPT2 adds build-tool metadata absent from the source manifest. Nothing
    # else (including typed values, names, order and framework IDs) is ignored.
    metadata = re.compile(r"^\s*A: (?:(?:http://schemas.android.com/apk/res/android:compileSdkVersion(?:Codename)?\(0x[0-9a-f]+\))|platformBuildVersion(?:Code|Name))=")
    return "\n".join(re.sub(r" \(line=\d+\)$", "", line) for line in text.splitlines() if not metadata.match(line))


def inspect_zip(path, dex=None, manifest=None):
    blob = Path(path).read_bytes()
    with zipfile.ZipFile(path) as archive:
        assert archive.testzip() is None
        if dex is not None:
            assert archive.namelist() == ["AndroidManifest.xml", "classes.dex"]
            assert all(i.date_time == (1980, 1, 1, 0, 0, 0) for i in archive.infolist())
            assert archive.read("classes.dex") == dex
            assert archive.read("AndroidManifest.xml") == manifest
        for info in archive.infolist():
            name, extra = struct.unpack_from("<HH", blob, info.header_offset + 26)
            if info.compress_type == zipfile.ZIP_STORED:
                assert (info.header_offset + 30 + name + extra) % 4 == 0, info.filename


def verify_apk(apk, folder, tools):
    run([tools / "zipalign.exe", "-c", "-v", "4", apk], folder / "alignment.txt")
    run([tools / "apksigner.bat", "verify", "--verbose", "--print-certs", "--min-sdk-version", "23", "--max-sdk-version", "36", apk], folder / "signature.txt")
    # Alter a stored DEX byte without repairing the signature or CRC.
    blob = bytearray(Path(apk).read_bytes())
    with zipfile.ZipFile(apk) as archive:
        info = archive.getinfo("classes.dex")
        name, extra = struct.unpack_from("<HH", blob, info.header_offset + 26)
        blob[info.header_offset + 30 + name + extra + 8] ^= 1
    tampered = folder / "tampered.apk"
    tampered.write_bytes(blob)
    run([tools / "apksigner.bat", "verify", tampered], folder / "tamper-rejection.txt", success=False)


def inspect_host(apk):
    with zipfile.ZipFile(apk) as archive:
        names = archive.namelist()
        assert "lib/arm64-v8a/libaic_jni.so" in names
        assert "assets/toolchain/NOTICE.txt" in names
        assert not any("bootstrap/" in n or n.endswith(("libaapt2.so", "android.jar")) for n in names)
    sources = list((ROOT / "host/app/src/main/java").rglob("*.kt"))
    assert not any(re.search(r"ProcessBuilder|Runtime\.getRuntime\(\)\.exec|libaapt2|bootstrap/", p.read_text(encoding="utf-8")) for p in sources)
    return {"apkSha256": hashlib.sha256(Path(apk).read_bytes()).hexdigest(), "bootstrapAbsent": True, "subprocessAbsent": True}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, default=DEFAULT_OUT)
    parser.add_argument("--host-apk", type=Path)
    args = parser.parse_args()
    out = args.output.resolve()
    out.mkdir(parents=True, exist_ok=True)
    # Never leave a stale successful report after a failed rerun.
    report = out / "desktop-verification.json"
    report.unlink(missing_ok=True)
    sdk = Path(os.environ["ANDROID_SDK_ROOT"])
    tools = sdk / "build-tools/35.0.0"
    java = Path(os.environ["JAVA_HOME"]) / "bin"
    cargo = shutil.which("cargo") or str(Path.home() / ".cargo/bin/cargo.exe")
    run([cargo, "build", "--manifest-path", ROOT / "compiler/Cargo.toml", "-p", "aic-cli", "--locked"], out / "cargo-build.txt")
    cli = ROOT / "compiler/target/debug/aic-cli.exe"
    key = ROOT / "compiler/.aic/debug.keystore"
    if not key.exists():
        key.parent.mkdir(parents=True, exist_ok=True)
        run([java / "keytool.exe", "-genkeypair", "-keystore", key, "-storepass", "android", "-keypass", "android", "-alias", "androiddebugkey", "-dname", "CN=Android Debug,O=Android,C=US", "-keyalg", "RSA", "-keysize", "2048", "-validity", "10000", "-noprompt"])
    ids = {"attr": {"theme": 16842752, "label": 16842753, "name": 16842755, "exported": 16842768, "minSdkVersion": 16843276, "targetSdkVersion": 16843376}, "style": {"Theme_Material_Light_NoActionBar": 16974401}}
    for group, fields in ids.items():
        dump = run([java / "javap.exe", "-constants", "-classpath", sdk / "platforms/android-35/android.jar", f"android.R${group}"], out / f"framework-{group}.txt")
        for name, value in fields.items():
            assert f"int {name} = {value};" in dump
    fixtures = dict(FIXTURES)
    for name, label in [("unicode", '\u7b46\u8a18 \U0001f4dd &"<>'), ("long-label", "x" * 32768)]:
        source = out / f"{name}.aic"
        source.write_text(FIXTURES["hello"].read_text(encoding="utf-8").replace('"AIC Hello"', json.dumps(label, ensure_ascii=False)), encoding="utf-8")
        fixtures[name] = source
    results = []
    for name, source in fixtures.items():
        for level in (0, 1):
            folder = out / f"{name}-o{level}"
            folder.mkdir(exist_ok=True)
            command = [cli, "compile", "--input", source, "--profile", "android-35", "--opt-level", level]
            run([*command, "--output-dir", folder], folder / "compile.txt")
            run([*command, "--output-dir", folder / "repeat"])
            for artifact in ("classes.dex", "AndroidManifest.xml", "AndroidManifest.axml", "unsigned.apk", "build-profile.txt"):
                assert (folder / artifact).read_bytes() == (folder / "repeat" / artifact).read_bytes(), artifact
            inspect_zip(folder / "unsigned.apk", (folder / "classes.dex").read_bytes(), (folder / "AndroidManifest.axml").read_bytes())
            run([tools / "aapt2.exe", "link", "--manifest", folder / "AndroidManifest.xml", "-I", sdk / "platforms/android-35/android.jar", "-o", folder / "oracle.apk"], folder / "oracle-link.txt")
            actual = run([tools / "aapt2.exe", "dump", "xmltree", folder / "unsigned.apk", "--file", "AndroidManifest.xml"], folder / "manifest-custom.txt")
            oracle = run([tools / "aapt2.exe", "dump", "xmltree", folder / "oracle.apk", "--file", "AndroidManifest.xml"], folder / "manifest-oracle.txt")
            assert normalized_tree(actual) == normalized_tree(oracle), f"Manifest mismatch: {name} O{level}"
            assert "uses-permission" not in actual
            badging = run([tools / "aapt2.exe", "dump", "badging", folder / "unsigned.apk"], folder / "badging.txt")
            assert "minSdkVersion:'23'" in badging and "targetSdkVersion:'35'" in badging
            assert "launchable-activity:" in badging
            run([tools / "dexdump.exe", "-f", folder / "classes.dex"], folder / "dexdump.txt")
            signed = folder / "signed.apk"
            run([tools / "apksigner.bat", "sign", "--ks", key, "--ks-key-alias", "androiddebugkey", "--ks-pass", "pass:android", "--key-pass", "pass:android", "--v1-signing-enabled", "true", "--v2-signing-enabled", "true", "--v3-signing-enabled", "false", "--v4-signing-enabled", "false", "--out", signed, folder / "unsigned.apk"])
            verify_apk(signed, folder, tools)
            results.append({"fixture": name, "optLevel": level, "unsignedSha256": hashlib.sha256((folder / "unsigned.apk").read_bytes()).hexdigest(), "manifestOracleParity": True, "reproducible": True, "aligned": True, "signatureApiRange": [23, 36], "tamperRejected": True})
            print(f"PASS: {name} O{level}", flush=True)
    result = {"buildFingerprint": build_fingerprint(), "profile": "android-35", "signingPath": "desktop official oracle; Android Keystore requires device evidence", "fixtures": results, "host": inspect_host(args.host_apk) if args.host_apk else None}
    report.write_text(json.dumps(result, indent=2), encoding="utf-8")
    print(f"PASS: desktop M8 acceptance; evidence: {report}")


if __name__ == "__main__":
    main()
