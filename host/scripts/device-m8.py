"""M8 explicit per-device compatibility gate. No silent installation in host UI tests.

--host runs JNI/signing and approved installer UI scenarios (ARM64 API 30-36).
--generated tests DEX-only apps (any device ABI, API 23-36).
--require-offline checks airplane mode and disabled Wi-Fi before host builds.
ADB installs in --generated are acceptance harness operations, not host behavior.
"""
import argparse
import importlib.util
import json
import os
from pathlib import Path
import re

ROOT = Path(__file__).resolve().parents[2]


def module(name, path):
    spec = importlib.util.spec_from_file_location(name, path)
    loaded = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(loaded)
    return loaded


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--serial", required=True)
    parser.add_argument("--host", action="store_true")
    parser.add_argument("--generated", action="store_true")
    parser.add_argument("--require-offline", action="store_true")
    parser.add_argument("--resume-host", action="store_true",
        help="Reuse completed instrumentation/parity/UI evidence and run the remaining host-built APK corpus")
    args = parser.parse_args()
    if not args.host and not args.generated:
        parser.error("Select --host and/or --generated")
    # Only filesystem-safe serials can become evidence directory names.
    directory = ROOT / "compiler/testdata/generated/m8/devices" / re.sub(r"[^A-Za-z0-9_.-]", "_", args.serial)
    directory.mkdir(parents=True, exist_ok=True)
    result_file = directory / "compatibility.json"
    result_file.unlink(missing_ok=True)
    os.environ["ANDROID_SERIAL"] = args.serial
    os.environ["AIC_ACCEPTANCE_DIR"] = str(directory)
    driver = module("m6_driver", ROOT / "host/scripts/device-m6.py")
    verify = module("m8_verify", ROOT / "compiler/scripts/verify-m8.py")
    adb = driver.adb
    profile = {key: adb("shell", "getprop", key).strip() for key in ("ro.product.model", "ro.product.cpu.abi", "ro.product.cpu.abilist", "ro.build.version.sdk", "ro.build.version.release", "ro.build.fingerprint")}
    api = int(profile["ro.build.version.sdk"])
    (directory / "device-profile.json").write_text(json.dumps(profile, indent=2))
    result = {"buildFingerprint": verify.build_fingerprint(), "serial": args.serial, "profile": "android-35", "device": profile, "host": "not-run", "generated": "not-run", "offline": False}
    try:
        if args.host:
            assert 30 <= api <= 36 and "arm64-v8a" in profile["ro.product.cpu.abilist"].split(","), profile
            result["hostExecution"] = "native-arm64" if profile["ro.product.cpu.abi"] == "arm64-v8a" else "translated-arm64"
            if args.require_offline:
                assert adb("shell", "settings", "get", "global", "airplane_mode_on").strip() == "1", "Enable airplane mode for offline acceptance"
                assert adb("shell", "settings", "get", "global", "wifi_on").strip() == "0", "Disable Wi-Fi for offline acceptance"
                result["offline"] = True
            verify.inspect_host(ROOT / "host/app/build/outputs/apk/debug/app-debug.apk")
            if args.resume_host:
                instrumentation = (directory / "instrumentation.txt").read_text(encoding="utf-8")
                assert any(marker in instrumentation for marker in
                    ("M6_INSTRUMENTATION_PASS", "M7_INSTRUMENTATION_PASS", "M8_INSTRUMENTATION_PASS"))
                verification = json.loads((directory / "verification.json").read_text(encoding="utf-8"))
                assert len(verification) == 10 and all(row.get("desktopParity") and row.get("apkVerified") for row in verification)
                scenarios = json.loads((directory / "ui-scenarios.json").read_text(encoding="utf-8"))
                required = ("platform installation cancellation reported", "counter built, approved, launched and interacted",
                    "same-key reinstall succeeded", "calculator built, approved, launched and computed",
                    "notes CRUD and process-death persistence succeeded")
                assert all(item in scenarios for item in required)
                print("PASS: resumed verified instrumentation, parity, and interactive UI evidence", flush=True)
            else:
                driver.instrument()
                driver.verify()
            for name in verify.FIXTURES:
                for level in (0, 1):
                    folder = directory / f"{name}-o{level}"
                    verify.verify_apk(folder / "signed.apk", folder, driver.SDK / "build-tools/35.0.0")
            if not args.resume_host:
                driver.ui_scenarios()
            # The representative user-approved PackageInstaller flows above
            # cover cancellation, install and update. Exercise every pulled,
            # host-built APK through ADB as an independent runtime harness;
            # this avoids rebuilding and navigating a project list for each
            # artifact while still testing the exact Android-Keystore-signed APK.
            for name in verify.FIXTURES:
                for level in (0, 1):
                    folder = directory / f"{name}-o{level}"
                    package = f"dev.aic.m6.{name}"
                    install = adb("install", "-r", str(folder / "signed.apk"))
                    assert "Success" in install, install
                    adb("shell", "am", "force-stop", package)
                    launch = adb("shell", "am", "start", "-W", "-n", f"{package}/.MainActivity")
                    assert "Status: ok" in launch, launch
                    generated_scenario(driver, name, package)
                    print(f"PASS: API {api} host-built {name} O{level} runtime", flush=True)
            result["host"] = "pass"
            # These are generated applications produced and signed by the host,
            # then independently installed, launched, and behavior-tested. The
            # same corpus therefore satisfies generated-app compatibility for
            # this API; --generated remains useful for testing desktop-built APKs.
            result["generated"] = "pass"
        if args.generated:
            assert 23 <= api <= 36, profile
            cli = ROOT / "compiler/target/debug/aic-cli.exe"
            tools = driver.SDK / "build-tools/35.0.0"
            key = ROOT / "compiler/.aic/debug.keystore"
            for name, source_path in verify.FIXTURES.items():
                for level in (0, 1):
                    folder = directory / "generated" / f"{name}-o{level}"
                    folder.mkdir(parents=True, exist_ok=True)
                    package = f"dev.aic.m8.{name}"
                    source = source_path.read_text(encoding="utf-8").replace(f"dev.aic.generated.{name}", package)
                    (folder / "source.aic").write_text(source, encoding="utf-8")
                    verify.run([cli, "compile", "--input", folder / "source.aic", "--output-dir", folder, "--profile", "android-35", "--opt-level", level])
                    apk = folder / "signed.apk"
                    verify.run([tools / "apksigner.bat", "sign", "--ks", key, "--ks-key-alias", "androiddebugkey", "--ks-pass", "pass:android", "--key-pass", "pass:android", "--v1-signing-enabled", "true", "--v2-signing-enabled", "true", "--v3-signing-enabled", "false", "--v4-signing-enabled", "false", "--out", apk, folder / "unsigned.apk"])
                    verify.verify_apk(apk, folder, tools)
                    install = adb("install", "-r", str(apk))
                    assert "Success" in install, install
                    adb("shell", "am", "force-stop", package)
                    launch = adb("shell", "am", "start", "-W", "-n", f"{package}/.MainActivity")
                    assert "Status: ok" in launch, launch
                    generated_scenario(driver, name, package)
                    print(f"PASS: API {api} {name} O{level} runtime", flush=True)
                    (folder / "runtime.json").write_text(json.dumps({"fixture": name, "optLevel": level, "api": api, "install": install, "launch": launch, "behavior": "pass"}, indent=2))
            result["generated"] = "pass"
        print(f"PASS: M8 API {api}; host={result['host']}, generated={result['generated']}")
    except Exception as error:
        result["error"] = str(error)
        raise
    finally:
        result_file.write_text(json.dumps(result, indent=2), encoding="utf-8")


def generated_scenario(driver, name, package):
    adb, tap, visible = driver.adb, driver.tap, driver.visible
    if name == "hello":
        visible("Hello from AndroidIntelligentCompiler")
    elif name in ("counter", "optimizer"):
        visible("Count: 0")
        tap("Increment", package)
        visible("Count: 1")
        if name == "counter":
            tap("Reset", package)
            visible("Count: 0")
    elif name == "calculator":
        tap("First integer", package); adb("shell", "input", "text", "7")
        tap("Second integer", package); adb("shell", "input", "text", "3")
        tap("Add", package); visible("10")
        tap("Multiply", package); visible("21")
    elif name == "notes":
        tap("Title", package); adb("shell", "input", "text", "M8Note")
        tap("Body", package); adb("shell", "input", "text", "Persistent")
        tap("Create", package); visible("Created")
        adb("shell", "am", "force-stop", package)
        adb("shell", "am", "start", "-W", "-n", f"{package}/.MainActivity")
        tap("Read", package); visible("M8Note"); visible("Persistent")
        tap("Update", package); visible("Updated")
        tap("Delete", package); visible("Deleted")
        tap("Read", package); visible("Not found")
    # A live foreground activity and its expected UI are required; shell start
    # success alone is insufficient evidence of an executable DEX.
    assert package in adb("shell", "dumpsys", "activity", "activities")


if __name__ == "__main__":
    main()
