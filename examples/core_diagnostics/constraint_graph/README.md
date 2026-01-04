# constraint_graph サンプル

- `simple_chain.reml` は `sum_pair`/`triple_product` の算術チェーンで構成され、TypeChecker の `ConstraintGraph` が最小例として観測できる。
- テレメトリ生成手順:
  ```bash
  cargo run --manifest-path compiler/frontend/Cargo.toml \
    -- --emit-telemetry constraint_graph=examples/core_diagnostics/output/simple_chain-constraint_graph.json \
    examples/core_diagnostics/constraint_graph/simple_chain.reml
  ```
- DOT/SVG 変換手順:
  ```bash
  cargo run --manifest-path tooling/telemetry/Cargo.toml -- \
    --dot-out examples/core_diagnostics/output/simple_chain.dot \
    --svg-out examples/core_diagnostics/output/simple_chain.svg \
    --graph-name SimpleChain \
    examples/core_diagnostics/output/simple_chain-constraint_graph.json
  ```
- `examples/core_diagnostics/output/` には上記手順で作成した `*.json`/`*.dot`/`*.svg` を保管し、`docs/spec/3-6-core-diagnostics-audit.md` や各ガイドの図版差し替え時に再利用する。
