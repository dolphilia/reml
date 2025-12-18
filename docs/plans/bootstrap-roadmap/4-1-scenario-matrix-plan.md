# 4.1 Phase 4 シナリオマトリクス整備計画

## 目的
- Phase 4 M1（シナリオマトリクス確定）の出口条件を満たすため、`.reml` シナリオの分類・仕様根拠・期待結果を一元管理する。
- `docs/spec/0-1-project-purpose.md` が定める性能と安全性の指標を、Chapter 1〜3 のコード例に沿って測定可能なテストケースへ落とし込む。
- `docs/spec/1-0-language-core-overview.md` から `docs/spec/3-10-core-env.md` までの既存サンプルを、`.reml` 実行資産として `examples/` および `reports/spec-audit/ch4/` に再配置する。
- Phase 3 で整備したリスト（`docs/plans/rust-migration/p1-test-migration-*.txt` 等）を再利用し、Phase 5 Self-host の前提となる「正例/境界例/負例」のトリオを Chapter ごとに揃える。
- `.reml` 実行を通じて、Chapter 1（構文・型・効果）〜Chapter 3（標準ライブラリ）の仕様どおりの許容範囲を明文化し、複数の表記揺れ・境界・意地悪ケースを網羅する。

## スコープ
- **含む**: `docs/spec/1-x`〜`3-x`・`docs/guides/core-parse-streaming.md` のサンプル抽出、`.reml` テストケース作成、`phase4-scenario-matrix.csv` の定義と更新フロー、`examples/spec_core`/`examples/practical` ディレクトリ構成案、`reports/spec-audit/ch4/` へのリンク整備。
- **含まない**: Rust 実装や CLI の挙動修正、セルフホスト工程そのもの、Phase 4 M2 以降で扱う CI ワークフロー設定（`4-2` 以降で管理）。
  - ただし、マトリクスの実行導線（`tooling/examples/run_phase4_suite.py` / `tooling/examples/run_examples.sh`）の最小更新は「マトリクス運用」の一部として扱う（スイート追加・レポート出力先追加など）。
- **前提条件**: Phase 3 の章別資産が `compiler/rust/`・`examples/` に揃っている、`docs/plans/bootstrap-roadmap/0-2-roadmap-structure.md` に沿って新規ファイルの命名・参照が決まっている。

## 成果物と出口条件
- `docs/plans/bootstrap-roadmap/assets/phase4-scenario-matrix.csv` を新設し、各行に `scenario_id`, `category`, `spec_anchor`, `input_path`, `expected`, `diagnostic_keys`, `resolution` を必須フィールドとして登録する。
- `examples/spec_core/`・`examples/practical/` にサブディレクトリ（`chapter1/boundary` 等）を定義し、マトリクスの `input_path` と 1:1 で対応させる命名規約を決める。
- `reports/spec-audit/ch4/spec-core-dashboard.md` と `reports/spec-audit/ch4/practical-suite-index.md` に、マトリクスと一致するハンドブックリンクを追加できる状態にする。
- `phase4-scenario-matrix.csv` に登録したカテゴリのうち 85% 以上が `.reml` 資産を伴い、`resolution` 列が `pending` 以外になっていることを確認する（M1 exit）。
- Chapter 1 のすべての構文規則について「正例/境界例/ギリギリエラー/明確なエラー」の 4 パターンを `.reml` で登録し、複数表記がある規則は各記法を個別の行として掲載する。

## 作業ブレークダウン

### 1. 資産棚卸しと分類軸の確定（69週目）
- `docs/spec/1-0-language-core-overview.md`, `docs/spec/1-1-syntax.md`, `docs/spec/1-2-types-Inference.md`, `docs/spec/1-3-effects-safety.md`, `docs/spec/3-0-core-library-overview.md` を横断し、サンプルコードを `Prelude/IO/Capability/Runtime/Plugin/CLI` のカテゴリへ分類。
- `docs/plans/rust-migration/p1-test-migration-*.txt` のケースを機械的に読み込み、既存 ID のまま `phase4-scenario-matrix.csv` にインポートするスクリプト（`scripts/migrate_phase4_matrix.py` 仮）を準備。
- `category` と `spec.chapter`（例: `chapter1.syntax`）の表を `docs/plans/bootstrap-roadmap/assets/README.md` に追記し、Phase 4 以降の参照に備える。

#### ✅ 69 週目実施ログ

- `docs/spec/1-0`〜`3-0` で引用されている `.reml` 資産を棚卸しし、`phase4-scenario-matrix.csv` を以下のカテゴリで更新した。Chapter 1（Prelude/Runtime/Capability/CLI）と Chapter 3（IO/Runtime/Plugin）を跨いで `expected` へのリンクを明示し、`resolution` と `stage_requirement` を Phase4 M1 exit 条件に沿って入力済み。

  | spec_anchor | サンプル資産 | category | expected・根拠 | 現状 |
  | --- | --- | --- | --- | --- |
  | `docs/spec/1-1-syntax.md§B.1` | `docs/spec/1-1-syntax/examples/use_nested(.reml)` | Prelude / Runtime | `reports/spec-audit/ch1/use_nested-20251119-diagnostics.json`, `use_nested_rustcap-20251117` | `ok`（Rust Frontend CLI/Streaming で診断 0） |
  | `docs/spec/1-1-syntax.md§B.5` | `docs/spec/1-1-syntax/examples/effect_handler.reml` | Capability | `reports/spec-audit/ch1/effect_handler-20251119-diagnostics.json` | `ok`（StageRequirement::AtLeast(Beta)） |
  | `docs/spec/1-0-language-core-overview.md§4.1` | `examples/cli/trace_sample.reml` | CLI | `reports/dual-write/front-end/poc/2025-11-07-w2-ast-inventory/trace_sample_cli.ocaml.diagnostics.json` | `pending`（Rust ゴールデン化待ち） |
| `docs/spec/3-5-core-io-path.md§7` | `examples/practical/core_io/file_copy/canonical.reml` / `examples/practical/core_path/security_check/relative_denied.reml` | IO | `reports/spec-audit/ch3/core_io_summary-20251201.md`, `tests/data/core_path/security/relative_denied.json` | `pending`（`expected/` へのゴールデン搬入待ち） |
  | `docs/spec/3-0-core-library-overview.md§3.6` | `examples/core_diagnostics/pipeline_branch.reml` | Runtime | `examples/core_diagnostics/pipeline_branch.expected.diagnostic.json` | `ok`（`effects.contract.stage_mismatch` を再現） |
| `docs/spec/3-0-core-library-overview.md§3.7` | `examples/practical/core_config/audit_bridge/audit_bridge.reml` | Plugin | `examples/core_config/reml.toml` | `pending`（`manifest dump` diff の expected 化を Phase4 内で行う） |

- `docs/plans/bootstrap-roadmap/assets/README.md` に `category × spec.chapter` の基準表を新設。Phase4 以降で追加カテゴリが必要になった場合はこの表を更新してから `phase4-scenario-matrix.csv` の `category` 列を編集する運用に切り替えた。
- `scripts/migrate_phase4_matrix.py` を作成。`python3 scripts/migrate_phase4_matrix.py --write` で `docs/plans/bootstrap-roadmap/p1-test-migration-*.txt` を解析し、未登録の ID を `phase4-scenario-matrix.csv` へ `variant=legacy` で一括追記できる。`--write` なしは dry-run として CSV を標準出力へ流す。
- `expected/cli/trace_sample/trace_sample_cli.diagnostic.json`, `expected/practical/core_io/file_copy/canonical.audit.jsonl`, `expected/practical/core_path/security_check/relative_denied.diagnostic.json`, `expected/practical/core_config/audit_bridge/manifest_snapshot.json` を追加し、該当シナリオ（CH1-CLI-101 / CH3-IO-201 / CH3-PATH-202 / CH3-PLG-310）の `resolution` を `ok` に更新。

### 2. `.reml` ケース作成とリンク付け（70〜71週目）
- `docs/spec/1-x` 各節に対して「正例/境界例/負例」の `.reml` を最低 1 セット作成し、`examples/spec_core/chapter1/` に配置。`docs/spec/1-5-formal-grammar-bnf.md` の各規則 ID をファイル名に含め、双方向参照を可能にする。
- `docs/spec/3-5-core-io-path.md`, `docs/spec/3-6-core-diagnostics-audit.md`, `docs/spec/3-8-core-runtime-capability.md`, `docs/spec/3-10-core-env.md` の実用例を `examples/practical/` に移植し、入出力および監査ログ例を `expected/` ディレクトリに保存。
- `docs/guides/runtime-bridges.md` / `docs/guides/plugin-authoring.md` と連携し、Capability を要求する `.reml` には `runtime_bridge`/`capability` の列を追加。Stage 要件を `phase4-scenario-matrix.csv` へ反映する。
- Chapter 1 の各構文に対し、`.reml` で表現可能な全バリエーションを列挙（例: `let` のパターン束縛書式、`match` の分岐、`effect handler` の `with`/`match` 等）。規則ごとに `variant` 列を設け、表記揺れの漏れが可視化されるようにする。

#### ✅ 70〜71 週実施ログ

- `examples/spec_core/README.md` を新設し、`chapter1/let_binding/`・`chapter1/effect_handlers/` に BNF 規則名を含むファイル（`bnf-valdecl-let-simple-ok.reml` など）と `expected/spec_core/...` のゴールデンを配置。`phase4-scenario-matrix.csv` の `CH1-LET-*` 行をこれらの ID へ差し替えて BNF ベースの命名規約を固定した。
- `examples/practical/core_io/file_copy/canonical.reml`, `examples/practical/core_path/security_check/relative_denied.reml`, `examples/practical/core_config/audit_bridge/audit_bridge.reml` を追加し、既存の `expected/practical/*` JSON / stdout / audit ログを新パスへ更新。関連仕様（`docs/spec/3-5-core-io-path.md`, `docs/spec/3-0-core-library-overview.md`）とガイド（`docs/guides/runtime-bridges.md`, `docs/guides/plugin-authoring.md`）の参照先も Practical 配下に揃えた。
- `examples/README.md`・`examples/practical/README.md` に Phase 4 の `spec_core`/`practical` 階層を追記し、`docs/notes/examples-regression-log.md` へ Practical 反映ログを残した。旧 `examples/core_io/*` などの参照は「実務ケースは `practical/` へ移行した」旨の注記を追加。
- `phase4-scenario-matrix.csv` へ `runtime_bridge` 列を追加し、`CH3-PLG-310` など Capability を要求する行に `audit_bridge` を登録。IO/Path/Plugin 系の `input_path` を Practical パスへ統一し、`expected/practical/core_io/file_copy/canonical.audit.jsonl` などのゴールデンと 1:1 対応させた。
- `examples/language-impl-comparison/`（比較サンプル）も Phase4 マトリクスで追跡できるよう、`tooling/examples/run_phase4_suite.py` に `--suite language_impl_comparison` を追加。`CH2-PARSE-501`（`basic_interpreter_combinator.reml`）を 1 件目として回帰実行できる状態にした（レポート出力: `reports/spec-audit/ch4/language-impl-comparison-dashboard.md`）。

#### 🔁 追加バックログ（Chapter 1〜3 `.reml` 拡充計画）

`examples/spec_core/chapter1` に偏っていたケースを Chapter 1 BNF／型推論／効果仕様全域へ広げ、さらに `chapter2`（Core.Parse API）と Chapter 3（Core.Text/Core.Diagnostics/Core.Env）へ `.reml` を追加する計画を以下のとおり整理した。全ケースは `phase4-scenario-matrix.csv` に `resolution=pending` で登録し、`expected/` ゴールデンおよび診断キーを定義したうえで `run_examples.sh --suite spec-core --suite practical` に組み込む。

##### Chapter 1（構文・型・効果）

- **CH1-MOD-003**（[1-1 §B.1](../spec/1-1-syntax.md#b1-モジュールとインポート)）: `examples/spec_core/chapter1/module_use/bnf-compilationunit-module-use-alias-ok.reml` と `expected/.../module_use/bnf-compilationunit-module-use-alias-ok.stdout` を追加し、`use Core.Parse.{Lex, Op.{Infix, Prefix}}` の再帰展開と `module spec_core.match_guard` のヘッダ処理を実行テスト化する。
- **CH1-MOD-004**（同 §B.1）: `examples/spec_core/chapter1/module_use/bnf-usedecl-super-root-invalid.reml` を用意し、`super` をルートモジュールで参照した際に `language.use.invalid_super` 診断を発行することを確認する。`expected/.../bnf-usedecl-super-root-invalid.diagnostic.json` でエラー位置・トークンを固定化する。
- **CH1-ATTR-101/102**（[1-1 §B.6](../spec/1-1-syntax.md#b6-属性attributes)）: `chapter1/attributes` ディレクトリを新設し、`bnf-attr-cfg-let-gate-ok.reml`（`@cfg(target = "cli")` で `let` ブロックを条件実行）と、未定義ターゲットを指定する `bnf-attr-cfg-missing-flag-error.reml`（`language.cfg.unsatisfied_branch` 診断）を追加する。`expected/...stdout` と `.diagnostic.json` を対にして `RunConfig` 分岐をテストする。
- **CH1-FN-101**（[1-1 §B.4 関数宣言](../spec/1-1-syntax.md#b4-宣言の種類)）: `chapter1/fn_decl/bnf-fndecl-generic-default-effect-ok.reml` でジェネリック引数・デフォルト引数・`!{io}` 効果注釈を組み合わせ、`expected/.../fn_decl/bnf-fndecl-generic-default-effect-ok.stdout` で推論と効果列の整合を確認する。
- **CH1-TYPE-201**（同 §B.4 型宣言）: `chapter1/type_decl/bnf-typedef-sum-recordpattern-ok.reml` を作成し、Sum/Record パターンと `..` 残余束縛を `expected/...stdout` でゴールデン化する。
- **CH1-TRAIT-301**（[1-2 §B.1](../spec/1-2-types-Inference.md#b-トレイト型クラス風と静的オーバーロード)）: `chapter1/trait_impl/bnf-traitdecl-default-where-ok.reml` で `trait Show<T> where T: Copy` + デフォルト実装を記述し、辞書生成のログを `expected/...stdout` に固定する。
- **CH1-IMPL-302**（[1-2 §B.2](../spec/1-2-types-Inference.md#b2-解決と整合性)）: `chapter1/trait_impl/bnf-impldecl-duplicate-error.reml` と `expected/...duplicate-error.diagnostic.json` を追加し、同一型への重複 `impl` に `typeclass.impl.duplicate` 診断が出るかを確認する。
- **CH1-INF-601**（[1-2 §H.1](../spec/1-2-types-Inference.md#h1-let-一般化)）: `chapter1/type_inference/bnf-inference-let-generalization-ok.reml` を追加し、`let id = fn x => x` が多相推論され `Vec<i64>`/`Vec<Text>` で再利用できることを `expected/...stdout` で実証する。
- **CH1-INF-602**（[1-2 §C.3 値制限](../spec/1-2-types-Inference.md#c3-値制限value-restriction)）: `chapter1/type_inference/bnf-inference-value-restriction-error.reml` を追加し、`var cell = []` を一般化しようとすると `language.inference.value_restriction` 診断が発生することを確認する。
- **CH1-EFF-701**（[1-3 §B](../spec/1-3-effects-safety.md#b-デフォルトの純粋性と値制限)）: `chapter1/effects/bnf-attr-pure-perform-error.reml` を増やし、`@pure fn` 内で `perform Console.log` を呼び出した際に `effects.purity.violated` 診断が発生することを `expected/...diagnostic.json` で固定する。
- **CH1-DSL-801**（[1-1 §B.8](../spec/1-1-syntax.md#b8-dsl制御ブロック-conductor)）: `chapter1/conductor/bnf-conductor-basic-pipeline-ok.reml` を実装し、`conductor telemetry { channels { ... } execution { ... } }` のブロック展開結果を `expected/...stdout` で確認する。`reports/spec-audit/ch4/` に `conductor` 監査タグを追加する計画に紐付ける。
- **CH1-MATCH-003**（[1-1 §C.3](../spec/1-1-syntax.md#c3-パターン束縛match-で共通)）: `chapter1/match_expr/bnf-matchexpr-when-guard-ok.reml` を追加し、`Some(x) when x > 10 as large` のような `when` + `as` バインダを許容することを `expected/...stdout` で保証する。

##### Chapter 2（Parser API）

- **CH2-PARSE-101**（[2-2 §A-3](../spec/2-2-core-combinator.md#a-3-変換コミット回復)）: `examples/spec_core/chapter2/parser_core/core-parse-or-commit-ok.reml` を新設し、`Core.Parse.or` と `commit` を組み合わせた `.reml` を `expected/...stdout` でゴールデン化する。`phase4-scenario-matrix` では `Runtime × chapter2.parser` として扱う。
- **CH2-PARSE-201**（[2-5 §E](../spec/2-5-error.md#e-recoverの仕様)）: `chapter2/parser_core/core-parse-recover-diagnostic.reml` を追加し、`Parse.recover` で `core.parse.recover.branch` 診断 JSON を生成する経路をテストする。実行時は `RunConfig.extensions["recover"].mode="collect"` と `sync_tokens=[";","\n"]` を設定し、どの同期点で回復したかを `scenario_notes` に残す。
- **CH2-STREAM-301**（[2-7 §C-1](../spec/2-7-core-parse-streaming.md#c-1-continuation-型)）: `chapter2/streaming/core-parse-runstream-demandhint-ok.reml` を記述し、`run_stream` と `DemandHint::More` の協調を `expected/...stdout` で検証する。
- **CH2-OP-401**（[2-4 §A-2](../spec/2-4-op-builder.md#a-2-レベル宣言fixity)）: `chapter2/op_builder/core-opbuilder-level-conflict-error.reml` と診断 JSON を作り、同じ優先度レベルに別 fixity を混在させた際の `core.parse.opbuilder.level_conflict` エラーを確認する。

##### Chapter 3（Core.Text / Diagnostics / Env）

- **CH3-TEXT-401**（[3-3 §4.1](../spec/3-3-core-text-unicode.md#41-grapheme-word-sentence-境界)）: `examples/practical/core_text/unicode/grapheme_nfc_mix.reml` を追加し、`Core.Text.graphemes` + `normalize(:nfc)` の往復結果を `expected/practical/core_text/unicode/grapheme_nfc_mix.stdout` で記録する。
- **CH3-TEXT-402**（[3-3 §3.3](../spec/3-3-core-text-unicode.md#33-診断連携と-parseerror)）: `.../grapheme_boundary_edge.reml` と診断 JSON を作り、結合文字の途中で切断した場合に `core.text.unicode.segment_mismatch` 診断が上がることをテストする。
- **CH3-DIAG-501**（[3-6 §1.1](../spec/3-6-core-diagnostics-audit.md#11-auditenvelope)）: `examples/practical/core_diagnostics/audit_envelope/stage_tag_capture.reml` を追加し、`AuditEnvelope.metadata` に `scenario.id` と `effect.stage.required` が埋まる JSONL を `expected/...audit.jsonl` に保存する。
- **CH3-RUNTIME-601**（[3-8 §10.2](../spec/3-8-core-runtime-capability.md#102-stage-ポリシーと-capability-契約)）: `examples/practical/core_runtime/capability/stage_mismatch_runtime_bridge.reml` を作成し、`runtime_bridge.stage_mismatch` 診断を `expected/...diagnostic.json` で固定する。
- **CH3-ENV-701**（[3-10 §1](../spec/3-10-core-env.md#1-環境変数アクセス)）: `examples/practical/core_env/envcfg/env_merge_by_profile.reml` と対応する stdout を整備し、`core.env.merge_profiles` が `@cfg` と同期することを確認する。`IO × chapter3.env` の代表ケースとして扱う。

##### FFI/Core Prelude 依存（spec_core ハーネス）

- **FFI-CORE-PRELUDE-001**（[3-1 Core Prelude](../spec/3-1-core-prelude-iteration.md) / [3-6 Diagnostics](../spec/3-6-core-diagnostics-audit.md)）：`compiler/rust/frontend/tests/core_iter_{effects,adapters,collectors,pipeline}.rs` を `.reml` シナリオと同様に追跡し、`reml_runtime_ffi` を `core_prelude` 付きで再利用できるかをマトリクスに記載する。`input_path` はテストファイル/スナップショット（`tests/snapshots/core_iter_*`）を指し、`expected` には `cargo test --package reml_frontend spec_core` の YAML ゴールデンを登録する。
- `resolution` が `pending` のあいだは capability shim の導入（`reml_runtime_ffi` に `capability::registry` を再輸出する補助モジュールを追加）を Phase 4.1 のフォローアップとして扱い、完了時に `ok` として `phase4-scenario-matrix.csv` と `reports/spec-audit/ch4/spec-core-dashboard.md` を同時更新する。

実装手順（共通）:

1. `examples/spec_core/chapter1/{module_use,attributes,fn_decl,...}` および `chapter2/{parser_core,streaming,op_builder}`、`examples/practical/core_{text,diagnostics,runtime,env}` を作成し、`docs/spec/0-3-code-style-guide.md` に沿って `.reml` を配置する。
2. 各 `.reml` に対応する `expected/` ゴールデン（`stdout`/`diagnostic.json`/`audit.jsonl`）を生成し、`phase4-scenario-matrix.csv` の `scenario_id` と 1:1 に対応させる。
3. `run_examples.sh` へ `spec_core chapter1/chapter2` サブセットを追加し、`cargo test -p reml_e2e -- --scenario spec-core` で自動実行できるよう `reports/spec-audit/ch4/spec-core-dashboard.md` にタグを追加する。
4. 診断ケース（`language.use.invalid_super` など）は `docs/spec/3-6-core-diagnostics-audit.md` のキー定義を引用し、必要に応じて `docs/spec/0-2-glossary.md` に用語を追記する。

### 3. マトリクス検証とレビューサインオフ（72週目）
- `phase4-scenario-matrix.csv` に `resolution` 列を設け、`ok` / `impl_fix` / `spec_fix` を入力。`docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md` とリンクするケースは `impl_fix` として登録。
- `reports/spec-audit/ch4/spec-core-dashboard.md` にシナリオ一覧と Pass/Fail 状態を出力する `scripts/gen_phase4_dashboard.py` を用意し、レビューで差分を確認できるようにする。
- Phase 4 レビュー会（週次）でマトリクスを共有し、未定義ケースを `docs/notes/phase4-practical-test-backlog.md` に追記。承認後に `phase4-scenario-matrix.csv` を `main` ブランチへ反映し、M1 完了を `docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` に記録。
- `.reml` 実行結果から「コンパイラ修正」「仕様追記」「許容」の別を判定し、`resolution` + `notes` に根拠を記載。判断に迷うケースは `docs/spec/1-x` の該当節を引用し、レビュー時に仕様の解釈を再確認する。

### 4. 更新運用とハンドオーバー（73週目）
- `phase4-scenario-matrix.csv` 更新ガイド（列定義、PR テンプレート、レビュー観点）を `docs/plans/bootstrap-roadmap/assets/phase4-scenario-matrix-guideline.md` として作成。
- `docs/plans/bootstrap-roadmap/4-4-field-regression-and-readiness-plan.md` と連携し、`resolution` が `impl_fix` / `spec_fix` のケースを自動で Issue/タスクに連携するワークフロー案を記述。
- `docs-migrations.log` に Phase 4 資産追加の履歴を残し、Phase 5 `phase5-readiness.md` で参照できるようにする。

## リスクとフォローアップ
- **シナリオ不足**: Chapter 1 の境界例が不足する場合は `docs/notes/core-library-outline.md` を参照し、追加ケースを `phase4-scenario-matrix.csv` に `priority=high` として登録。リードタイムが足りない場合は `run_examples.sh --suite spec-core` をスキップできるガードを `4-2` タスクと調整する。
- **分類不一致**: `category` や `spec.chapter` が統一されていない場合は `scripts/validate_phase4_matrix.py`（仮）で lint を走らせ、`docs/plans/bootstrap-roadmap/0-4-risk-handling.md` の「表記崩れ」リスクとして報告。
- **リンク切れ**: `examples/` リネーム時には `README.md` / `SUMMARY.md` / `phase4-scenario-matrix.csv` を同時更新し、`docs/plans/bootstrap-roadmap/0-2-roadmap-structure.md` の「相互参照維持」要件を満たす。
