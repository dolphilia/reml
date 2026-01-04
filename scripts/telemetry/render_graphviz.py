#!/usr/bin/env python3
"""TraitResolutionTelemetry から Graphviz DOT/SVG を生成する補助スクリプト。"""

import argparse
import json
import subprocess
import sys
from pathlib import Path


def escape_label(label: str) -> str:
    return label.replace('"', '\\"')


def node_shape(kind: str) -> str:
    mapping = {
        "type": "ellipse",
        "capability": "box",
        "implementation": "diamond",
    }
    return mapping.get(kind, "ellipse")


def build_dot(graph: dict, graph_name: str) -> str:
    nodes = graph.get("nodes") or []
    edges = graph.get("edges") or []
    lines = [f"digraph {graph_name} {{", "  rankdir=LR;"]
    for node in nodes:
        node_id = f'n{node.get("id", 0)}'
        label = escape_label(node.get("label", node_id))
        shape = node_shape(str(node.get("kind", "type")))
        lines.append(f'  {node_id} [label="{label}", shape={shape}];')
    for edge in edges:
        from_id = f'n{edge.get("from", 0)}'
        to_id = f'n{edge.get("to", 0)}'
        kind = edge.get("kind")
        attr = ""
        if kind:
            attr = f' [label="{escape_label(str(kind))}"]'
        lines.append(f"  {from_id} -> {to_id}{attr};")
    lines.append("}")
    return "\n".join(lines)


def render_svg(dot_path: Path, svg_path: Path) -> None:
    cmd = ["dot", "-Tsvg", str(dot_path), "-o", str(svg_path)]
    try:
        subprocess.run(cmd, check=True)
    except FileNotFoundError as exc:
        raise RuntimeError("Graphviz (dot) が見つかりません") from exc
    except subprocess.CalledProcessError as exc:
        raise RuntimeError(f"dot コマンドが失敗しました: {exc}") from exc


def main() -> int:
    parser = argparse.ArgumentParser(
        description="TraitResolutionTelemetry JSON から Graphviz DOT/SVG を生成します。"
    )
    parser.add_argument("input", help="Telemetry JSON (constraint_graph) のパス")
    parser.add_argument(
        "--dot-out",
        help="出力する DOT ファイル。省略時は <input>.dot",
    )
    parser.add_argument(
        "--svg-out",
        help="SVG を同時生成する場合の出力先（Graphviz dot が必要）",
    )
    parser.add_argument(
        "--graph-name",
        default="ConstraintGraph",
        help="Graphviz 上のグラフ名（既定: ConstraintGraph）",
    )
    args = parser.parse_args()

    input_path = Path(args.input)
    if not input_path.exists():
        sys.stderr.write(f"[telemetry] 入力ファイルが存在しません: {input_path}\n")
        return 1

    graph_data = json.loads(input_path.read_text(encoding="utf-8"))
    dot_text = build_dot(graph_data.get("graph", {}), args.graph_name)

    dot_path = Path(args.dot_out) if args.dot_out else input_path.with_suffix(".dot")
    dot_path.parent.mkdir(parents=True, exist_ok=True)
    dot_path.write_text(dot_text, encoding="utf-8")
    print(f"[telemetry] DOT を {dot_path} へ書き出しました")

    if args.svg_out:
        svg_path = Path(args.svg_out)
        svg_path.parent.mkdir(parents=True, exist_ok=True)
        try:
            render_svg(dot_path, svg_path)
        except RuntimeError as exc:
            sys.stderr.write(f"[telemetry] {exc}\n")
            return 1
        print(f"[telemetry] SVG を {svg_path} へ書き出しました")

    return 0


if __name__ == "__main__":
    sys.exit(main())
