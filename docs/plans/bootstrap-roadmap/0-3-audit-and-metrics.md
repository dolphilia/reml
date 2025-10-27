# 0.3 測定・監査・レビュー記録

本章では Phase 1〜4 に共通する測定指標、診断と監査ログの収集方法、レビュー記録フォーマットを定義する。[3-6-core-diagnostics-audit.md](../../spec/3-6-core-diagnostics-audit.md) と `docs/notes/llvm-spec-status-survey.md` のフォーマットを継承し、各フェーズの完了条件を定量的に確認できるようにする。

## 0.3.1 指標セット
| カテゴリ | 指標 | 定義 | 収集タイミング | 仕様参照 |
|----------|------|------|----------------|----------|
| 性能 | `parse_throughput` | 10MB ソースの解析時間 (ms) | フェーズごとに最低 3 回計測 | [0-1-project-purpose.md](../../spec/0-1-project-purpose.md) §1.1 |
| 性能 | `memory_peak_ratio` | ピークメモリ / 入力サイズ | 各フェーズ主要マイルストーン後 | 同上 |
| 安全性 | `stage_mismatch_count` | Capability Stage ミスマッチ件数 | CI (PR ごと) | [3-8-core-runtime-capability.md](../../spec/3-8-core-runtime-capability.md) |
| 安全性 | `ffi_ownership_violation` | FFI 所有権警告件数 | CI + 週次レビュー | [3-9-core-async-ffi-unsafe.md](../../spec/3-9-core-async-ffi-unsafe.md) |
| 安全性 | `iterator.stage.audit_pass_rate` | `typeclass.iterator.stage_mismatch` 診断で必須監査キーが揃った割合 (0.0〜1.0) | CI（週次レビュー、PRごと） | [3-6-core-diagnostics-audit.md](../../spec/3-6-core-diagnostics-audit.md) §2.4 |
| 安全性 | `diagnostic.audit_presence_rate` | 診断 JSON で `audit` / `cli.audit_id` / `cli.change_set` / `schema.version` / `timestamp` が欠落なく出力された割合（0.0〜1.0、欠落時は 0.0） | CI（週次レビュー、PRごと） | [3-6-core-diagnostics-audit.md](../../spec/3-6-core-diagnostics-audit.md) §1 |
| 型クラス | `typeclass.metadata_pass_rate` | `extensions.typeclass` / `audit_metadata.typeclass.*` が完全に埋まっている割合 (0.0〜1.0) | CI（週次レビュー、PRごと） | 同 §1.4 |
| 型クラス | `typeclass.dictionary_pass_rate` | `extensions.typeclass.dictionary.*` と `AuditEnvelope.metadata["typeclass.dictionary.*"]` が `kind != "none"` で揃っている割合 (0.0〜1.0) | CI（週次レビュー、PRごと） | [1-2-types-Inference.md](../../spec/1-2-types-Inference.md) §B, [3-6-core-diagnostics-audit.md](../../spec/3-6-core-diagnostics-audit.md) §1 |
| 安全性 | `ffi_bridge.audit_pass_rate` | `ffi.contract.*` 診断で `AuditEnvelope.metadata.bridge.*` と拡張フィールドが揃った割合 (0.0〜1.0) | CI（週次レビュー、PRごと） | [3-9-core-async-ffi-unsafe.md](../../spec/3-9-core-async-ffi-unsafe.md), [3-6-core-diagnostics-audit.md](../../spec/3-6-core-diagnostics-audit.md) |
| Parser | `parser.parse_result_consistency` | `Parser_driver.run` と `run_partial` が生成する `ParseResult`（`value`/`span`/`diagnostics`/`consumed`/`committed`）の一致率。1.0 未満の場合は `parser_driver_tests.ml` の差分レポートを添付。 | CI（`dune runtest tests`、parser_driver シナリオ） | [2-1-parser-type.md](../../spec/2-1-parser-type.md) §A, [2-6-execution-strategy.md](../../spec/2-6-execution-strategy.md) §G |
| Parser | `parser.farthest_error_offset` | `DiagState.farthest_error_offset` が報告する最遠エラー位置（バイトオフセット）。`None` または 0 の場合は回復ロジックが無効化されているとみなしブロッカー登録。 | CI（`test_parse_result_state.ml` / CLI 失敗シナリオ） | 同上 |
| DX | `diagnostic_regressions` | 診断差分の件数 | PR ごと | [3-6-core-diagnostics-audit.md](../../spec/3-6-core-diagnostics-audit.md) |
| DX | `audit_diff.regressions` | `tooling/review/audit-diff.py` が算出した `diagnostic.regressions + metadata.changed` 件数 | PR ごと | 同上／`reports/diagnostic-format-regression.md` |
| DX | `audit_dashboard.generated` | 直近の `audit_dashboard` ジョブが成功し `reports/audit/dashboard/index.html` を出力した回数 | 週次レビュー | `docs/plans/bootstrap-roadmap/2-4-diagnostics-audit-pipeline.md` §5.2 |
| DX | `audit_query.coverage` | DSL プリセット（Stage/FFI/型クラス）でヒットした監査ログ数 / 全監査ログ数 | PR ごと | `docs/plans/bootstrap-roadmap/2-4-diagnostics-audit-pipeline.md` §5.3 |
| DX | `error_resolution_latency` | 重大バグの修正までの日数 | 月次 | [0-1-project-purpose.md](../../spec/0-1-project-purpose.md) §2.2 |

- CI 集計スクリプト: `tooling/ci/collect-iterator-audit-metrics.py` を用いて診断 JSON を検査し、結果を `tooling/ci/iterator-audit-metrics.json` に書き出す。`metrics[]` 配列には `diagnostic.audit_presence_rate`・iterator・FFI ブリッジ指標に加えて `review` セクション（`audit_diff.regressions`, `audit_query.coverage`）が含まれ、必須フィールド欠落時は `pass_rate = 0.0` に丸める。Linux / macOS / Windows いずれの CI ワークフローでも `--require-success` オプションを有効化し、`pass_rate` が 1.0 未満または DX 指標が閾値を超えた場合にジョブ全体を失敗させる。

### 0.3.1a レビュー支援ツール連携
- 差分検知: `tooling/review/audit-diff.py --base reports/audit/baseline.jsonl --target reports/audit/current.jsonl --output reports/audit/review/<commit>/diff` を CI/ローカル双方で実行し、`diff.json` に記録された `diagnostic.regressions`, `metadata.changed`, `pass_rate.delta` を `audit_diff.regressions` の計測値として採用する。PR テンプレートでは差分サマリを貼り付け、`reports/diagnostic-format-regression.md` のチェックリストに沿ってレビューする。CI 側では `collect-iterator-audit-metrics.py --section review --review-diff reports/audit/review/<commit>/diff.json` を呼び出し、差分指標を取得する。
- コメント投稿: `tooling/ci/publish-audit-diff.py --diff reports/audit/review/<commit>/diff.json` で Markdown コメントを生成し、`actions/github-script` などから PR へ投稿する。`--max-details` で差分テーブルの件数を調整可能。
- 可視化: `tooling/review/audit_dashboard.py --metrics <metrics.json> --render --output reports/audit/dashboard/` により `index.{html,md}` / `metrics.timeseries.csv` / `metrics.snapshot.json` を生成し、`audit_dashboard` CI ジョブでアーティファクト化する。マイルストーン固定値は `reports/audit/dashboard/releases/<milestone>/` に保存し、週次レビュー後に `audit_dashboard.generated` のカウントを更新する。CI 計測は `collect-iterator-audit-metrics.py --section review --review-dashboard reports/audit/dashboard/index.html` を介して行い、生成ファイルが存在しない場合は `failures[]` に記録される。
- クエリ: `tooling/review/audit-query --query-file tooling/review/presets/stage-regressions.dsl --from reports/audit/review/<commit>/target.jsonl --output reports/audit/review/<commit>/query/stage.json` のように DSL プリセットを用いて重点領域を抽出し、ヒット件数を `audit_query.coverage` の計算に利用する。プリセット一覧は `tooling/review/presets/README.md` で管理し、更新時は `docs/spec/3-6-core-diagnostics-audit.md` 付録へ同期する。CI では `collect-iterator-audit-metrics.py --section review --review-coverage reports/audit/review/<commit>/query/stage.json` を通じて集計する。
- いずれのツールも共通の正規化ライブラリ `tooling/review/audit_shared.py` を使用し、スキーマ更新時は当該モジュールと `tooling/json-schema/audit-diff.schema.json` を併せて更新する。更新差分は `reports/audit/review/<commit>/diff.md` に記録し、計測値を `0-3-audit-and-metrics.md` へ転記する。

### メタデータキー定義表（Diagnostic.extensions / AuditEnvelope.metadata）
| ドメイン | キー | `Diagnostic.extensions` | `AuditEnvelope.metadata` | 必須フェーズ | 備考 |
|----------|------|-------------------------|---------------------------|--------------|------|
| CLI 実行 | `cli.audit_id` | `diagnostic.extensions.cli.audit_id` | `metadata["cli.audit_id"]` | Phase 2-3 以降 | `audit_id` を CLI 実行単位で共有し、診断・監査の突合に利用する。 |
| CLI 実行 | `cli.change_set` | `diagnostic.extensions.cli.change_set` | `metadata["cli.change_set"]` | Phase 2-3 以降 | 差分適用対象の識別子。スキーマ v1.1 で追加。 |
| 型クラス | `typeclass.trait` | `extensions.typeclass.trait` | `metadata["typeclass.trait"]` | Phase 2-4 | 制約に対応するトレイト名。 |
| 型クラス | `typeclass.type_args[]` | `extensions.typeclass.type_args[]` | `metadata["typeclass.type_args"]` | Phase 2-4 | 文字列表現された型引数。 |
| 型クラス | `typeclass.constraint` | `extensions.typeclass.constraint` | `metadata["typeclass.constraint"]` | Phase 2-4 | `trait<args...>` 形式の制約表示。 |
| 型クラス | `typeclass.resolution_state` | `extensions.typeclass.resolution_state` | `metadata["typeclass.resolution_state"]` | Phase 2-4 | `resolved` / `stage_mismatch` / `unresolved` / `ambiguous` / `unresolved_typevar` / `cyclic` / `pending`。 |
| 型クラス | `typeclass.dictionary` | `extensions.typeclass.dictionary` | `metadata["typeclass.dictionary"]` | Phase 2-4 | 採用辞書の JSON 表現。`kind = "none"` で辞書欠落を明示。 |
| 型クラス | `typeclass.candidates[]` | `extensions.typeclass.candidates[]` | `metadata["typeclass.candidates"]` | Phase 2-4 | 候補辞書の配列。要素は `typeclass.dictionary` と同構造。 |
| 型クラス | `typeclass.pending[]` | `extensions.typeclass.pending[]` | `metadata["typeclass.pending"]` | Phase 2-4 | 後続処理へ委ねた制約の一覧。 |
| 型クラス | `typeclass.generalized_typevars[]` | `extensions.typeclass.generalized_typevars[]` | `metadata["typeclass.generalized_typevars"]` | Phase 2-4 | 一般化・未解決の型変数。 |
| 型クラス | `typeclass.graph.export_dot` | `extensions.typeclass.graph.export_dot` | `metadata["typeclass.graph.export_dot"]` | Phase 2-4 | 制約グラフ DOT ファイルのパスまたは `null`。 |
| 型クラス | `typeclass.span.start` / `.end` | `extensions.typeclass.span.start` 等 | `metadata["typeclass.span.start"]` 等 | Phase 2-4 | 制約導入位置のオフセット。 |
| 効果 | `effect.stage.required` | `extensions.effect.stage.required` | `metadata["effect.stage.required"]` | Phase 2-2 | Stage 宣言の期待値。 |
| 効果 | `effect.stage.actual` | `extensions.effect.stage.actual` | `metadata["effect.stage.actual"]` | Phase 2-2 | 実測 Stage。 |
| 効果 | `effect.stage.residual` | `extensions.effect.stage.residual` | `metadata["effect.stage.residual"]` | Phase 2-4 | 残余効果の JSON。 |
| 効果 | `effect.handler_stack` | `extensions.effect.handler_stack[]` | `metadata["effect.handler_stack"]` | Phase 2-4 | ハンドラ適用順。 |
| FFI | `bridge.platform` | `extensions.bridge.platform` | `metadata["bridge.platform"]` | Phase 2-3 | `linux-gnu` / `windows-msvc` / `macos-arm64` 等。 |
| FFI | `bridge.abi` | `extensions.bridge.abi` | `metadata["bridge.abi"]` | Phase 2-3 | 呼出規約の識別子。 |
| FFI | `bridge.ownership` | `extensions.bridge.ownership` | `metadata["bridge.ownership"]` | Phase 2-3 | 引数の所有権。 |
| FFI | `bridge.audit_pass_rate` | `extensions.bridge.audit_pass_rate` | `metadata["bridge.audit_pass_rate"]` | Phase 2-4 | CI での合格率。 |
| 解析 | `parser.input_name` | `extensions.parse.input_name` | `metadata["parse.input_name"]` | Phase 1-4 | ソース名。 |
| 解析 | `parser.stage_trace[]` | `extensions.parse.stage_trace[]` | `metadata["parse.stage_trace"]` | Phase 2-4 | レキサー→パーサ→補助解析の順序。 |

- キー追加時は本表を更新し、`docs/spec/3-6-core-diagnostics-audit.md` の付録と差異が無いか確認する。新規キーは `schema.version` をインクリメントした上で CI の `jsonschema` 検証対象に追加する。
- 監査永続化ストアの健全性チェックは `tooling/ci/verify-audit-metadata.py --index reports/audit/index.json --root . --history-dir reports/audit/history` を用いる。CI では `--strict` オプションの有効化を検討し、`retained_entries` の再計算結果と拡張キー欠落の双方を検証に含める。
- CI で監査ログを収集した後は `tooling/ci/create-audit-index.py --output reports/audit/index.json --audit <profile:target:path[:status[:level[:pass_rate]]]>` を実行して index を生成する。生成済み index を `verify-audit-metadata.py` に渡すことで一貫したゲートフローを構築する。

### `schema.version` 更新履歴
| バージョン | 反映日 | 主な変更点 | 参照ドキュメント | CI 導入 |
|-----------|--------|------------|-------------------|---------|
| v1.0 | 2025-09-30 | CLI `audit_id` / `change_set` の導入、`bridge.platform` キー確定 | `docs/plans/bootstrap-roadmap/2-3-ffi-contract-extension.md` §3 | Linux のみ (`bootstrap-linux.yml`) |
| v1.1 | 2025-10-24 | `extensions.bridge.*` 拡張、`schema_version` フィールドの必須化、macOS 監査サンプル追加 | `docs/plans/bootstrap-roadmap/2-3-completion-report.md` §5, `docs/plans/bootstrap-roadmap/2-3-to-2-4-handover.md` | Linux / macOS CI (`bootstrap-linux.yml`, `bootstrap-macos.yml`) |
| v1.1+phase2-4 | 2025-10-29（予定） | 型クラス・効果メタデータの必須化、Windows 監査ジョブ (ID 22) 対応、`bridge.audit_pass_rate` 追加 | `docs/plans/bootstrap-roadmap/2-4-diagnostics-audit-pipeline.md` §4 | Linux / Windows / macOS CI へ順次反映予定 |

- 履歴の更新と同時に `reports/audit/index.json` の `schema_version` フィールドを確認し、`tooling/ci/collect-iterator-audit-metrics.py` が最新値を指していることを確認する。
- メジャー更新（`v2.0` 等）時は `docs/migrations/audit-schema-history.md` に詳細な移行手順を残し、互換ウィンドウと移行スクリプトの有無を記載する。

### macOS 追加指標（Phase 1-8 以降）
| カテゴリ | 指標 | 定義 | 収集タイミング | 計画参照 |
|----------|------|------|----------------|----------|
| CI | `ci_build_time_macos` | `bootstrap-macos` ワークフローにおける `dune build` の実行時間（分） | push/pr ごと | [1-8-macos-prebuild-support.md](1-8-macos-prebuild-support.md) §5 |
| CI | `ci_test_time_macos` | `bootstrap-macos` ワークフローにおける `dune runtest` の実行時間（分） | push/pr ごと | 同上 |
| 品質 | `llvm_verify_macos` | `llvm-as` → `opt -verify` → `llc -mtriple=arm64-apple-darwin` の成否（0=成功,1=失敗） | CI 実行ごと | 同上 |
| 成果物 | `runtime_macho_size` | `libreml_runtime.a` (Mach-O) のファイルサイズ（KB） | 週次 | 同上 |
| 運用 | `macos_runner_queue_time` | GitHub Actions macOS ランナーの待機時間（分） | 週次レビュー | 同上 |

> **補足**: macOS 指標は Linux 指標との比較を想定し、`metrics.json` にターゲット別セクションを設けて記録する。乖離が 15% を超えた場合は `0-4-risk-handling.md` に登録して原因調査を開始する。

### Phase 1-8 実測値（macOS Apple Silicon ARM64）

**測定日**: 2025-10-11
**環境**: macOS 14.x / Apple Silicon (ARM64) / LLVM 18.1.8 / OCaml 5.2.1

| 指標 | 実測値 | 備考 |
|------|--------|------|
| `ci_build_time_macos` | 2.4秒 | `dune build` フルビルド（クリーンビルド後） |
| `ci_test_time_macos` | ~30秒 | `dune runtest` + ランタイムテスト + AddressSanitizer |
| `llvm_verify_macos` | 成功 (0) | ARM64 ターゲットで全サンプル検証成功 |
| `runtime_macho_size` | 56 KB | `libreml_runtime.a` (ARM64 Mach-O) |
| `macos_runner_queue_time` | 未計測 | GitHub Actions の実運用開始後に記録 |

**LLVM IR 検証結果**:
- ターゲット: `arm64-apple-darwin`
- 検証パイプライン: `llvm-as` → `opt -verify` → `llc -mtriple=arm64-apple-darwin`
- 全テストサンプル検証成功（examples/cli/*.reml）

## 0.3.2 レポートテンプレート
- **週次レポート**: `reports/week-YYYYMMDD.md`（将来追加予定）に以下の項目を記録する。
  - 主要マイルストーン進捗
  - 指標の最新値
  - リスク/ブロッカー（`0-4-risk-handling.md` へのリンク）
- **フェーズ終了レビュー**: 各 Phase 文書末尾のチェックリストと合わせて、以下を必須記録とする。
  - 指標表（最新値と目標）
  - レビュア署名（Parser/Type/Runtime/Toolchain）
  - 仕様変更一覧（ファイル/節/概要）

## 0.3.3 診断・監査ログ整合性
- `Diagnostic` オブジェクトの拡張フィールド (`extensions`) は [3-6-core-diagnostics-audit.md](../../spec/3-6-core-diagnostics-audit.md) に定義されたキー (`effect.stage.required`, `bridge.target` など) を使用する。
- `Diagnostic` と `AuditEnvelope` のフィールド差分と移行計画は [2-4-diagnostics-audit-pipeline.md](2-4-diagnostics-audit-pipeline.md#diagnostic-field-table-draft) の比較表ドラフトを参照する。
- `tooling/runtime/audit-schema.json` のバージョン管理は [2-4-diagnostics-audit-pipeline.md](2-4-diagnostics-audit-pipeline.md#audit-envelope-draft) の移行ステップ案に従い、`schema.version` フィールドを更新した際は本節と `docs/notes/audit-migrations.md`（新規予定）に履歴を残す。
- 監査ログ (`AuditEnvelope`) は JSON Lines 形式で保存し、以下を必須フィールドとする。
  - `metadata.effect.stage.required`
  - `metadata.bridge.target`
  - `metadata.bridge.abi`
  - `metadata.bridge.ownership`
- スキーマ検証: `tooling/runtime/audit-schema.json`（ドラフト）を基準に `bridge.*` フィールドを検証するツールを Phase 2-3 で整備する。仮段階では `tooling/ci/collect-iterator-audit-metrics.py` の `ffi_bridge.audit_pass_rate` を用いて欠落を検知する。
- ログ検証用に `tools/audit-verify`（将来実装予定）を準備し、CI で `--strict` フラグを用いて検証。

### 監査ログ収集・永続化フロー
1. **実行コマンド**  
   - ローカル検証: `remlc <target>.reml --emit-audit --audit-store=local --audit-level=full [追加オプション]`  
     実行後に `tooling/audit-store/local/<timestamp>/` 下へ `*.jsonl` と `index.json` が生成される。`index.json.latest` は最後のビルド ID を指すシンボリックリンクとして維持する。  
   - CI 実行: `remlc ... --emit-audit --audit-store=ci --audit-level=summary` を推奨し、`reports/audit/<target>/<YYYY>/<MM>/<DD>/<commit>_<target>_<build-id>.jsonl` を生成する。効果・FFI 検証ジョブでは `--audit-level=full` を併用する。
2. **インデックス更新**  
   - すべてのプロファイルで `AuditEnvelope.build_id` を `<utc timestamp>-<commit sha>` 形式で発行し、`reports/audit/index.json`（CI）または `tooling/audit-store/local/index.json`（ローカル）へ追記する。  
   - CI ではビルドごとのメタデータ（`target`, `pass_rate`, `audit_level`, `artifact_path`）を必須フィールドとし、`reports/audit/index.json` の `pruned` 配列で削除済みビルド ID を管理する。
3. **履歴・失敗ログ**  
   - `tooling/ci/collect-iterator-audit-metrics.py --prune` を週次で実行し、`tooling/ci/audit-retention.toml` に定義した `retain = {ci = 100, local = 30}` を超える履歴を削除する。削除対象は `reports/audit/history/<target>.jsonl.gz` へ圧縮退避し、失敗ビルドは `reports/audit/failed/<commit>/` に完全保存する。  
   - 圧縮履歴を更新した際は `reports/audit/usage.csv` に容量を追記し、500MB を超えた場合は `0-4-risk-handling.md` に対応策を登録する。
4. **メトリクス集計**  
   - `tooling/ci/collect-iterator-audit-metrics.py --summary reports/audit/index.json --output reports/audit/summary.md` を実行し、`ffi_bridge.audit_pass_rate` と `iterator.stage.audit_pass_rate` の推移を Markdown サマリとして生成する。  
   - サマリ生成後は CI アーティファクトとして保存し、レビュー時に `reports/ffi-bridge-summary.md` からリンクする。
5. **レビューチェックリスト**  
   - PR で `reports/audit/index.json` または `tooling/audit-store/local/index.json` が更新された場合は、レビュアが `audit-retention.toml` の閾値・`usage.csv` の容量推移・`summary.md` のメトリクス変化を確認する。  
   - 必須フィールド欠落や pass_rate < 1.0 を検出した場合はブロッカーとして `0-4-risk-handling.md` に登録し、修正完了後に削除する。

## 0.3.4 レビュア体制
| 領域 | 主担当 | 副担当 | レビュー頻度 |
|------|--------|--------|--------------|
| Parser/Core.Parse | TBD (Phase 1 決定) | TBD | 週次 |
| Type/Effects | TBD | TBD | 週次 |
| Runtime/Capability | TBD | TBD | 隔週 |
| Toolchain/CI | TBD | TBD | 隔週 |

レビュアの割当が変更された場合は、この表と各 Phase 文書のレビュア欄を更新する。担当者が空欄の場合は `0-4-risk-handling.md` にリスクとして記録し、埋めるまでフェーズ進行を停止する。

## 0.3.5 仕様差分追跡
- 仕様ファイルに変更が入った際は、以下の形式で記録する。
  - `YYYY-MM-DD / ファイル:節 / 変更概要 / 参照コミット`
- 記録は Phase ごとにセクションを分け、フェーズ終了時にレビューアが確認する。
- 差分が複数フェーズに跨る場合は、各フェーズで影響範囲を明記し、必要に応じて追加タスクを `0-4-risk-handling.md` に登録する。
- 2025-10-24 / docs/spec/3-6-core-diagnostics-audit.md:§2.4 / CLI・監査ゴールデン出力で `schema.version`, `audit.timestamp`, `AuditEnvelope.metadata.bridge.*` が欠落していることを確認。`scripts/validate-diagnostic-json.sh` ではスキーマ違反なしだが `ffi_bridge.audit_pass_rate = 0.0` のまま。 / pending (Phase 2-7)
- 2025-10-24 / docs/spec/3-8-core-runtime-capability.md:§10 / Stage 監査ログで `extensions.bridge.stage.*` と `effect.stage.*` が未出力のため `iterator.stage.audit_pass_rate = 0.0` を維持。Phase 2-7 技術的負債 ID22/23 と連携して解消予定。 / pending (Phase 2-7)

## 0.3.6 最適化パス統計（Phase 3 Week 10-11）

### 実装統計
| カテゴリ | 指標 | 値 | 備考 |
|----------|------|-----|------|
| コード規模 | Core IR 総行数 | 5,642行 | ir.ml, desugar.ml, cfg.ml, const_fold.ml, dce.ml, pipeline.ml, ir_printer.ml |
| テスト | テストケース総数 | 42件 | test_core_ir, test_desugar, test_cfg, test_const_fold (26件), test_dce (9件), test_pipeline (7件) |
| テスト | 成功率 | 100% (42/42) | 回帰なし |
| 最適化 | 定数畳み込み式数 | 変動 | パイプラインテストで計測 |
| 最適化 | 削除束縛数 | 変動 | DCEテストで計測 |
| 最適化 | 削除ブロック数 | 変動 | DCEテストで計測 |
| 性能 | ConstFold実行時間 | <0.001秒 | テストケース平均 |
| 性能 | DCE実行時間 | <0.001秒 | テストケース平均 |

### 最適化効果の例
- **定数畳み込み**: `10 + 20` → `30`（26件のテストで検証）
- **死コード削除**: `let x = 42 in 10` → `10`（9件のテストで検証）
- **パイプライン統合**: 不動点反復で複数パスを自動適用（7件のテストで検証）

### 品質指標
| 指標 | 値 | 目標 | 状態 |
|------|-----|------|------|
| `diagnostic_regressions` | 0件 | 0件 | ✅ 達成 |
| `stage_mismatch_count` | 0件 | 0件 | ✅ 達成 |
| テストカバレッジ | 100% | 95%以上 | ✅ 達成 |

## 0.3.7 RuntimeCapability 運用と効果診断ゴールデン

### JSON 管理手順
- Capability Registry は `tooling/runtime/capabilities/` に配置する。デフォルト設定は `default.json`、プラットフォーム差分は `{platform}.json`（例: `linux.json`, `windows.json`）で管理し、コミット時に必ず本節へ変更履歴を追記する。
- JSON フォーマット（暫定）は以下を必須キーとする。`stage` は `experimental` / `beta` / `stable` のいずれか、`capabilities` は `RuntimeCapability` 列挙子文字列、`overrides` はターゲットトリプル別の上書き設定。
  ```json
  {
    "stage": "stable",
    "capabilities": ["io", "panic", "runtime"],
    "overrides": {
      "x86_64-pc-windows-msvc": ["ffi", "process"]
    }
  }
  ```
- JSON の編集手順:
  1. 変更箇所を `tooling/runtime/README.md`（Phase 2-2 で追加予定）に記録し、出典となる仕様 (`docs/spec/3-8-core-runtime-capability.md`) を併記する。
  2. `scripts/validate-runtime-capabilities.sh`（Phase 2-2 で整備）を実行し、スキーマ検証と Stage 解釈トレースの再計算を行う。スクリプトは `reports/runtime-capabilities-validation.json` に `stage_summary`（CLI/JSON/環境変数/Runtime 判定の一覧）を出力し、CI で `jq` フォーマットチェックを通過することを確認する。
  3. 差分を `0-3.9 進捗ログ` に追記し、`stage_summary` から抜粋した Stage 変更点（例: `default.json: beta → stable`）を合わせて記録する。レビュアには JSON とサマリの両方を提示する。
- Stage が変更された場合は、必ず効果診断ゴールデンと `AuditEnvelope` ゴールデンを再生成し、`stage_trace` の差分が Typer/Runtime で一致していることを確認する。
- CLI/環境変数による Stage 指定を検証する場合は、`--cli-stage <stage>` / `--env-stage <stage>` を併用し、`stage_trace` の冒頭エントリ（`cli_option` / `env_var`）と整合を確認する。

#### Windows / 追加ターゲット差分検証
- `tooling/runtime/capabilities/default.json` では `overrides.x86_64-pc-windows-msvc` に Windows 専用の Stage と Capability を定義している。新しいターゲットを追加する場合も同じ `overrides` セクションか、個別の `{platform}.json` に追記し、本節へ差分を記録する。
- 検証手順:
  1. `scripts/validate-runtime-capabilities.sh tooling/runtime/capabilities/default.json --output reports/runtime-capabilities-validation.json` を実行し、`stage_summary.runtime_candidates` に `target: x86_64-pc-windows-msvc` が含まれることを確認する。
  2. 同ファイルの `overrides` に `target: arm64-apple-darwin` が追加された場合は同コマンドで再検証し、`runtime_candidates` に `arm64-apple-darwin` が `stage: beta` として出力されること、および `stage_trace` に同ターゲットのエントリが追加されていることを確認する。検証ログと併せて `reports/ffi-macos-summary.md` に記録し、レビューコメントで共有する。
  3. Stage や Capability を更新した場合は、`reports/runtime-capabilities-validation.json` の `stage_summary.json[].overrides` と `stage_trace` を 0.3.9 進捗ログへ抜粋し、レビューで参照できるようにする。
  4. 追加ターゲット（例: `aarch64-pc-windows-msvc` や `x86_64-unknown-linux-gnu` の派生）を導入した際は、同コマンドに `--cli-stage` / `--env-stage` を付与して優先度を再確認し、`tooling/ci/sync-iterator-audit.sh --metrics tooling/ci/iterator-audit-metrics.json --verify-log tooling/ci/llvm-verify.log --output reports/iterator-stage-summary.md` を再実行して `iterator.stage.audit_pass_rate = 1.0` ・`ffi_bridge.audit_pass_rate = 1.0` を維持しているかを確かめる。
- 検証の結果、`pass_rate < 1.0` となった場合や `stage_trace` に欠落が発生した場合は、影響段階が解消されるまで `0-4-risk-handling.md` に TODO を登録し、ロールバック方針と併せて共有する。

### CLI オプション優先度と検証
- Stage 解決は「CLI `--effect-stage` → JSON `--runtime-capabilities` → 環境変数 `REMLC_EFFECT_STAGE`」の優先順を採用し、`RuntimeCapabilityResolver`（Phase 2-2 で導入予定）で一元化する。
- 動作確認フロー:
  1. `remlc examples/effects/demo.reml --effect-stage beta --format=json` を実行し、`Diagnostic.extensions["effect.stage.required"]` が `beta` になることを確認。
  2. 同一コマンドに `--runtime-capabilities tooling/runtime/capabilities/linux.json` を追加し、JSON の `stage` が採用されることを `effect.stage.actual` で確認。
  3. どちらも指定せず `REMLC_EFFECT_STAGE=stable` を設定し、環境変数が採用されることを確認。
  4. 上記 3 ケースで `Diagnostic.extensions["effect.stage_trace"]` に出力される `source` / `stage` / `capability` の配列が CLI 指定 → JSON → 環境変数の順序で記録されていることを確認し、Runtime 側の `AuditEnvelope.metadata.stage_trace` も同一配列であることを `grep` などで突き合わせる。
- 上記 3 ケースの出力を `compiler/ocaml/tests/golden/diagnostics/effects/stage-resolution.json.golden`（新設）でスナップショット化し、`dune runtest compiler/ocaml/tests/test_diagnostics.ml` に統合する。

### 監査ログと CI 指標
- Stage 判定および FFI ブリッジ検証は `RuntimeCapabilityResolver` → `AuditEnvelope` → `tooling/ci/collect-iterator-audit-metrics.py` → `iterator.stage.audit_pass_rate` / `ffi_bridge.audit_pass_rate` の順で連携する。各段階で `stage_trace` または `bridge.*` が欠落した場合は CI を失敗させる。
- `remlc examples/effects/demo.reml --emit-audit --audit-store=local --audit-level=full --effect-stage beta` を実行し、`AuditEnvelope.metadata.stage_trace` に Typer 判定と Runtime 判定が連続して格納されていることを確認する。監査ゴールデンは `compiler/ocaml/tests/golden/audit/effects-stage.json.golden`（新設）に保存し、`tooling/audit-store/local/` に生成された `index.json` がビルド ID を記録しているか確認する。
- CI では `tooling/ci/sync-iterator-audit.sh --metrics /tmp/iterator-audit.json --audit compiler/ocaml/tests/golden/audit/effects-stage.json.golden` を実行し、`iterator.stage.audit_pass_rate` と `ffi_bridge.audit_pass_rate` がいずれも 1.0 であることをゲート条件とする。Stage 判定差分が発生した場合は `stage_trace` の乖離内容を Markdown サマリに追記し、FFI 契約差分が発生した場合は `bridge.*` 欠落項目をサマリへ明記してレビューへ共有する。
- 監査ログの更新後は `reports/runtime-capabilities-validation.json` の `stage_summary`・`iterator-stage-summary.md` および FFI ブリッジ用サマリ（導入後に `reports/ffi-bridge-summary.md` 予定）を本節へリンクする。

### 監査スキーマのバージョン管理ポリシー
- 管理対象: `tooling/runtime/audit-schema.json`（監査 JSON Lines スキーマ）を単一の真実源とし、更新時は `schema.version` フィールドを必ずインクリメントする。命名規約は `v<major>.<minor>`。  
- 変更手順:
  1. スキーマに差分が生じる場合は `docs/plans/bootstrap-roadmap/2-4-diagnostics-audit-pipeline.md` および関連仕様 (`docs/spec/3-6-core-diagnostics-audit.md`) に反映内容を追記し、レビュー依頼の際に `schema.version` の変更理由を記録する。
  2. スキーマ更新と同じブランチで `scripts/ci/verify-audit-schema.sh`（Phase 2-4 で導入予定）を実行し、`python3 -m jsonschema --instance <audit.jsonl> --schema tooling/runtime/audit-schema.json` で生成ログを検証する。CI へ導入後は Linux / Windows / macOS ジョブで同スクリプトを実行し、`schema-report.json` をアーティファクト化する。
  3. スキーマ変更が行われた場合は `docs/migrations/audit-schema-history.md`（新設予定）または既存レポートに差分サマリを追記し、`docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` の本節にリンクを追加する。
- リリース前チェック: `tooling/ci/collect-iterator-audit-metrics.py` は `schema_version` の不整合を検出した場合に失敗するよう設定する。CI での失敗は Phase 2-4 のゴール条件に含め、`compiler/ocaml/docs/technical-debt.md` で追跡する。
- 互換性ウィンドウ: `schema.version` がメジャー更新 (`v2.x` → `v3.0` 等) の場合は、旧バージョンログを 2 フェーズ分保持し、`scripts/audit/upgrade-schema.py`（導入予定）で自動移行できることを確認してから旧バージョンの受理を停止する。

### 効果診断ゴールデンの整備
- ゴールデン配置: `compiler/ocaml/tests/golden/diagnostics/effects/`（`*.golden`）に JSON スナップショットを保存し、必須キー `effect.stage.required` / `effect.stage.actual` / `effect.stage.residual` / `effect.stage.source` および `diagnostic.extensions.effect.stage_trace` / `diagnostic.extensions.effect.attribute` / `diagnostic.extensions.effect.residual` を全て検証する。
- 更新手順:
  1. `remlc` を `--format=json --emit-diagnostics` モードで実行し、一時ファイルを生成。
  2. `scripts/update-effects-golden.sh`（Phase 2-2 で追加予定）を用いて対象ゴールデンのみを上書きする。自動プロモートは使用しない。スクリプトでは `stage_trace` の差分を検知し、Typer / Runtime フェーズの順序が正しいかを静的チェックする。
  3. 更新後に `tooling/ci/collect-iterator-audit-metrics.py` を実行し、`iterator.stage.audit_pass_rate` / `ffi_bridge.audit_pass_rate` が 1.0 を維持していることを確認する。
  4. 差分と検証結果を本節に追記し、Phase 2-2 の週次レビュー議事録と同期する。
- ゴールデン差分がまだ確認されていない場合や Stage 検証が未完了の場合は、`0-4-risk-handling.md` に TODO を登録して Phase 2-2 の完了条件に含める。

## 0.3.8 LLVM ABI テスト統計（Phase 3 Week 15）

### 実装統計
| カテゴリ | 指標 | 値 | 備考 |
|----------|------|-----|------|
| コード規模 | ABI実装総行数 | 211行 | abi.ml/mli（System V ABI判定・LLVM属性設定） |
| コード規模 | テストコード総行数 | 518行 | test_abi.ml（拡張後） |
| テスト | テストケース総数 | 61件 | 既存45件 + 新規16件（境界値9件、エッジケース7件） |
| テスト | 成功率 | 100% (61/61) | 回帰なし |
| カバレッジ | 型サイズテスト | 20件 | プリミティブ9件、タプル8件、エッジケース3件 |
| カバレッジ | ABI判定テスト | 26件 | 戻り値13件、引数13件（境界値・エッジケース含む） |
| カバレッジ | LLVM属性テスト | 6件 | sret 3件、byval 3件 |
| カバレッジ | デバッグ関数テスト | 4件 | 文字列表現検証 |

### ABI判定精度
| 項目 | 詳細 | 検証結果 |
|------|------|----------|
| 16バイト境界 | (i64, i8) 15バイト以下 → DirectReturn/DirectArg | ✅ 正確 |
| 16バイト境界 | (i64, i64) 16バイト → DirectReturn/DirectArg | ✅ 正確 |
| 16バイト境界超過 | (i64, i64, i8) 17バイト超 → SretReturn/ByvalArg | ✅ 正確 |
| ネスト構造 | ((i64, i64), i64) 24バイト → SretReturn/ByvalArg | ✅ 正確 |
| エッジケース | () 空タプル 0バイト → DirectReturn/DirectArg | ✅ 正確 |
| FAT pointer | {data: String, count: i64} 24バイト → SretReturn/ByvalArg | ✅ 正確 |

### 品質指標
| 指標 | 値 | 目標 | 状態 |
|------|-----|------|------|
| `diagnostic_regressions` | 0件 | 0件 | ✅ 達成 |
| `stage_mismatch_count` | 0件 | 0件 | ✅ 達成 |
| テストカバレッジ | 100% | 95%以上 | ✅ 達成 |
| 境界値検証 | 3ケース | 2ケース以上 | ✅ 達成 |

### 技術的発見
- **パディング挙動**: (i64, i8)は実際には16バイトにパディングされ、境界値以下として正しく扱われる
- **ネストタプル**: ((i64, i64), i64)はフラット化され24バイトとして正しくABI判定される
- **関数型**: 現在の実装では関数ポインタ（8バイト）として扱われ、将来的にクロージャ（16バイト）への拡張が必要

## 0.3.9 進捗ログ

### phase2.week31
| マイルストーン | 期限 | 担当（ロール） | 成果物 | 状態 | 備考 |
|----------------|------|----------------|--------|------|------|
| Kick-off レビュー会議 | 31週目 Day1 午前 | 仕様差分補正チームリード、Phase 2-7 代表 | レビュースコープ承認メモ、連絡窓口一覧 | 予定 | `docs/plans/bootstrap-roadmap/2-5-spec-drift-remediation.md` §1.1、技術的負債 ID22/23 最新状況共有 |
| Chapter/領域別レビュー | 31週目 Day3 終了 | Chapter 1/2/3 担当、診断ログ担当 | 差分リスト初版（章別）、チェックリスト記入結果 | 予定 | `docs/plans/bootstrap-roadmap/checklists/spec-drift-review-checklist.md` を使用し、`scripts/validate-diagnostic-json.sh` 実行ログを添付 |
| スケジュール確定報告 | 31週目 Day5 終了 | 仕様差分補正チーム PM、Phase 2-7 調整役 | 週次レビュー計画（Week32-34）、`0-3-audit-and-metrics.md` 更新 | 予定 | Kick-off/領域レビュー結果を反映し、遅延時は `0-4-risk-handling.md` 登録 |

- 関連チェックリスト: `docs/plans/bootstrap-roadmap/checklists/spec-drift-review-checklist.md`（Phase 2-5 仕様差分レビュー）
- ログ記録手順: 各マイルストーン完了後に本表の状態欄を更新し、差分リスト ID とリンクを追記する。未達の場合は理由とリカバリ計画を備考欄へ記載し、必要に応じて `0-4-risk-handling.md` へエスカレーションする。

- 2025-10-06 / compiler/ocaml / パーサードライバーを `Result<Ast, Diagnostic>` へ移行し、`tests/test_parser.ml` に診断メタデータ検証を追加。`diagnostic_regressions` 指標は `dune test` による差分チェックで監視。
- 2025-10-07 / compiler/ocaml / Phase 3 Week 10-11 完了: Core IR 最適化パス（定数畳み込み、死コード削除、パイプライン統合）を実装。総コード行数: 約5,642行、テスト: 42件全て成功。
- 2025-10-09 / compiler/ocaml / Phase 3 Week 15 完了: ABI判定・属性設定のユニットテスト実装。総テストケース: 61件（既存45件 + 新規16件）、成功率: 100%。16バイト境界の正確な判定を検証済み。
- 2025-10-09 / tooling/ci/docker / `ghcr.io/reml/bootstrap-runtime:dev-local` を linux/amd64 でビルド（所要 ~530 秒、圧縮前 4.09GB）。`scripts/docker/run-runtime-tests.sh` と `scripts/docker/smoke-linux.sh` を実行し、既知の失敗（Let Polymorphism A2、LLVM ゴールデン差分、`basic_interpreter.reml` の構文エラー）を確認。計測値を `tooling/ci/docker/metrics.json` に記録。
- 2025-10-10 / .github/workflows / ランタイム CI 統合完了: `bootstrap-linux.yml` に Valgrind 検証・アーティファクト収集を追加し、Lint/Build/Test/Artifact の 4 ジョブ構成で Phase 1-5 §8 の CI 自動化を達成。
- 2025-10-16 / compiler/ocaml / `compiler/ocaml/scripts/benchmark_typeclass.sh --static-only` を実行し、辞書渡し／モノモルフィゼーションの静的比較レポート (`compiler/ocaml/benchmark_results/static_comparison.json`) を生成。現時点では while/for 未実装のため IR/ビットコード生成がスキップされメトリクスは 0 だが、Phase 3 でループ実装後に再計測予定。
- 2025-10-16 / tooling/ci / `collect-iterator-audit-metrics.py` → `sync-iterator-audit.sh` を手動実行し、`iterator.stage.audit_pass_rate = 1.0` を確認。`/tmp/iterator-summary.md` に生成した Markdown を次回 CI から `reports/` 階層へ保存し、週次で本ドキュメントへ転記する運用を開始。
- 2025-10-18 / tooling/runtime / `scripts/validate-runtime-capabilities.sh tooling/runtime/capabilities/default.json` を再実行し、`reports/runtime-capabilities-validation.json` の `runtime_candidates` に Windows (`x86_64-pc-windows-msvc`) の Stage `beta` が存在することを確認。運用手順を §0.3.7 に追記し、Phase 2-2 の Windows override 検証フローを確定。
- 2025-10-19 / tooling/runtime / `tooling/runtime/capabilities/default.json` に `arm64-apple-darwin` override（Stage `beta`, Capabilities: `ffi.bridge`, `process.spawn`）を追加。`reports/runtime-capabilities-validation.json`・`stage_trace` を手動更新し、`reports/ffi-macos-summary.md` を計測ログテンプレートとして新設。スクリプト再実行と CI ログ収集は Phase 2-3 macOS 計測タスクで実施予定。
- 2025-10-24 / docs/plans/bootstrap-roadmap/2-5-spec-drift-remediation.md / `reports/diagnostic-format-regression.md` の手順に従い `npm ci --prefix tooling/lsp/tests/client_compat` と `scripts/validate-diagnostic-json.sh` を実行。ゴールデンは schema v2 に適合したが `ffi_bridge.audit_pass_rate`・`iterator.stage.audit_pass_rate` は 0.0 のままで、差分リスト初期エントリとして Phase 2-7 タスク (ID22/23) を参照する運用を開始。

## 0.3.10 ランタイムテスト統計（Phase 1-5 Week 16）

### CI 統合実装
| カテゴリ | 指標 | 値 | 備考 |
|----------|------|-----|------|
| CI ワークフロー | ステップ追加数 | 5件 | Valgrind インストール、ビルド、テスト、Valgrind 検証、アーティファクト収集 |
| テスト | 実行テストケース | 14件 | メモリアロケータ（6件）、参照カウント（8件） |
| テスト | 成功率 | 100% | 全テスト成功（ローカル検証済み） |
| メモリ検証 | Valgrind 統合 | 有効 | `--leak-check=full --error-exitcode=1` で実行 |
| メモリ検証 | AddressSanitizer | 有効 | `DEBUG=1` ビルドで自動有効化 |
| アーティファクト | 保持期間（成功時） | 30日 | `libreml_runtime.a` および `.o` ファイル |
| アーティファクト | 保持期間（失敗時） | 7日 | テストバイナリおよびログファイル |

### メモリ安全性検証
| 項目 | 検証方法 | 結果 |
|------|----------|------|
| リーク検出 | Valgrind `--leak-check=full` | ✅ 0件（全テスト通過） |
| ダングリングポインタ | AddressSanitizer | ✅ 0件（全テスト通過） |
| 二重解放 | AddressSanitizer | ✅ 0件（全テスト通過） |
| 境界チェック | AddressSanitizer | ✅ 0件（全テスト通過） |

### 自動化範囲
- ✅ ランタイムビルド（`make runtime`）
- ✅ 基本テスト実行（`make test`）
- ✅ Valgrind メモリ検証（全テストバイナリ）
- ✅ アーティファクト自動収集（成功時・失敗時）
- ✅ ローカル再現手順のドキュメント化（`runtime/native/README.md`）

### 品質指標
| 指標 | 値 | 目標 | 状態 |
|------|-----|------|------|
| `diagnostic_regressions` | 0件 | 0件 | ✅ 達成 |
| `memory_leak_count` | 0件 | 0件 | ✅ 達成 |
| `test_coverage` | 100% | 95%以上 | ✅ 達成 |
| CI 実行時間（追加分） | 約3-5分 | 10分以内 | ✅ 達成 |

### 今後の課題（Phase 2 以降）
- [ ] Windows 環境での Valgrind 代替（Dr. Memory など）
- [ ] macOS 環境でのメモリリーク検証（leaks コマンド）
- [ ] CI 実行時間の最適化（キャッシュ戦略の改善）
- [ ] クロスプラットフォームでのアーティファクト統合

---

本章で定義した指標とログフォーマットは、計画書全体の共通基盤として扱う。各 Phase 文書はここで定義した指標を利用し、進行状況と品質を定量的に管理する。
