"""M6 physical-device acceptance. Uses the host UI and Android confirmation dialogs.

Run --instrument first, then --verify and --ui. ADB is the test driver/oracle;
the generated-app compilation, packaging and signing happen in the host UID.
"""
import argparse
import json
import os
from pathlib import Path
import re
import subprocess
import time
import xml.etree.ElementTree as ET

ROOT = Path(__file__).resolve().parents[2]
OUT = Path(os.environ.get("AIC_ACCEPTANCE_DIR", ROOT / "compiler/testdata/generated/m6"))
OUT.mkdir(parents=True, exist_ok=True)
SDK = Path(os.environ["ANDROID_SDK_ROOT"])
ADB = SDK / "platform-tools/adb.exe"
serials = [line.split()[0] for line in subprocess.check_output([str(ADB), "devices"], text=True).splitlines()[1:] if "\tdevice" in line]
SERIAL = os.environ.get("ANDROID_SERIAL")
if SERIAL:
    assert SERIAL in serials, "Selected ANDROID_SERIAL is not an authorized connected device"
else:
    assert len(serials) == 1, "Connect exactly one authorized reference device or set ANDROID_SERIAL"
    SERIAL = serials[0]

def adb(*args, binary=False):
    result = subprocess.run([str(ADB), "-s", SERIAL, *args], capture_output=True, check=True)
    return result.stdout if binary else result.stdout.decode("utf-8", errors="replace")

def ui():
    last_error = None
    for _ in range(5):
        try:
            adb("shell", "uiautomator", "dump", "/sdcard/aic-m6-ui.xml")
            text = adb("shell", "cat", "/sdcard/aic-m6-ui.xml")
            nodes = list(ET.fromstring(text).iter("node"))
            (OUT / "last-ui.xml").write_text(text, encoding="utf-8")
            return nodes
        except (subprocess.CalledProcessError, ET.ParseError) as error:
            last_error = error
            time.sleep(0.5)
    raise AssertionError(f"Android UI hierarchy unavailable after retries: {last_error}")

def tap_node(node):
    x1, y1, x2, y2 = map(int, re.findall(r"\d+", node.attrib["bounds"]))
    assert node.get("enabled") == "true", node.attrib
    # Android tablet caption handles occupy the horizontal center of the first
    # full-width input. Tap within its text area, away from system window chrome.
    x = x1 + min(80, (x2-x1)//2) if node.get("class") == "android.widget.EditText" else (x1+x2)//2
    adb("shell", "input", "tap", str(x), str((y1+y2)//2))

def tap(text, package=None):
    for _ in range(5):
        for node in ui():
            if node.get("text", "").casefold() == text.casefold() and (package is None or node.get("package") == package):
                tap_node(node)
                return
    raise AssertionError("Control not found: " + text)

def tap_list_item(text, package=None):
    """Find an item in a platform ListView, scrolling through long project lists."""
    previous = None
    for _ in range(60):
        nodes = ui()
        for node in nodes:
            if node.get("text", "").casefold() == text.casefold() and (package is None or node.get("package") == package):
                tap_node(node)
                return
        scrollable = next((node for node in nodes if node.get("scrollable") == "true"), None)
        if scrollable is None:
            break
        visible_items = tuple((node.get("text"), node.get("content-desc")) for node in nodes
            if node.get("text") or node.get("content-desc"))
        if visible_items == previous:
            break
        previous = visible_items
        x1, y1, x2, y2 = map(int, re.findall(r"\d+", scrollable.attrib["bounds"]))
        adb("shell", "input", "swipe", str((x1+x2)//2), str(y1+(y2-y1)*3//4), str((x1+x2)//2), str(y1+(y2-y1)//4), "250")
        time.sleep(0.25)
    raise AssertionError("List item not found after scrolling: " + text)

def visible(text):
    for _ in range(5):
        if any(node.get("text") == text for node in ui()):
            return
    raise AssertionError("Expected text: " + text)

def prefs(name):
    last_error = None
    for _ in range(5):
        try:
            text = adb("shell", "run-as", "dev.aic.host", "cat", f"shared_prefs/{name}.xml")
            return {node.get("name"): node.get("value", node.text or "") for node in ET.fromstring(text)}
        except (subprocess.CalledProcessError, ET.ParseError) as error:
            last_error = error
            time.sleep(0.25)
    raise AssertionError(f"Host preferences unavailable after retries: {name}: {last_error}")

def host():
    adb("shell", "am", "start", "-W", "-f", "0x10008000", "-n", "dev.aic.host/.MainActivity")

def wait_install():
    for _ in range(40):
        state = prefs("installer")
        if state.get("session") == "-1":
            assert state.get("installed") == "true", state
            return
        nodes = ui()
        # Only normal platform installation confirmation is automated. Security
        # settings and optional external scan submissions remain explicit UI steps.
        for node in nodes:
            if node.get("text") in ("Install", "Update") and "packageinstaller" in node.get("package", ""):
                tap_node(node)
                break
        else:
            time.sleep(0.5)
    raise AssertionError("Installation needs attention on the device; see last-ui.xml")

def wait_install_started():
    for _ in range(20):
        if prefs("installer").get("session") != "-1":
            return
        time.sleep(0.25)
    raise AssertionError("Install action did not create a PackageInstaller session")

def choose(name):
    host()
    tap_list_item("Projects", "dev.aic.host")
    tap_list_item(name, "dev.aic.host")
    previous_result = prefs("build").get("result")
    tap_list_item("Build", "dev.aic.host")
    observed_running = False
    for _ in range(120):
        state = prefs("build")
        observed_running = observed_running or state.get("running") == "true"
        result = state.get("result")
        if state.get("running") == "false" and result and (observed_running or result != previous_result):
            # SharedPreferences can be committed before MainActivity processes
            # BuildController.changed and refreshes the button states.
            for _ in range(20):
                install = next((node for node in ui()
                    if node.get("text") == "Install" and node.get("package") == "dev.aic.host"), None)
                if install is not None and install.get("enabled") == "true":
                    return
                time.sleep(0.25)
            raise AssertionError("Build completed but Install did not become enabled")
        time.sleep(0.25)
    raise AssertionError("Host build timed out")

def install_and_launch():
    tap_list_item("Install", "dev.aic.host")
    wait_install_started()
    wait_install()
    host()
    tap_list_item("Launch", "dev.aic.host")

def instrument():
    for apk in ("host/app/build/outputs/apk/debug/app-debug.apk", "host/app/build/outputs/apk/androidTest/debug/app-debug-androidTest.apk"):
        result = adb("install", "-r", "-t", "-g", str(ROOT / apk))
        assert "Success" in result, result
    result = adb("shell", "am", "instrument", "-w", "dev.aic.host.test/dev.aic.host.HostInstrumentation")
    (OUT / "instrumentation.txt").write_text(result, encoding="utf-8")
    # Keep the historical M6 driver usable with the current host suite. The
    # marker advances when the instrumentation contract gains milestone tests.
    assert any(marker in result for marker in (
        "M6_INSTRUMENTATION_PASS",
        "M7_INSTRUMENTATION_PASS",
        "M8_INSTRUMENTATION_PASS",
    )), result
    print(result, flush=True)

def verify():
    tools = SDK / "build-tools/35.0.0"
    cargo = Path(os.environ["USERPROFILE"]) / ".cargo/bin/cargo.exe"
    results = []
    for name in ("hello", "counter", "calculator", "notes", "optimizer"):
        for level in (0, 1):
            folder = f"{name}-o{level}"
            directory = OUT / folder
            directory.mkdir(exist_ok=True)
            for artifact in ("source.aic", "classes.dex", "signed.apk", "unsigned.apk", "AndroidManifest.xml", "AndroidManifest.axml", "tool-invocations.log"):
                (directory / artifact).write_bytes(adb("exec-out", "run-as", "dev.aic.host", "cat", f"files/acceptance/{folder}/{artifact}", binary=True))
            subprocess.run([str(cargo), "run", "--manifest-path", str(ROOT / "compiler/Cargo.toml"), "-p", "aic-cli", "--", "compile", "--input", str(directory / "source.aic"), "--output-dir", str(directory / "desktop"), "--profile", "android-35", "--opt-level", str(level)], check=True, capture_output=True)
            assert (directory / "classes.dex").read_bytes() == (directory / "desktop/classes.dex").read_bytes()
            assert (directory / "AndroidManifest.xml").read_bytes() == (directory / "desktop/AndroidManifest.xml").read_bytes()
            for artifact in ("AndroidManifest.axml", "unsigned.apk"):
                assert (directory / artifact).read_bytes() == (directory / "desktop" / artifact).read_bytes()
            assert "bundled:" not in (directory / "tool-invocations.log").read_text()
            assert "<uses-permission" not in (directory / "AndroidManifest.xml").read_text()
            commands = [([str(tools / "apksigner.bat"), "verify", "--verbose", str(directory / "signed.apk")], "signature.txt"),
                        ([str(tools / "zipalign.exe"), "-c", "-v", "4", str(directory / "signed.apk")], "alignment.txt"),
                        ([str(tools / "dexdump.exe"), "-f", str(directory / "classes.dex")], "dexdump.txt")]
            for command, output in commands:
                result = subprocess.run(command, capture_output=True, check=True)
                (directory / output).write_bytes(result.stdout + result.stderr)
            results.append({"fixture":name, "optLevel":level, "desktopParity":True, "apkVerified":True, "permissions":[]})
    (OUT / "verification.json").write_text(json.dumps(results, indent=2))
    print("PASS: ten on-device APKs independently verified; desktop/JNI DEX and manifest parity", flush=True)

def ui_scenarios():
    results = []
    started = adb("shell", "date", r"+%m-%d\ %H:%M:%S.000").strip()
    host()
    if prefs("installer").get("session") != "-1":
        tap_list_item("Cancel install", "dev.aic.host")
        for _ in range(20):
            if prefs("installer").get("session") == "-1":
                break
            time.sleep(0.25)
        else:
            raise AssertionError("Interrupted installation session could not be cancelled")
        results.append("interrupted installation session recovered")
    choose("M6 counter O1")
    tap_list_item("Install", "dev.aic.host")
    wait_install_started()
    tap("Cancel")
    for _ in range(20):
        if prefs("installer").get("session") == "-1": break
        time.sleep(0.25)
    cancelled = prefs("installer")
    assert cancelled.get("installed") == "false" and cancelled.get("status") != "Installed successfully", cancelled
    results.append("platform installation cancellation reported")
    host()
    install_and_launch()
    tap("Increment"); visible("Count: 1")
    tap("Reset"); visible("Count: 0")
    results.append("counter built, approved, launched and interacted")
    choose("M6 counter O1"); install_and_launch()
    tap("Increment"); visible("Count: 1")
    results.append("same-key reinstall succeeded")
    choose("M6 calculator O1"); install_and_launch()
    tap("First integer"); adb("shell", "input", "text", "7")
    tap("Second integer"); adb("shell", "input", "text", "3")
    tap("Add"); visible("10")
    tap("Multiply"); visible("21")
    results.append("calculator built, approved, launched and computed")
    choose("M6 notes O1"); install_and_launch()
    tap("Title"); adb("shell", "input", "text", "M6Note")
    tap("Body"); adb("shell", "input", "text", "Persistent")
    tap("Create"); visible("Created")
    adb("shell", "am", "force-stop", "dev.aic.m6.notes")
    host(); tap_list_item("Launch", "dev.aic.host")
    tap("Read"); visible("M6Note"); visible("Persistent")
    tap("Update"); visible("Updated")
    tap("Delete"); visible("Deleted")
    tap("Read"); visible("Not found")
    results.append("notes CRUD and process-death persistence succeeded")
    crashes = adb("logcat", "-d", "-b", "crash", "-t", started)
    assert "dev.aic.host" not in crashes and "dev.aic.m6." not in crashes, crashes
    (OUT / "ui-crashes.txt").write_text(crashes, encoding="utf-8")
    (OUT / "ui-scenarios.json").write_text(json.dumps(results, indent=2))
    print("PASS: " + "; ".join(results), flush=True)
    host()

if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("--instrument", action="store_true")
    parser.add_argument("--verify", action="store_true")
    parser.add_argument("--ui", action="store_true")
    args = parser.parse_args()
    profile = {key: adb("shell", "getprop", key).strip() for key in ("ro.product.model", "ro.product.cpu.abi", "ro.build.version.release", "ro.build.fingerprint")}
    assert profile["ro.product.cpu.abi"] == "arm64-v8a", profile
    (OUT / "device-profile.json").write_text(json.dumps(profile, indent=2))
    if args.instrument: instrument()
    if args.verify: verify()
    if args.ui: ui_scenarios()
