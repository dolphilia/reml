# DIAG-002 `Diagnostic.audit` / `timestamp` 必須化計画

## 1. 背景と症状
- Chapter 3 では `Diagnostic` 構造体の `audit` / `timestamp` を必須フィールドとして定義し、監査ログとの整合を保証する（docs/spec/3-6-core-diagnostics-audit.md:21-40）。  
- 現行 OCaml 実装は `Diagnostic.audit : Audit_envelope.t option`、`timestamp : string option` としており、空のまま CLI/LSP へ出力される（compiler/ocaml/src/diagnostic.ml:120-134）。  
- `collect-iterator-audit-metrics.py` が `AuditEnvelope` 欠落を検知できず、`ffi_bridge.audit_pass_rate` / `iterator.stage.audit_pass_rate` が 0.0 のまま放置されている（docs/plans/bootstrap-roadmap/2-4-completion-report.md:43-60）。

## 2. Before / After
### Before
- 診断生成時に `AuditEnvelope` が省略され、監査ログへイベントが記録されない。  
- `timestamp` が未設定の場合、`Diagnostic` を JSON へシリアライズするとフィールド自体が欠落し、CI スキーマ検証は通過するが監査要件を満たせない。

### After
- `Diagnostic.t` の `audit` / `timestamp` を必須フィールドに変更し、生成時に空の `AuditEnvelope.empty()` と ISO8601 時刻を自動設定する。  
- 監査情報が未入力の診断は `AuditEnvelope.metadata["missing"] = ["field"]` を追加し、`0-4-risk-handling.md` に記録できるようにする。  
- CLI/LSP で JSON 出力時に常に `audit` / `timestamp` が含まれ、Phase 2-8 の監査パイプラインが仕様通りに動作する。

#### 実装イメージ
```ocaml
let now () = Audit_envelope.iso8601_timestamp ()
let ensure_audit audit_opt =
  match audit_opt with
  | Some audit -> audit
  | None -> Audit_envelope.empty ()
```

## 3. 影響範囲と検証
- **スキーマ**: `tooling/json-schema/diagnostic-v2.schema.json` を更新し、`audit` / `timestamp` を必須化。`scripts/validate-diagnostic-json.sh` で全フィクスチャが通過することを確認。  
- **OCaml 実装**: `compiler/ocaml/src/diagnostic.ml` および `compiler/ocaml/src/diagnostic_builder.ml` で `audit` / `timestamp` を生成時に強制するリグレッションテストを追加し、`diagnostic_tests.ml` に「欠落フィールドが存在すると例外を投げる」ケースを新設。  
- **CI メトリクス**: `ffi_bridge.audit_pass_rate` / `iterator.stage.audit_pass_rate` が 1.0 になることを GitHub Actions（Windows/macOS 含む）で検証。  
- **互換性**: 旧 `Legacy` 変換で `timestamp` / `audit` を補完する処理を追加し、CLI テキスト出力刷新（Phase 2-7）と整合を取る。

## 4. フォローアップ
- `reports/diagnostic-format-regression.md` に監査フィールド付きサンプルを追加し、レビューチェックリストに「必須フィールド欠落禁止」を明記。  
- `docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md` の監査ダッシュボード更新タスクへ「新必須フィールドの統計」を追記。  
- `docs/spec/3-6-core-diagnostics-audit.md` に OCaml 実装状況を脚注で追加し、必須化時期を明確化する。  
- `docs/notes/diagnostic-audit-gap.md`（未作成の場合は新設）へ必須化の背景と移行チェックリストを記録し、Phase 3 のセルフホスト側でも監査欠落が再発しないようトレーサビリティを確保する。
- **タイミング**: Phase 2-5 の立ち上がり直後に優先対応し、Phase 2-6 へ移行する前までに必須化とメトリクス回復を完了する。

## 残課題
- `AuditEnvelope.empty()` に含める既定値（`audit_id` / `change_set` の扱い）について監査チームの合意が必要。  
- `timestamp` の生成を `Core.Numeric.now()` へ委譲するか、OCaml 側で `Ptime` / `Unix.gmtime` を利用するかを選定したい。
