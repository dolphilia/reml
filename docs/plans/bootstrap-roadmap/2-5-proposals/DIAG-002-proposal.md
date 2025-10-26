# DIAG-002 `Diagnostic.audit` / `timestamp` 必須化計画

## 1. 背景と症状
- Chapter 3 では `Diagnostic` 構造体の `audit` / `timestamp` を必須フィールドとして定義し、監査ログとの整合を保証する（docs/spec/3-6-core-diagnostics-audit.md:21-40）。  
- 現行 OCaml 実装は `Diagnostic.audit : Audit_envelope.t option`、`timestamp : string option` としており、空のまま CLI/LSP へ出力される（compiler/ocaml/src/diagnostic.ml:120-135）。  
- `Diagnostic.diagnostic_of_legacy` が `audit = None` を返すため、旧来フローから変換した診断が監査必須値を持たないまま残存する（compiler/ocaml/src/diagnostic.ml:181-203）。  
- `Diagnostic.Builder.build` は `audit_metadata` が空のままだと `audit` を設定しない実装であり、`Builder` 利用箇所でも監査未設定が発生し得る（compiler/ocaml/src/diagnostic.ml:818-900）。  
- 監査集計スクリプトは `audit`/`audit_metadata` が埋まっている前提で必須キーを検証しているが（tooling/ci/collect-iterator-audit-metrics.py:61-174）、欠落診断はスキップされるため `ffi_bridge.audit_pass_rate` / `iterator.stage.audit_pass_rate` が 0.0 のまま放置されている（docs/plans/bootstrap-roadmap/2-4-completion-report.md:43-60）。

## 2. Before / After
### Before
- 診断生成時に `AuditEnvelope` が省略され、監査ログへイベントが記録されない。  
- `timestamp` が未設定の場合、`Diagnostic` を JSON へシリアライズするとフィールド自体が欠落し、CI スキーマ検証は通過するが監査要件を満たせない。

### After
- `Diagnostic.t` の `audit` / `timestamp` を必須フィールドに変更し、生成時に空の `Audit_envelope.empty_envelope` と ISO8601 時刻を自動設定する。  
- 監査情報が未入力の診断は `AuditEnvelope.metadata["missing"] = ["field"]` を追加し、`0-4-risk-handling.md` に記録できるようにする。  
- CLI/LSP で JSON 出力時に常に `audit` / `timestamp` が含まれ、Phase 2-8 の監査パイプラインが仕様通りに動作する。

#### 実装イメージ
```ocaml
let now () = Audit_envelope.iso8601_timestamp ()
let ensure_audit audit_opt =
  match audit_opt with
  | Some audit -> audit
  | None -> Audit_envelope.empty_envelope
```

## 3. 影響範囲と検証
- **スキーマ**: `tooling/json-schema/diagnostic-v2.schema.json` を更新し、`audit` / `timestamp` を必須化。`scripts/validate-diagnostic-json.sh` を用いて CLI/LSP 双方のゴールデンが通過することを確認し、`reports/diagnostic-format-regression.md` のチェックリストを更新する。  
- **OCaml 実装**: `compiler/ocaml/src/diagnostic.ml` / `compiler/ocaml/src/diagnostic_builder.ml` / `compiler/ocaml/src/diagnostic_serialization.ml` を改修し、`Diagnostic.make` 系 API・`Diagnostic.Builder`・`diagnostic_of_legacy` の全パスで `audit` / `timestamp` が `None` にならないよう保証する。`compiler/ocaml/tests/test_diagnostic.ml`（新設予定）または既存テストに欠落フィールドを禁止するケースを追加する。  
- **CI メトリクス**: `ffi_bridge.audit_pass_rate` / `iterator.stage.audit_pass_rate` が 1.0 になることを GitHub Actions（Windows/macOS 含む）で検証。  
- **互換性**: 旧 `Legacy` 変換で `timestamp` / `audit` を補完する処理を追加し、CLI テキスト出力刷新（Phase 2-7）と整合を取る。

## 4. フォローアップ
- `reports/diagnostic-format-regression.md` に監査フィールド付きサンプルを追加し、レビューチェックリストに「必須フィールド欠落禁止」を明記。  
- `docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md` の監査ダッシュボード更新タスクへ「新必須フィールドの統計」を追記。  
- `docs/spec/3-6-core-diagnostics-audit.md` に OCaml 実装状況を脚注で追加し、必須化時期を明確化する。  
- `docs/notes/diagnostic-audit-gap.md`（未作成の場合は新設）へ必須化の背景と移行チェックリストを記録し、Phase 3 のセルフホスト側でも監査欠落が再発しないようトレーサビリティを確保する。
- **タイミング**: Phase 2-5 の立ち上がり直後に優先対応し、Phase 2-6 へ移行する前までに必須化とメトリクス回復を完了する。

## 5. 実施ステップ
1. **現状洗い出し（Week31 Day1）**  
   - `Diagnostic` を直接構築している箇所を棚卸しし、`Diagnostic.Builder` を経由していない呼び出し（例: `diagnostic_of_legacy`、テスト用ユーティリティ）を一覧化する。  
   - `audit_metadata` を空のまま返すコンビネータ（`Diagnostic.Builder.create` 直後に `Builder.build` しているケースなど）を調査し、監査キーが付与されない経路を `docs/plans/bootstrap-roadmap/2-5-review-log.md` に記録する。
2. **型定義とビルダー更新（Week31 Day2）**  
   - `Diagnostic.t` の `audit` / `timestamp` を必須型へ変更し、`Diagnostic.Builder.build` で必ず `Audit_envelope.empty_envelope` と `iso8601_timestamp` を設定する。  
   - `merge_audit_metadata` の事前条件を更新し、監査メタデータによって `audit` が上書きされても必須項目が残るようアサーションを追加する（compiler/ocaml/src/diagnostic.ml:245-273）。
3. **Legacy / シリアライズ整備（Week31 Day2-3）**  
   - `diagnostic_of_legacy` に `audit` 自動補完を追加し、`Legacy` からの移行で必須値が失われないようにする。  
   - `Diagnostic_serialization.of_diagnostic` と JSON 出力（CLI/LSP）に対して、`audit` / `timestamp` が欠落した場合は例外または警告を発生させるチェックを挿入する（compiler/ocaml/src/diagnostic_serialization.ml:59-84）。  
   - `tooling/json-schema/diagnostic-v2.schema.json` の `required` 配列へ `audit` / `timestamp` を追加し、AJV テストを更新する。
4. **監査メトリクス連携（Week31 Day3-4）**  
   - `tooling/ci/collect-iterator-audit-metrics.py` のエラー報告を強化し、必須フィールド欠落時に `pass_rate` を 0.0 へ落とすだけでなく、欠落行を明示するログを出力する。  
   - `0-3-audit-and-metrics.md` に `diagnostic.audit_presence_rate` を追加入力し、CI でトラッキングする。  
   - GitHub Actions の Linux/Windows/macOS すべてで当該スクリプトを実行し、欠落時はジョブが失敗するゲートを組み込む。
5. **テストとドキュメント反映（Week31 Day4-5）**  
   - `compiler/ocaml/tests/golden/` 配下の診断 JSON を再生成し、`scripts/validate-diagnostic-json.sh` と `reports/diagnostic-format-regression.md` の手順でレビューする。  
   - `docs/spec/3-6-core-diagnostics-audit.md` に OCaml 実装の必須化完了を脚注として追記し、監査キー一覧の最新版を反映する。  
   - `docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md` に、Phase 2-7 で監査ダッシュボードへ新フィールドを反映するタスクを追加する。

## 残課題
- `Audit_envelope.empty_envelope` に含める既定値（`audit_id` / `change_set` の扱い）について監査チームの合意が必要。  
- `timestamp` の生成を `Core.Numeric.now()` へ委譲するか、OCaml 側で `Ptime` / `Unix.gmtime` を利用するかを選定したい。
