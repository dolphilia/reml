#!/usr/bin/env python3
import os
import subprocess
import sys


def main() -> int:
    if len(sys.argv) != 4:
        print("usage: codegen_hello.py <reml> <source> <build_dir>")
        return 1

    reml = sys.argv[1]
    source = sys.argv[2]
    build_dir = sys.argv[3]

    os.makedirs(build_dir, exist_ok=True)
    exe_name = "hello_world"
    if os.name == "nt":
        exe_name += ".exe"
    exe_path = os.path.join(build_dir, exe_name)

    cmd = [reml, "internal", "codegen", source, "--emit-bin", exe_path]
    result = subprocess.run(cmd, capture_output=True, text=True)
    if result.returncode != 0:
        sys.stderr.write(result.stderr)
        sys.stderr.write(result.stdout)
        return result.returncode

    run_result = subprocess.run([exe_path])
    if run_result.returncode != 42:
        sys.stderr.write(f"unexpected exit code: {run_result.returncode}\n")
        return 1

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
