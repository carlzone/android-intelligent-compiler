"""Aggregate actual M8 runtime evidence; missing/failed rows never count as passes."""
import argparse
import json
import importlib.util
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("--evidence", type=Path, default=ROOT / "compiler/testdata/generated/m8")
parser.add_argument("--require-complete", action="store_true")
args = parser.parse_args()
spec = importlib.util.spec_from_file_location("verify_m8", Path(__file__).with_name("verify-m8.py"))
verify = importlib.util.module_from_spec(spec)
spec.loader.exec_module(verify)
fingerprint = verify.build_fingerprint()
rows = [{"api": api, "profile": "android-35", "generated": "missing", "host": "not-applicable" if api < 30 else "missing", "evidence": []} for api in range(23, 37)]
for path in sorted((args.evidence / "devices").glob("*/compatibility.json")):
    report = json.loads(path.read_text(encoding="utf-8"))
    if report.get("buildFingerprint") != fingerprint:
        continue
    api = int(report["device"]["ro.build.version.sdk"])
    if report.get("profile") != "android-35" or not 23 <= api <= 36:
        continue
    row = rows[api-23]
    row["evidence"].append(str(path.relative_to(args.evidence)))
    if report.get("generated") == "pass":
        row["generated"] = "pass"
    if api >= 30 and report.get("host") == "pass" and report.get("offline") is True and report["device"]["ro.product.cpu.abi"] == "arm64-v8a":
        row["host"] = "pass"
complete = all(r["generated"] == "pass" and r["host"] in ("pass", "not-applicable") for r in rows)
args.evidence.mkdir(parents=True, exist_ok=True)
(args.evidence / "compatibility-matrix.json").write_text(json.dumps({"buildFingerprint": fingerprint, "runtimeComplete": complete, "rows": rows}, indent=2), encoding="utf-8")
for row in rows:
    print(f"API {row['api']}: generated={row['generated']}, host={row['host']}")
if args.require_complete and not complete:
    raise SystemExit("M8 runtime exit gate remains open: missing compatibility coverage")
