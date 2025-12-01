# Reml プロジェクト概要

[![Bootstrap Linux CI](https://github.com/dolphilia/kestrel/actions/workflows/bootstrap-linux.yml/badge.svg)](https://github.com/dolphilia/kestrel/actions/workflows/bootstrap-linux.yml)
[![Bootstrap macOS CI](https://github.com/dolphilia/kestrel/actions/workflows/bootstrap-macos.yml/badge.svg)](https://github.com/dolphilia/kestrel/actions/workflows/bootstrap-macos.yml)

Reml (Readable & Expressive Meta Language) はパーサーコンビネーターと静的保証に重点を置いた言語設計プロジェクトです。本リポジトリは仕様、設計ガイド、ブートストラップ実装計画、サンプル実装を集約し、言語実装とエコシステム整備を進めるための中枢ドキュメントとして機能します。

## ディレクトリ構成（再編後）

- `docs/`: 仕様書・ガイド・調査ノート・計画書を集約したアーカイブ
  - `docs/spec/`: 章番号付き Reml 公式仕様
  - `docs/guides/`: ツールチェーンや DSL 運用ガイド
  - `docs/notes/`: 調査メモと将来計画
  - `docs/plans/`: ブートストラップ実装計画・ロードマップ
- `compiler/`: Phase 1 (OCaml ブートストラップ) 〜 Phase 3 (セルフホスト) を受け止める実装領域
- `runtime/`: 最小ランタイムと Capability 拡張の実装領域
- `tooling/`: CLI・CI・リリース・LSP など開発ツール資産
- `examples/`: 仕様や計画書と連動したサンプル実装・比較資料
- `reports/`: CI/ローカルの監査ログと計測レポート。`reports/audit/index.json`・`summary.md`・`history/*.jsonl.gz`・`failed/<build-id>/` などの永続成果物を格納する。
- `docs-migrations.log`: 大規模ドキュメント移行の履歴
- `AGENTS.md` / `CLAUDE.md`: AI エージェント向け作業ガイド

## ドキュメントへの導線

- 仕様書・ガイド・調査ノートの全体索引: [`docs/README.md`](docs/README.md)
- ブートストラップ計画の統合マップ: [`docs/plans/bootstrap-roadmap/README.md`](docs/plans/bootstrap-roadmap/README.md)
- リポジトリ再編計画書: [`docs/plans/repository-restructure-plan.md`](docs/plans/repository-restructure-plan.md)
- 仕様書の差分履歴や横断的メモ: `docs/notes/` 配下の各ノートを参照
- Core Parse コンビネーター抽出の進捗: [`docs/spec/2-2-core-combinator.md`](docs/spec/2-2-core-combinator.md) 脚注および [`docs/notes/core-parse-api-evolution.md`](docs/notes/core-parse-api-evolution.md) Phase 2-5 Step6 を参照
- Unicode 識別子の暫定対応状況: [`docs/spec/1-1-syntax.md`](docs/spec/1-1-syntax.md)・[`docs/spec/1-5-formal-grammar-bnf.md`](docs/spec/1-5-formal-grammar-bnf.md) の脚注と [`docs/spec/0-2-glossary.md`](docs/spec/0-2-glossary.md) の「Unicode 識別子プロファイル（暫定）」を参照（Phase 2-7 `lexer-unicode` タスクで本実装予定）
- W3 型推論 dual-write の成果物と CLI オプション: [`reports/dual-write/front-end/w3-type-inference/README.md`](reports/dual-write/front-end/w3-type-inference/README.md) に `--dualwrite-root` の運用ルールと `remlc --frontend {ocaml,rust} --emit typeck-debug <dir>` を含む実行手順をまとめています。`scripts/poc_dualwrite_compare.sh --mode typeck --dualwrite-root reports/dual-write/front-end/w3-type-inference --run-id <label>` を利用し、Typed AST/Constraint/Impl Registry/Effects メトリクスの差分ログを取得してください。

## 実装ロードマップの要点

- **Phase 1 (OCaml ブートストラップ)**: パーサー/型推論/IR/LLVM/最小ランタイム/CLI/CI を揃える
- **Phase 2 (仕様安定化)**: 型クラス・効果タグ・診断メタデータ・Windows 対応を正式化
- **Phase 3 (Self-Host 移行)**: Reml 自身でコンパイラを構築し、標準ライブラリ API を完成
- **Phase 4 (リリース体制)**: マルチターゲット CI・署名・配布パイプライン・サポートポリシーを整備

詳細タスクや依存関係は [`docs/plans/bootstrap-roadmap/`](docs/plans/bootstrap-roadmap/) 以下を参照してください。

## サンプル実装

- [代数的効果サンプルセット](examples/algebraic-effects/README.md)
- [言語実装比較ミニ言語集](examples/language-impl-comparison/README.md)
- [Core.Collections 統合サンプル](examples/core-collections/README.md)
- [Core.Text & Unicode サンプル](examples/core-text/README.md)

## Core.Collections 進捗

- `docs/plans/bootstrap-roadmap/3-2-core-collections-plan.md` §6 で定義されたドキュメント・サンプル検証を完了し、同セクションの作業ログにも新規サンプルと API ハイライト（`List.push_front`/`Map.from_pairs`/`Vec.collect_from`/`Cell`/`Ref`/`Table.insert`）を追記しました。
- `docs/spec/3-2-core-collections.md` では `Map.from_pairs` の説明ブロックに `examples/core-collections/usage.reml` への `NOTE` を追加し、実装サンプルが `CollectError::DuplicateKey` を含む `List` → `Map` のパスと `Vec`・`Table`・`Cell`/`Ref` の効果タグを明示することを示しています。

## Core.Text 進捗

- `examples/core-text/text_unicode.reml` で Bytes/Str/String の三層モデル、`GraphemeSeq`、`TextBuilder`、`log_grapheme_stats`、`TextDecodeOptions` を横断するサンプルを追加し、`expected/text_unicode.*.golden` にトークン列・監査メタデータ・ストリーミング decode レポートを保存しました。
- `docs/spec/3-3-core-text-unicode.md` §9 に `examples/core-text` への注記を追記し、`reports/spec-audit/ch1/core_text_examples-YYYYMMDD.md` から実行ログを参照できるようにしています。サンプルの進捗記録は `docs/plans/bootstrap-roadmap/3-3-core-text-unicode-plan.md` §5 と `docs/notes/docs-update-log.md` で管理します。

## Core.Numeric & Time 進捗

- `docs/plans/bootstrap-roadmap/3-4-core-numeric-time-plan.md` §1.3 で `Core.Collections/Core.Iter/Core.Diagnostics/Core.Runtime` との依存関係を洗い出し、`docs/plans/bootstrap-roadmap/assets/core-numeric-time-dependency-map.drawio` に Numeric/Time API の連携図を追加しました。M4 (`Numeric / IO & Path`) の検証指標は同図と仕様 (`docs/spec/3-4-core-numeric-time.md`, `docs/spec/3-6-core-diagnostics-audit.md`, `docs/spec/3-5-core-io-path.md`) を基準に更新しています。
- `MetricPoint` → `AuditSink`、`StatisticsError` → `Diagnostic`、`Timestamp` → `IO` の 3 経路について、`docs/notes/core-numeric-time-gap-log.md` にバックログ（2025-12-01 付）を登録し、README・Phase3 Self-Host・監査計画 (`docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md`) から追跡できるようにしました。
- Time フォーマットのロケール表 (`docs/plans/bootstrap-roadmap/assets/time-format-locale-map.csv`) を `tooling/scripts/update_time_locale_table.py` で自動生成し、`compiler/rust/runtime/src/time/locale_table_data.rs` と `time::tests::planned_locale_is_rejected` で `LocaleStatus`（Supported/Planned）を検証できるようにしました。更新結果は `reports/spec-audit/ch3/time_format-locales.md` へ記録し、Gap Plan T-1 を消化しています。
- ICU パターン（`yyyy-MM-dd'T'HH:mm:ss` 等）を `time` 記法へ変換するトランスレータを `compiler/rust/runtime/src/time/format/icu.rs` に追加し、`tests/data/time/format/icu_cases.json` と `time::tests::{time_format_icu_cases_from_dataset,time_parse_icu_cases_from_dataset}` でフォーマット/パースの両方を検証しました。
- 代表的な IANA 名（`Asia/Tokyo`/`Europe/London`/`America/New_York`）を `timezone()` で受理できるよう静的オフセットテーブルを導入し、`tests/data/time/timezone_iana.json` / `time::tests::timezone_cases_from_dataset` にログを追加。`reports/spec-audit/ch3/time_timezone-iana.md` と `collect-iterator-audit-metrics.py --tz-source tests/data/time/timezone_iana.json` で進捗を共有しています。
- Core.IO から `TZ` / `LC_TIME` を収集する `time_env_snapshot()` を追加し、`TimeError` が `time.env.{timezone,locale}` を監査メタデータへ出力できるようにしました。`reports/spec-audit/ch3/time_env-bridge.md` に環境情報の取得フローをまとめ、Gap Plan T-4 の成果物として共有しています。
- `compiler/rust/runtime/src/diagnostics/metric_point.rs` に `MetricPoint`/`MetricValue`/`emit_metric` を実装し、`tests/data/metrics/metric_point_cases.json` と `reports/spec-audit/ch3/metric_point-emit_metric.json` を `collect-iterator-audit-metrics.py --section numeric_time --scenario emit_metric --metric-source ...` で検証できるようにしました。`effect.capability = "metrics.emit"` / `metric_point.tag.*` など監査メタデータの整合性を `docs/plans/bootstrap-roadmap/3-4-core-numeric-time-plan.md` §5.1 に記録しています。
- `StatisticsError` → `Diagnostic` のブリッジは `compiler/rust/runtime/src/numeric/error.rs` で `column`/`aggregation`/`audit_id` と `numeric.statistics.*`/`data.stats.*` の両メタデータへ書き込むように進捗し、`StatisticsTags`/`StatisticsError::with_tags` でタグセットを一括付与できるようにしました。`encode_sample_value` で `NaN`/`±Infinity` を JSON 文字列化し、`scripts/validate-diagnostic-json.sh --suite numeric` を追加して `tests/data/numeric/`・`tests/expected/numeric_*.json` を `diagnostic-v2` スキーマで検証できるようにしました（Plan §3.2）。
- `HistogramBucket`/`HistogramBucketState`/`StatisticsError` を `compiler/rust/runtime/src/numeric/{histogram,error}.rs` に PoC 実装し、`docs/plans/bootstrap-roadmap/assets/histogram-error-matrix.md` と `tests/data/numeric/histogram/*.json`、`scripts/validate-diagnostic-json.sh --pattern numeric.histogram` 連携で `3-4-core-numeric-time-plan.md` §2.2 の検証ルールを再現できるようにしました。
- `median`/`mode`/`range` を `compiler/rust/runtime/src/numeric/mod.rs` に追加し、`IterNumericExt` から利用できるようになりました。`reports/spec-audit/ch3/numeric_basic-extended.md` に `cargo test --manifest-path compiler/rust/runtime/Cargo.toml --features core-numeric numeric::tests::median_mode_and_range_cover_basic_cases` の結果を収録し、Gap Plan N-1 の完了条件を満たしています。併せて `Decimal`/`BigInt`/`BigRational` に対する `Numeric` 実装と `decimal`/`bigint`/`ratio` feature を導入し、`tests/data/numeric/decimal_cases.json` を `scripts/validate-diagnostic-json.sh --suite numeric` の対象へ追加しました。

## Core.IO & Path 進捗

- Plan 3-5 §6 のサンプル更新タスクを完了し、`examples/core_io/file_copy.reml` と `examples/core_path/security_check.reml` を追加しました。`tooling/examples/run_examples.sh --suite core_io|core_path` で再現でき、CI 指標 `core_io.example_suite_pass_rate` を `docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` に登録しています。
- `docs/spec/3-5-core-io-path.md` §4.2/§7 や `docs/spec/3-0-core-library-overview.md` にサンプル参照を追記し、Reader/Writer + `IoContext` と Path セキュリティヘルパの監査ポイントが一目で追えるようにしました。ガイド (`docs/guides/runtime-bridges.md`, `docs/guides/plugin-authoring.md`) では `IoContext.helper` と Capability チェックのベストプラクティスを追加しています。
- `docs/plans/bootstrap-roadmap/3-5-core-io-path-plan.md`、`docs/plans/rust-migration/overview.md`、`docs/plans/bootstrap-roadmap/3-0-phase3-self-host.md` にも本タスクの完了ログと依存関係を明記し、`docs/notes/runtime-bridges-roadmap.md` で Runtime Bridge とサンプル/CI Runbook の突合を管理します。

## コントリビューションのヒント

1. 仕様変更・ガイド更新時は `docs/spec/` および関連ノートの整合性を確認し、必要に応じて `docs-migrations.log` を更新
2. 実装タスクを着手する場合は `compiler/`, `runtime/`, `tooling/` の README を確認し、対応する計画書 (`docs/plans/...`) と同期
3. サンプルの追加・更新時は `examples/README.md` と関連仕様からのリンクを整備
4. 大規模なディレクトリ移動やリファクタリングを行う場合は [`docs/plans/repository-restructure-plan.md`](docs/plans/repository-restructure-plan.md) のフェーズ区分に従う
5. CLI の監査ログ圧縮 (`reports/audit/history/*.jsonl.gz`) は `camlzip` に依存するため、開発環境では `opam install . --deps-only --with-test` を実行して依存関係を揃える（`reml_ocaml.opam` に統合済み）。

## ライセンスとクレジット

Reml プロジェクトに関する利用条件やクレジット情報は今後 `docs/` 配下に集約予定です。暫定的な運用ポリシーは各仕様書・計画書内のライセンス欄を参照してください。
