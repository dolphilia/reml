#!/usr/bin/env python3

import argparse
import json
import pathlib
from typing import Any, Dict, List


def load_summary_files(run_dir: pathlib.Path) -> List[Dict[str, Any]]:
    summaries: List[Dict[str, Any]] = []
    for path in sorted(run_dir.glob("*.summary.json")):
        try:
            data = json.loads(path.read_text(encoding="utf-8"))
        except json.JSONDecodeError:
            continue
        data["_path"] = str(path)
        summaries.append(data)
    return summaries


def evaluate_case(entry: Dict[str, Any]) -> Dict[str, Any]:
    ocaml_queries = entry.get("ocaml_packrat_queries", 0)
    ocaml_hits = entry.get("ocaml_packrat_hits", 0)
    rust_queries = entry.get("rust_packrat_queries", 0)
    rust_hits = entry.get("rust_packrat_hits", 0)
    packrat_ok = (ocaml_queries == rust_queries) and (ocaml_hits == rust_hits)
    diag_ok = bool(entry.get("diag_match"))
    ast_ok = bool(entry.get("ast_match"))
    result = {
        "case": entry.get("case", "<unknown>"),
        "source": entry.get("source", ""),
        "ast_ok": ast_ok,
        "diag_ok": diag_ok,
        "packrat_ok": packrat_ok,
        "ocaml_diag": entry.get("ocaml_diag_count", 0),
        "rust_diag": entry.get("rust_diag_count", 0),
        "ocaml_packrat": f"{ocaml_queries}/{ocaml_hits}",
        "rust_packrat": f"{rust_queries}/{rust_hits}",
        "diag_delta": entry.get("rust_diag_count", 0)
        - entry.get("ocaml_diag_count", 0),
    }
    return result


def build_report(run_dir: pathlib.Path) -> Dict[str, Any]:
    summaries = load_summary_files(run_dir)
    cases = [evaluate_case(entry) for entry in summaries]
    totals = {
        "cases": len(cases),
        "ast_ok": sum(1 for case in cases if case["ast_ok"]),
        "diag_ok": sum(1 for case in cases if case["diag_ok"]),
        "packrat_ok": sum(1 for case in cases if case["packrat_ok"]),
    }
    totals["ast_fail"] = totals["cases"] - totals["ast_ok"]
    totals["diag_fail"] = totals["cases"] - totals["diag_ok"]
    totals["packrat_fail"] = totals["cases"] - totals["packrat_ok"]
    return {"run_dir": str(run_dir), "totals": totals, "cases": cases}


def format_markdown(report: Dict[str, Any]) -> str:
    totals = report["totals"]
    lines = [
        f"# Dual-write Report ({report['run_dir']})",
        "",
        f"- ケース数: {totals['cases']}",
        f"- AST 一致: {totals['ast_ok']} / {totals['cases']}",
        f"- 診断一致: {totals['diag_ok']} / {totals['cases']}",
        f"- Packrat 一致: {totals['packrat_ok']} / {totals['cases']}",
        "",
    ]
    headers = (
        "| case | AST | diagnostics | packrat | ocaml_diag | rust_diag | "
        "ocaml_packrat | rust_packrat |"
    )
    lines.append(headers)
    lines.append(
        "| --- | --- | --- | --- | --- | --- | --- | --- |"
    )
    for case in report["cases"]:
        lines.append(
            "| {case} | {ast} | {diag} | {packrat} | {ocaml_diag} | {rust_diag} | "
            "{ocaml_packrat} | {rust_packrat} |".format(
                case=case["case"],
                ast="✅" if case["ast_ok"] else "❌",
                diag="✅" if case["diag_ok"] else "❌",
                packrat="✅" if case["packrat_ok"] else "❌",
                ocaml_diag=case["ocaml_diag"],
                rust_diag=case["rust_diag"],
                ocaml_packrat=case["ocaml_packrat"],
                rust_packrat=case["rust_packrat"],
            )
        )
    lines.append("")
    failing = [
        case
        for case in report["cases"]
        if not (case["ast_ok"] and case["diag_ok"] and case["packrat_ok"])
    ]
    if failing:
        lines.append("## 差分のあるケース")
        for case in failing:
            lines.append(
                f"- `{case['case']}`: "
                f"AST={case['ast_ok']}, diag={case['diag_ok']}, "
                f"packrat={case['packrat_ok']} "
                f"(ocaml_diag={case['ocaml_diag']}, rust_diag={case['rust_diag']}, "
                f"Δdiag={case['diag_delta']})"
            )
    return "\n".join(lines) + "\n"


def main() -> None:
    parser = argparse.ArgumentParser(
        description="Dual-write summary aggregator"
    )
    parser.add_argument("run_dir", help="summary JSON を含むディレクトリ")
    parser.add_argument("--out-json", help="JSON 出力パス", default="")
    parser.add_argument("--out-md", help="Markdown 出力パス", default="")
    args = parser.parse_args()

    run_dir = pathlib.Path(args.run_dir).resolve()
    if not run_dir.exists():
        parser.error(f"run_dir が存在しません: {run_dir}")

    report = build_report(run_dir)

    if args.out_json:
        json_path = pathlib.Path(args.out_json)
        json_path.write_text(
            json.dumps(report, ensure_ascii=False, indent=2) + "\n",
            encoding="utf-8",
        )

    if args.out_md:
        md_path = pathlib.Path(args.out_md)
        md_path.write_text(format_markdown(report), encoding="utf-8")

    if not args.out_json and not args.out_md:
        print(json.dumps(report, ensure_ascii=False, indent=2))


if __name__ == "__main__":
    main()
