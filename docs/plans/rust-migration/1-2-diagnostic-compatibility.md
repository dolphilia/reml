# 1.2 診断互換性計画

Rust フロントエンド移植において、OCaml 実装と同一の診断 (`Diagnostic.t`) を生成するための基準と検証手順を定義する。構文・型推論・効果解析で発生する診断が JSON/LSP/監査メトリクスに反映される点までをカバーし、`reports/diagnostic-format-regression.md` のフローと整合させる。

## 1.2.1 目的
- OCaml 版で生成される診断 JSON / CLI 出力 / LSP データを Rust 版でも再現し、`effects.*`, `parser.stream.*`, `type_row.*` など拡張フィールドを含めて完全互換を確保する。
- Dual-write 実行時に発生する差分を特定・分類・記録する手順を定義し、仕様差分か実装差分かを判定できる状態を作る。
- CI（P3）で導入予定の自動比較ジョブを想定し、Rust 版診断生成の API とメトリクス収集を標準化する。

## 1.2.2 スコープ
- **対象**: `Diagnostic.Builder`, `parser_driver.ml` の recover 拡張、型推論エラー (`Type_error`), 効果監査 (`Type_inference_effect`, `collect-iterator-audit-metrics.py`)。
- **除外**: CLI レイヤーの最終的なテキスト整形（`diagnostic_formatter.ml`）の Rust 実装詳細。テキスト整形は Phase P2 で再検討し、P1 では JSON 互換性と LSP/XLang への出力のみ確認する。
- **前提**: P0 で確定したゴールデン (`compiler/ocaml/tests/golden/diagnostics/`) が最新であり、`scripts/validate-diagnostic-json.sh` が成功する状態。

## 1.2.3 ベースラインと比較対象

| 出力経路 | OCaml 版ベースライン | 検証用ツール | 備考 |
| --- | --- | --- | --- |
| CLI JSON | `compiler/ocaml/tests/golden/diagnostics/*.json.golden` | `scripts/validate-diagnostic-json.sh` | JSON Schema v2.0.0-draft に準拠 |
| CLI テキスト | `compiler/ocaml/tests/golden/diagnostics/*.txt.golden` | `diagnostic_formatter.mli` を参照（P1 では参考） | P2 で Rust CLI 実装と同期 |
| LSP JSON-RPC | `tooling/lsp/tests/client_compat/fixtures/*.json` | `npm run ci --prefix tooling/lsp/tests/client_compat` | Position 情報の差分は許容なし |
| 監査メトリクス | `reports/diagnostic-format-regression.md` 手順で生成 | `tooling/ci/collect-iterator-audit-metrics.py` | `--section parser`/`effects` 等 |

## 1.2.4 差分分類と対応

| 分類 | 例 | 対応 |
| --- | --- | --- |
| 仕様差分（許容外） | `severity` が `Warning` から `Error` へ | 即時修正。Rust 実装のバグとして扱い、差分ログに記録 |
| 実装差分（許容内） | フィールド順序、空配列省略 | `reports/diagnostic-format-regression.md` で規定された正規化を適用 |
| 新拡張フィールド追加 | `extensions.effect_syntax.*` の増加 | `docs/spec/3-6-core-diagnostics-audit.md` 等の仕様更新を伴う。P1 では原則追加しない |
| Precision 差分 | 数値のフォーマット違い | `serde_json::Number` の文字列表現を OCaml と揃える（`format!("{:.6}", ...)` 等） |

## 1.2.5 Dual-write 検証フロー
1. `remlc --frontend ocaml --format json --emit-ast path.reml > reports/dual-write/front-end/ocaml/<case>.json`
2. `remlc --frontend rust --format json --emit-ast path.reml > reports/dual-write/front-end/rust/<case>.json`
3. `scripts/validate-diagnostic-json.sh reports/dual-write/front-end/{ocaml,rust}/<case>.json` を実行して Schema 検証
4. `jq --sort-keys` で整形し `diff -u`。差分がある場合は `reports/dual-write/front-end/diff/diagnostic_<case>.diff` に保存
5. `collect-iterator-audit-metrics.py --section parser --baseline reports/dual-write/front-end/ocaml/<case>.json --candidate reports/dual-write/front-end/rust/<case>.json` を実行し、メトリクス差分を取得
6. 差分内容を `reports/diagnostic-format-regression.md` のフォーマットで記録し、`docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md` に TODO を追加（必要なら）

## 1.2.6 重点監視フィールド

| キー | 説明 | 参照仕様 | 検証観点 |
| --- | --- | --- | --- |
| `expected_tokens` (recover 拡張) | 期待トークン列 | `docs/spec/2-5-error.md` | OCaml と同順序・同件数、`message/context` の有無一致 |
| `effects.stage.*` | 効果段階監査 | `docs/spec/3-6-core-diagnostics-audit.md` | `type_row` 診断と連動、空配列は省略 |
| `parser.stream.*` | ストリーミング監査 | `docs/guides/core-parse-streaming.md` | Packrat 収束率、`backpressure_sync` |
| `type_row.*` / `typeclass.dictionary.*` | 型行 / 型クラス辞書監査 | `docs/spec/1-3-effects-safety.md`, `EFFECT-002-proposal.md` | dual-write で JSON 配列順序を固定 |
| `extensions["recover"]` | 再回復ヒント | `reports/diagnostic-format-regression.md` | CLI/LSP 両方で一致すること |

## 1.2.7 Rust 実装での設計指針
- `Diagnostic` モデルは `serde` で JSON 直列化可能な構造体として設計し、既存スキーマと同じフィールド名を採用。`Option`/空配列の扱いは OCaml 実装に合わせて「空配列 → 省略」「空文字列 → 省略」。
- `Diagnostic.Builder` の API を Rust でも提供し、`set_expected`, `set_extension` 等のメソッド名を踏襲。`recover` 拡張は専用ビルダ関数を定義する。
- `parse_error` / `type_error` などイベント単位でログ出力を行い、dual-write 比較時に原因追跡できるよう `trace_id` を付与する。
- 効果監査 (`Type_inference_effect`) のメタデータは `HashMap<String, Value>` で保持し、`collect-iterator-audit-metrics.py` が期待するキーセットを維持。Rust 実装では `serde_json::Value` で透過的に扱う。

## 1.2.8 テスト拡張計画
- **ゴールデン増補**: 効果構文 PoC (`effect_syntax.*`) や Streaming Runner (`parser/streaming-outcome.json.golden`) を Rust 版向けに再実行し、差分がなければ共通ゴールデンとして維持。
- **CLI/LSP 一貫性テスト**: `tooling/lsp/tests/client_compat` を Rust 実装で再利用できるよう、`remlc` CLI に Rust フロントエンド選択フラグを追加。LSP から得た診断 JSON を CLI 出力と diff。
- **手動検証ノート**: 仕様変更や例外的な差分は `reports/diagnostic-format-regression.md` の指示に従って調査ノートを残し、`docs/notes/` に TODO 付きで記録する。

## 1.2.9 既知リスクと対策
- **JSON 直列化の順序差**: Rust の `serde_json` はマップ順序を保証しないため、`IndexMap` を採用してフィールド順序を OCaml と揃える。`sort_keys` を行ってから比較することも必須。
- **数値フォーマットの差分**: `f64` 等をそのまま直列化すると指数表記が変化する可能性がある。OCaml 版が文字列を保持している箇所（リテラル等）は Rust でも文字列として保存。
- **Packrat 統計の収集差**: Rust 実装で `packrat_stats` を実装しないと `parser.stream.packrat_hit` 等が 0 になる。`Core_parse_streaming.packrat_cache` 同等のメトリクスを実装する。
- **外部依存ライブラリ**: JSON Schema 検証のために `jsonschema` crate を導入する場合、スキーマファイルのメンテナンスを `docs/spec/2-5-error.md` と同期させる。

## 1.2.10 ドキュメント連携
- 本計画で確定した比較ルールは `1-0-front-end-transition.md` に記載したマイルストーンと連動させ、レビュー時に参照する。
- 差分の緩和条件や例外は `appendix/glossary-alignment.md`・`docs/spec/3-6-core-diagnostics-audit.md` に反映し、用語・キー名称の整合を保つ。
- CI への組み込み手順は P3 ドキュメント (`3-0-ci-and-dual-write-strategy.md`) に移植する。P1 ではローカルおよび臨時 CI ジョブで実施。

## 1.2.11 型推論起因診断の比較手順（W3 拡張）
- `docs/plans/rust-migration/appendix/w3-typeck-dualwrite-plan.md` で定義した `effects-metrics.{ocaml,rust}.json` と `typeck-debug.{ocaml,rust}.json` を診断互換性の必須成果物に追加する。`collect-iterator-audit-metrics.py --section effects --require-success` を実行し、`effects.impl_resolve.delta` `effects.stage_mismatch.delta` が ±0.5pt 以内であることを確認する。
- `scripts/poc_dualwrite_compare.sh --mode typeck --run-id <label> --cases <file>` を実行すると、`reports/dual-write/front-end/w3-type-inference/<case>/` に `diagnostics.{ocaml,rust}.json` / `effects-metrics.*` / `typeck-debug.*` が保存される。`typeck-debug` には `effect_scope`, `residual_effects`, `recoverable`, `ocaml_exception` など型推論固有のフィールドが含まれるため、`jq --sort-keys` で整形した後 `diff -u` を取得する。
- Rust 側で `Result<T, TypeError>` を導入した箇所は、OCaml 側の例外名・診断コード・Recover ヒントを `diagnostic::codes::TYPE_*` に写像し、`typeck-debug` に `{"ocaml_exception": "...", "rust_error": "...", "diagnostic_code": "TYPE_xxx"}` の形で両実装のメタデータを併記する。これにより、`scripts/validate-diagnostic-json.sh` が指摘した差分を `typeck-debug` から逆引きできる。
- CLI 追加フラグ: `remlc --frontend rust --emit typed-ast --emit constraints --emit typeck-debug <dir>` / `remlc --frontend ocaml --emit-constraints-json <path> --emit-typeck-debug <path>`。両方の出力を `p1-front-end-checklists.csv` の新規行（型推論診断）の受入基準として記録し、`docs/spec/3-6-core-diagnostics-audit.md` へのフィードバック対象にする。
- *2027-01-17 進捗*: `reports/dual-write/front-end/w3-type-inference/2027-01-15-w3-typeck/diagnostic-validate.log` で `scripts/validate-diagnostic-json.sh` 通過、`effects-metrics.{ocaml,rust}.json` の `effects.unify.*` / `effects.impl_resolve.*` 誤差 0 を確認した。`ffi_dispatch_async` のみ OCaml 側診断が `Type_error` で終了するため `typeck_match=false` だが、診断 JSON の差分は `docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md#W3-TYPECK-ffi-dispatch-async` で追跡し、その他 4 ケースは `typeck-debug` を含め完全一致した。

## 1.2.12 W4 診断互換試験向けベースライン更新
- *2027-11-07 進捗*: W4 Step1（ゲート設定）として OCaml 側資産を再検証し、`reports/dual-write/front-end/w4-diagnostics/baseline/` に成果物を集約した。  
  - `npm ci && npm run ci --prefix tooling/lsp/tests/client_compat` を実行し、LSP V2 フィクスチャ 9 件の pass を確認。  
  - `scripts/validate-diagnostic-json.sh $(cat tmp/w4-parser-diag-paths.txt)` で Schema v2.0.0-draft を 10 ケース通過させ、リスト外だった `compiler/ocaml/tests/golden/diagnostics/effects/syntax-constructs.json.golden` は診断 JSON ではないため `TODO: DIAG-RUST-03` として別扱いにした。  
  - `collect-iterator-audit-metrics.py --section parser|effects|streaming` の結果を `parser-metrics.ocaml.json` / `effects-metrics.ocaml.json` / `streaming-metrics.ocaml.json` に保存し、`domain/multi-domain.json.golden` の audit メタデータ不足で `diagnostic.audit_presence_rate` が 0.7 から上がらないことを確認。是正タスクは `TODO: DIAG-RUST-04`（`docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md`）で追跡する。  
- Rust 側 dual-write を始める前に上記 TODO を解消し、OCaml 基準の完全通過を達成することが W4 Step2 以降の着手条件となる。
