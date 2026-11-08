"""Run the scene-only unit tests despite the application's WASM-only renderer.
From the repository root: python src/scene/test_native.py
Uses Cargo's local dependency artifacts; no installs or downloads.
"""
import glob
import os
from pathlib import Path
import subprocess

root = Path(__file__).resolve().parents[2]
os.chdir(root)
# Cargo builds native dependencies before rejecting the WebGL-only main.
# This leaves main and Cargo.toml untouched.
built = subprocess.run(["cargo", "test", "--no-run"], capture_output=True, text=True)
if built.returncode and "from_webgl2_context" not in built.stderr:
    raise SystemExit(built.stderr)
cmd = ["rustc", "--edition=2024", "-C", "opt-level=2", "--test", "src/scene/native_tests.rs", "-L", "dependency=target/debug/deps", "-o", "target/scene-tests.exe" if os.name == "nt" else "target/scene-tests"]
for name in ["glam", "wasm_bindgen", "js_sys", "web_sys"]:
    artifacts = glob.glob(f"target/debug/deps/lib{name}-*.rlib")
    if not artifacts:
        raise SystemExit(f"Missing native artifact for {name}: {built.stderr}")
    cmd.extend(["--extern", f"{name}={max(artifacts, key=os.path.getmtime)}"])
subprocess.run(cmd, check=True)
subprocess.run([cmd[cmd.index("-o") + 1], "--nocapture"], check=True)
