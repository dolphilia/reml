#!/usr/bin/env python3
import os
import subprocess
import sys


def normalize_text(text: str) -> str:
    return text.replace("\r\n", "\n").replace("\r", "\n")


def read_text(path: str) -> str:
    with open(path, "r", encoding="utf-8") as handle:
        return normalize_text(handle.read())


def write_text(path: str, text: str) -> None:
    os.makedirs(os.path.dirname(path), exist_ok=True)
    with open(path, "w", encoding="utf-8", newline="\n") as handle:
        handle.write(text)


def discover_reml_files(root: str) -> list[str]:
    items: list[str] = []
    for dirpath, _, filenames in os.walk(root):
        for name in filenames:
            if name.endswith(".reml"):
                items.append(os.path.join(dirpath, name))
    return sorted(items)


def main() -> int:
    if len(sys.argv) != 5:
        print(
            "usage: spec_core_runner.py <reml> <examples_root> <expected_root> <output_dir>"
        )
        return 1

    reml = sys.argv[1]
    examples_root = sys.argv[2]
    expected_root = sys.argv[3]
    output_dir = sys.argv[4]

    failures: list[str] = []
    missing_expected: list[str] = []

    for source in discover_reml_files(examples_root):
        rel = os.path.relpath(source, examples_root)
        base = os.path.splitext(rel)[0]
        expected_stdout = os.path.join(expected_root, base + ".stdout")
        expected_diag = os.path.join(expected_root, base + ".diagnostic.json")

        has_stdout = os.path.exists(expected_stdout)
        has_diag = os.path.exists(expected_diag)

        if has_stdout and has_diag:
            failures.append(f"expected conflict: {rel}")
            continue
        if not has_stdout and not has_diag:
            missing_expected.append(rel)
            continue

        bin_path = os.path.join(output_dir, "bin", base)
        obj_path = os.path.join(output_dir, "obj", base + ".o")
        if os.name == "nt":
            bin_path += ".exe"

        compile_cmd = [
            reml,
            "internal",
            "codegen",
            source,
            "--diag-json",
            "--emit-obj",
            obj_path,
        ]

        if has_stdout:
            compile_cmd += ["--emit-bin", bin_path]

        compile_result = subprocess.run(compile_cmd, capture_output=True, text=True)
        diag_output = normalize_text(compile_result.stdout)

        if has_diag:
            expected = read_text(expected_diag)
            if diag_output != expected:
                actual_path = os.path.join(output_dir, "actual", base + ".diagnostic.json")
                write_text(actual_path, diag_output)
                failures.append(f"diagnostic mismatch: {rel}")
            if compile_result.returncode == 0:
                failures.append(f"expected failure but succeeded: {rel}")
            continue

        if compile_result.returncode != 0:
            actual_path = os.path.join(output_dir, "actual", base + ".diagnostic.json")
            write_text(actual_path, diag_output)
            failures.append(f"compile failed: {rel}")
            continue

        run_result = subprocess.run([bin_path], capture_output=True, text=True)
        stdout_output = normalize_text(run_result.stdout)
        stderr_output = normalize_text(run_result.stderr)

        expected = read_text(expected_stdout)
        if stdout_output != expected:
            actual_path = os.path.join(output_dir, "actual", base + ".stdout")
            write_text(actual_path, stdout_output)
            failures.append(f"stdout mismatch: {rel}")

        if run_result.returncode != 0:
            stderr_path = os.path.join(output_dir, "actual", base + ".stderr")
            write_text(stderr_path, stderr_output)
            failures.append(f"non-zero exit: {rel} -> {run_result.returncode}")

    if missing_expected:
        for rel in missing_expected:
            sys.stderr.write(f"missing expected output: {rel}\n")
        failures.extend([f"missing expected: {rel}" for rel in missing_expected])

    if failures:
        for failure in failures:
            sys.stderr.write(f"{failure}\n")
        return 1

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
