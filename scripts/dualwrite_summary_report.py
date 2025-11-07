#!/usr/bin/env python3

import argparse
import json
import pathlib
from typing import Any, Dict, List, Sequence


def load_summary_files(run_dir: pathlib.Path) -> List[Dict[str, Any]]:
    summaries: List[Dict[str, Any]] = []
    summary_paths: Sequence[pathlib.Path] = sorted(run_dir.glob("*.summary.json"))
    if not summary_paths:
        summary_paths = sorted(run_dir.glob("*/summary.json"))
    for path in summary_paths:
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


def _metric_value(metrics: Any, key: str) -> str:
    if isinstance(metrics, dict):
        value = metrics.get(key)
        return str(value) if value is not None else "-"
    return "-"


def build_typeck_rows(summaries: Sequence[Dict[str, Any]]) -> List[Dict[str, Any]]:
    rows: List[Dict[str, Any]] = []
    for entry in summaries:
        metrics = entry.get("typeck_metrics")
        if not isinstance(metrics, dict):
            continue
        rows.append(
            {
                "case": entry.get("case", "<unknown>"),
                "match": metrics.get("match"),
                "typed_functions": (
                    _metric_value(metrics.get("ocaml"), "typed_functions"),
                    _metric_value(metrics.get("rust"), "typed_functions"),
                ),
                "constraints": (
                    _metric_value(metrics.get("ocaml"), "constraints_total"),
                    _metric_value(metrics.get("rust"), "constraints_total"),
                ),
                "diagnostics": (
                    entry.get("ocaml_diag_count", "-"),
                    entry.get("rust_diag_count", "-"),
                ),
            }
        )
    rows.sort(key=lambda row: row["case"])
    return rows


def format_typeck_table(rows: Sequence[Dict[str, Any]]) -> str:
    lines = [
        "| case | typeck_match | typed_functions (ocaml/rust) | "
        "constraints_total (ocaml/rust) | diagnostics (ocaml/rust) |",
        "| --- | --- | --- | --- | --- |",
    ]
    for row in rows:
        lines.append(
            "| {case} | {match} | {typed_ocaml} / {typed_rust} | "
            "{cons_ocaml} / {cons_rust} | {diag_ocaml} / {diag_rust} |".format(
                case=row["case"],
                match=row["match"],
                typed_ocaml=row["typed_functions"][0],
                typed_rust=row["typed_functions"][1],
                cons_ocaml=row["constraints"][0],
                cons_rust=row["constraints"][1],
                diag_ocaml=row["diagnostics"][0],
                diag_rust=row["diagnostics"][1],
            )
        )
    return "\n".join(lines) + "\n"


def update_typeck_readme(readme_path: pathlib.Path, table_markdown: str) -> None:
    start_marker = "<!-- TYPECK_TABLE_START -->"
    end_marker = "<!-- TYPECK_TABLE_END -->"
    content = readme_path.read_text(encoding="utf-8")
    if start_marker not in content or end_marker not in content:
        raise ValueError(
            f"{readme_path} に typeck テーブル用マーカーが見つかりません。"
        )
    start_idx = content.index(start_marker) + len(start_marker)
    end_idx = content.index(end_marker)
    new_content = (
        content[:start_idx].rstrip()
        + "\n\n"
        + table_markdown.rstrip()
        + "\n\n"
        + content[end_idx:]
    )
    readme_path.write_text(new_content, encoding="utf-8")


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
    parser.add_argument(
        "--typeck-table",
        help="Typeck サマリ表を Markdown で出力するパス",
        default="",
    )
    parser.add_argument(
        "--update-typeck-readme",
        help="Typeck サマリ表を埋め込む README パス",
        default="",
    )
    args = parser.parse_args()

    run_dir = pathlib.Path(args.run_dir).resolve()
    if not run_dir.exists():
        parser.error(f"run_dir が存在しません: {run_dir}")

    report = build_report(run_dir)
    summaries = load_summary_files(run_dir)

    if args.out_json:
        json_path = pathlib.Path(args.out_json)
        json_path.write_text(
            json.dumps(report, ensure_ascii=False, indent=2) + "\n",
            encoding="utf-8",
        )

    if args.out_md:
        md_path = pathlib.Path(args.out_md)
        md_path.write_text(format_markdown(report), encoding="utf-8")

    needs_typeck_table = bool(args.typeck_table or args.update_typeck_readme)
    if needs_typeck_table:
        rows = build_typeck_rows(summaries)
        if not rows:
            parser.error(
                "typeck サマリを生成できませんでした（typeck_metrics が存在しません）。"
            )
        table_md = format_typeck_table(rows)
        if args.typeck_table:
            pathlib.Path(args.typeck_table).write_text(table_md, encoding="utf-8")
        if args.update_typeck_readme:
            update_typeck_readme(
                pathlib.Path(args.update_typeck_readme).resolve(),
                table_md,
            )

    if not args.out_json and not args.out_md and not needs_typeck_table:
        print(json.dumps(report, ensure_ascii=False, indent=2))


if __name__ == "__main__":
    main()
