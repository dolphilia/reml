# 2-5 レビュー記録 — DIAG-002 Day1 調査

DIAG-002 の初期洗い出し結果を記録し、後続フェーズでの追跡に利用する。  
関連計画: [`docs/plans/bootstrap-roadmap/2-5-proposals/DIAG-002-proposal.md`](./2-5-proposals/DIAG-002-proposal.md)

## 1. Diagnostic を直接構築している経路
| 種別 | ファイル:行 | 状態 | 想定対応 |
|------|-------------|------|----------|
| Legacy 変換 | `compiler/ocaml/src/diagnostic.ml:181` | `Diagnostic.Legacy.t` から `Diagnostic.t` をレコード直接構築。`audit = None` のまま返却され、`Legacy.audit_metadata` が空の場合は監査キーが欠落する。 | Week31 Day2 以降で `Diagnostic.Builder` 経由の移行パスを追加し、最低限 `Audit_envelope.empty_envelope` と `iso8601_timestamp` を強制する。既存のテストは Builder 経路へ切り替える。 |

## 2. 監査メタデータが不足する経路（`Diagnostic.Builder.create` → `Builder.build`）
| 優先度 | ファイル:行 | 出力チャネル | 現状 | 対応メモ |
|--------|-------------|--------------|--------|----------|
| 高 | `compiler/ocaml/src/llvm_gen/verify.ml:131` | `--verify-ir` 失敗時 (CLI) | `Builder.build` 直後の診断をそのまま `main.ml:597` から出力。`attach_audit` が呼ばれないため `cli.audit_id` / `cli.change_set` など `tooling/ci/collect-iterator-audit-metrics.py` が必須とするキーが欠落し、`ffi_bridge.audit_pass_rate` 集計で非準拠扱い。 | Day2 で `Verify.error_to_diagnostic` に `Diagnostic.set_audit_id` / `set_change_set` を注入するか、`main.ml` 側で再利用している `attach_audit` を適用する。 |
| 中 | `compiler/ocaml/src/diagnostic.ml:945` | `Parser_driver.process_lexer_error` | Builder 直後は監査メタデータが空だが、`main.ml:803` で `attach_audit` を通すため CLI/LSP 出力時点では `cli.audit_id` / `cli.change_set` が補完される。 | 現状維持でも仕様違反にはならないが、計測ログ用の `parser.*` 系キーを Builder 側で自動付与する改善案を検討。 |
| 中 | `compiler/ocaml/src/diagnostic.ml:950` | `Parser_driver.process_parser_error` | Lexer エラーと同じ挙動。`attach_audit` により最終的な監査キーは揃う。 | Parser 向けメタデータ自動化を Lexer と合わせて検討。 |
| 低 | `compiler/ocaml/tests/test_cli_diagnostics.ml:27` | CLI フォーマッタのゴールデン | テスト専用のダミー診断。監査キーが空のままのため、必須化後は `Diagnostic.set_audit_id` 等でフィクスチャを更新する必要がある。 | Day3 以降でゴールデン再生成。レビュー時に `REMLC_FIXED_TIMESTAMP` を考慮。 |

## 3. 補足メモ
- `main.ml:665-694` の Core IR / Codegen 例外、`main.ml:744-748` の型推論エラー、`main.ml:803-804` のパース失敗は `attach_audit` を経由しており、`cli.audit_id`・`cli.change_set` が付与される。
- `tooling/ci/collect-iterator-audit-metrics.py` は 14 件の audit メタデータキーを必須としている。High 優先度の経路から出力される診断は pass rate を 0.0 に固定する要因となるため、Phase 2-5 内での修正を優先する。*** End Patch*** End Patch

## 4. Legacy / シリアライズ整備 進捗（2025-11-02 更新）
- **監査キー補完**: Builder/Legacy 双方で `ensure_audit_id` / `ensure_change_set` を導入し、空値の場合は `phase2.5.audit.v1` テンプレート（CLI: `audit_id = "cli/" ^ build_id ^ "#" ^ sequence`、Legacy: `audit_id = "legacy-import/" ^ build_id`）を生成してから `Audit_envelope.has_required_keys` を通過させる。`missing` フィールドは必須キーが揃った段階で自動的に除去される（compiler/ocaml/src/diagnostic.ml:304-370）。
- **Audit_envelope 拡張**: `Audit_envelope.has_required_keys` を CLI 監査キー込みで再定義し、`missing_required_keys` を公開して検証・エラーメッセージ両方に利用できるようにした（compiler/ocaml/src/audit_envelope.ml:120-189）。
- **シリアライズ検証**: `Diagnostic_serialization.of_diagnostic` で必須キーと `timestamp` をチェックし、欠落時は `[diagnostic_serialization] …` を stderr に出力して `Invalid_argument` を送出する運用へ移行した（compiler/ocaml/src/diagnostic_serialization.ml:75-88）。
- **テスト/ログ**: `dune runtest`（compiler/ocaml）を再実行し、更新された診断ゴールデン（Typeclass/FFI/Effects）を整合させた。`tooling/ci/collect-iterator-audit-metrics.py` は不足フィールドを stderr に出力するようになり、`--require-success` 実行時のトラブルシューティングが容易になった。
