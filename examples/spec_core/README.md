# spec_core スイート

Phase 4 で Chapter 1〜2 の構文・型・パーサ API サンプルを `.reml` 実行資産として再編成したディレクトリです。`docs/spec/1-5-formal-grammar-bnf.md` に記載された BNF 規則ごとにサブディレクトリを分割し、`examples/spec_core/chapter<chapter>/<rule_group>/bnf-<RuleName>-<variant>.reml` という命名規約で管理します。

- `chapter1/` 配下: `ValDecl`, `HandleExpr`, `ModuleUse`, `Attr`, `FnDecl`, `TypeDecl`, `TraitImpl`, `TypeInference`, `Conductor` など Chapter 1 BNF の正例/境界例/負例セット
- `chapter2/` 配下: `Core.Parse` と Streaming/Ops ビルダーの実行例 (`parser_core/`, `streaming/`, `op_builder/`) を章ごとに整理
- `expected/spec_core/`: それぞれの `.reml` に対応する `stdout` または `diagnostic.json` ゴールデン
- `phase4-scenario-matrix.csv`: `scenario_id`・`spec_anchor`・`variant` と本ディレクトリ構成を 1:1 で対応させています。

> 運用メモ: サンプル追加時は `docs/plans/bootstrap-roadmap/assets/phase4-scenario-matrix.csv` へ行を追加し、`variant` 列で「canonical/boundary/invalid」などの表記を合わせてください。

## Missing Examples ディレクトリ

`docs/plans/bootstrap-roadmap/4-1-missing-examples-plan.md` に従い、Chapter 1 で不足していた以下のディレクトリを追加しました。`tooling/examples/run_examples.sh --suite spec_core` はこれらの存在確認を行い、欠落しているとスイート全体を停止します。

- `chapter1/control_flow/`: `If` / `Loop` / `While` / `For` の境界例と構文エラーを格納
- `chapter1/literals/`: 整数・浮動小数・raw 文字列などリテラルのパターンを集約
- `chapter1/lambda/`: クロージャ捕捉やパターン引数などラムダ表現の正例/変種

## `expected/spec_core/**` のゴールデン生成手順

Missing Examples を含む各 `.reml` について CLI の出力を `expected/spec_core/**` に保存する際は、以下の手順を利用してください。`chapter1/control_flow` を例にしたコマンドを示します。

```bash
# 正常終了シナリオ（stdout ゴールデン）
(cd compiler/frontend && \
  cargo run --quiet --bin reml_frontend -- \
    ../../examples/spec_core/chapter1/control_flow/bnf-ifexpr-blocks-ok.reml \
    > ../../expected/spec_core/chapter1/control_flow/bnf-ifexpr-blocks-ok.stdout)

# 診断シナリオ（diagnostic JSON ゴールデン）
(cd compiler/frontend && \
  cargo run --quiet --bin reml_frontend -- \
    --output json \
    ../../examples/spec_core/chapter1/control_flow/bnf-loopexpr-unreachable-code.reml \
    | jq '.' \
    > ../../expected/spec_core/chapter1/control_flow/bnf-loopexpr-unreachable-code.diagnostic.json)
```

`literals` や `lambda` も同様に `.stdout` または `.diagnostic.json` を `expected/spec_core/chapter1/<directory>/` へ配置してください。ゴールデン更新後は `tooling/examples/run_examples.sh --suite spec_core` を再実行し、`reports/spec-audit/ch4/spec-core-dashboard.md` に結果を反映させます。
