# spec_core スイート

Phase 4 で Chapter 1〜2 の構文・型・パーサ API サンプルを `.reml` 実行資産として再編成したディレクトリです。`docs/spec/1-5-formal-grammar-bnf.md` に記載された BNF 規則ごとにサブディレクトリを分割し、`examples/spec_core/chapter<chapter>/<rule_group>/bnf-<RuleName>-<variant>.reml` という命名規約で管理します。

- `chapter1/` 配下: `ValDecl`, `HandleExpr`, `ModuleUse`, `Attr`, `FnDecl`, `TypeDecl`, `TraitImpl`, `TypeInference`, `Conductor` など Chapter 1 BNF の正例/境界例/負例セット
- `chapter2/` 配下: `Core.Parse` と Streaming/Ops ビルダーの実行例 (`parser_core/`, `streaming/`, `op_builder/`) を章ごとに整理
- `expected/spec_core/`: それぞれの `.reml` に対応する `stdout` または `diagnostic.json` ゴールデン
- `phase4-scenario-matrix.csv`: `scenario_id`・`spec_anchor`・`variant` と本ディレクトリ構成を 1:1 で対応させています。

> 運用メモ: サンプル追加時は `docs/plans/bootstrap-roadmap/assets/phase4-scenario-matrix.csv` へ行を追加し、`variant` 列で「canonical/boundary/invalid」などの表記を合わせてください。
