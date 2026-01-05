# 第2部 第6章: 診断と出力 調査メモ

## 参照した資料
- `compiler/frontend/src/diagnostic/mod.rs:1-15`（診断モジュールの公開範囲と再エクスポート）
- `compiler/frontend/src/diagnostic/model.rs:1-742`（診断モデル、AuditEnvelope、Expected 系、DiagnosticBuilder）
- `compiler/frontend/src/diagnostic/recover.rs:1-220`（ExpectedToken/ExpectedTokensSummary の整列・要約）
- `compiler/frontend/src/diagnostic/json.rs:1-520`（診断 JSON の組み立て、Span→Location 変換）
- `compiler/frontend/src/diagnostic/formatter.rs:1-218`（監査メタデータ生成と Audit ID 付与）
- `compiler/frontend/src/diagnostic/unicode.rs:1-220`（Unicode 詳細と span の補正）
- `compiler/frontend/src/diagnostic/filter.rs:1-210`（Severity フィルタとステージ降格）
- `compiler/frontend/src/diagnostic/effects.rs:1-239`（StageAuditPayload と capability 監査）
- `compiler/frontend/src/diagnostic/messages/config.rs:1-204`（Config 診断テンプレート）
- `compiler/frontend/src/error.rs:1-58`（Recoverability/FrontendError の基礎）
- `compiler/frontend/src/output/cli.rs:1-258`（CLI 出力形式と LSP 変換）
- `compiler/frontend/src/output/localization.rs:1-136`（ローカライズキー抽出）
- `compiler/frontend/src/bin/reml_frontend.rs:3344-3700`（診断 JSON の集約と監査メタデータの適用）
- `docs/spec/2-5-error.md`（Core.Parse 観点の診断モデル）
- `docs/spec/3-6-core-diagnostics-audit.md`（診断/監査の統一モデル）

## 調査メモ

### 診断モデルの中心構造
- `FrontendDiagnostic` がフロントエンド診断の中心モデル。`severity`/`domain`/`code`/`span`/`notes`/`fixits` などのコア要素に加え、`expected_*` と `audit_metadata`/`audit` を持つ。(`compiler/frontend/src/diagnostic/model.rs:264-588`)
- `AuditEnvelope` は監査メタデータの入れ物で、診断本体にも同じ情報を複製する方針。(`compiler/frontend/src/diagnostic/model.rs:20-57`)
- `DiagnosticSeverity` と `DiagnosticDomain` は JSON ラベル化用の `as_str`/`label` を持ち、CLI/LSP 共通の文字列を生成する。(`compiler/frontend/src/diagnostic/model.rs:60-137`)
- `DiagnosticFixIt` は Insert/Replace/Delete の 3 種と span を提供し、JSON 化で `kind`/`text` が使われる。(`compiler/frontend/src/diagnostic/model.rs:212-261`, `compiler/frontend/src/diagnostic/json.rs:451-466`)

### Expected/Recover 系の要約
- `ExpectedToken` は Keyword/Token/Class/Rule/Custom などの分類を持ち、優先度とソート規則を内包する。(`compiler/frontend/src/diagnostic/recover.rs:10-121`)
- `ExpectedTokenCollector` は重複排除と優先度ソートを行い、`ExpectedTokensSummary` を生成する。(`compiler/frontend/src/diagnostic/recover.rs:123-220`)
- `FrontendDiagnostic::set_expected_tokens` と `overwrite_expected_summary` で `expected_*` を同期し、期待トークンが空のときに placeholder/humanized を設定する。(`compiler/frontend/src/diagnostic/model.rs:478-562`)
- ストリーミング解析では `ensure_streaming_expected` / `force_streaming_expected` が、期待トークンの空振りを補正する。(`compiler/frontend/src/diagnostic/model.rs:564-587`)

### JSON 変換と Span/Location
- `build_frontend_diagnostic` が診断 JSON の本体を構築し、`primary`/`location`/`expected`/`structured_hints` などをまとめる。(`compiler/frontend/src/diagnostic/json.rs:47-132`)
- `LineIndex` がバイト offset から行・列へ変換する簡易インデクス。`span_to_primary_value` がハイライト情報を付与する。(`compiler/frontend/src/diagnostic/json.rs:12-330`)
- `build_expected_field` と `build_recover_extension` が Expected/Recover の JSON フィールドを分離して構築する。(`compiler/frontend/src/diagnostic/json.rs:152-258`)

### 監査メタデータと Stage/Capability
- `FormatterContext` と `complete_audit_metadata` が CLI 由来の run_id/args を `audit_metadata` に注入し、`audit_id` を生成する。(`compiler/frontend/src/diagnostic/formatter.rs:11-150`)
- `StageAuditPayload` が stage/capability の追跡情報を JSON 拡張に展開し、実験段階の診断降格に利用される。(`compiler/frontend/src/diagnostic/effects.rs:170-239`, `compiler/frontend/src/diagnostic/filter.rs:5-72`)
- `apply_experimental_stage_policy` は `--ack-experimental-diagnostics` の指定がない場合に Error→Warning へ降格する。(`compiler/frontend/src/diagnostic/filter.rs:5-15`)

### Unicode 補正と診断強調
- `integrate_unicode_metadata` が Unicode 問題を含む診断で span を補正し、`extensions["unicode"]` と `audit_metadata` に詳細を埋め込む。(`compiler/frontend/src/diagnostic/unicode.rs:118-219`)

### CLI 出力と LSP 変換
- `OutputFormat` が human/json/lsp/lsp-derive を受け付け、`emit_cli_output` が分岐する。(`compiler/frontend/src/output/cli.rs:9-177`)
- Human 出力は `severity: message` と `--> file:line:col` を簡易表示し、LocalizationKey があれば追記する。(`compiler/frontend/src/output/cli.rs:180-215`)
- LSP 出力は `publishDiagnostics` と `window/logMessage` の JSON-RPC を出力し、診断 JSON を `data.diagnostic` に埋め込む。(`compiler/frontend/src/output/cli.rs:225-259`)
- `LocalizationKey` は `message_key`/`locale_args` を抽出し、LSP data に埋め込むための JSON を生成する。(`compiler/frontend/src/output/localization.rs:1-79`)

### 診断の集約と CLI 連携
- `build_parser_diagnostics` が Parser 由来の `FrontendDiagnostic` を JSON に変換する中核。期待トークン補正、extensions の追加、監査メタデータの最終化を行う。(`compiler/frontend/src/bin/reml_frontend.rs:3344-3505`)
- 型検査診断は `build_type_diagnostics` で JSON に組み立てられ、必要に応じて pattern/recover 拡張を注入する。(`compiler/frontend/src/bin/reml_frontend.rs:3533-3690`)

### 仕様との照合メモ
- `DiagnosticSeverity`/`SeverityHint`/`DiagnosticFixIt` が `docs/spec/2-5-error.md` および `docs/spec/3-6-core-diagnostics-audit.md` の列挙と一致している。
- `DiagnosticBuilder` が `docs/spec/3-6-core-diagnostics-audit.md` の必須フィールド要件（severity/domain/code）を満たすか検証する。(`compiler/frontend/src/diagnostic/model.rs:615-741`)
- `build_frontend_diagnostic` の JSON 形は `docs/spec/3-6-core-diagnostics-audit.md` の構造に近いが、`source_dsl` など未実装の項目が残る。

### 未確認事項 / TODO
- `DiagnosticFilter` の include/exclude パターンが CLI オプション (`--diagnostic-filter`) とどこで接続されているかを追跡する。
- `AuditEnvelope.metadata` に `schema_version` や `dsl` 関連のキーが挿入される箇所を追う（3.6 仕様の更新点）。
