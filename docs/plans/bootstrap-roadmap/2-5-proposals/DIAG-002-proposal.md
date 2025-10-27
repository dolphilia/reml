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
   - **調査結果（Week31 Day1 更新）**:  
     - 直接構築経路は `compiler/ocaml/src/diagnostic.ml:181`（Legacy 変換）のみ。Builder 経路へ差し替えない限り `audit = None` が残り続けるため、Day2 の必須化作業に組み込む。  
     - `Diagnostic.Builder.create` → `Builder.build` の直列利用経路は Lexer/Parser エラー、LLVM verify 失敗、CLI ゴールデン（計 4 件）。特に `compiler/ocaml/src/llvm_gen/verify.ml:131` は `main.ml:597` から `attach_audit` を通さず出力しており、`tooling/ci/collect-iterator-audit-metrics.py` が期待する `cli.audit_id` / `cli.change_set` 等が欠落している。  
     - 詳細は [`docs/plans/bootstrap-roadmap/2-5-review-log.md`](../2-5-review-log.md) に記録済み。High 優先経路は Week31 Day2 の修正対象とし、Medium/Low は Day3 以降の改善案に回す。
2. **型定義とビルダー更新（Week31 Day2）**  
   - **2.1 型定義の必須化**: `compiler/ocaml/src/diagnostic.ml` の `type t` から `audit : Audit_envelope.t option` / `timestamp : string option` を除外し、`Audit_envelope.t` / `Audit_envelope.timestamp` を直接保持する。`Diagnostic.make` / `make_with_defaults` / `of_builder` など全コンストラクタを `ensure_audit : Audit_envelope.t option -> Audit_envelope.t` と `ensure_timestamp : string option -> string` 経由で初期化し、`docs/spec/3-6-core-diagnostics-audit.md` に定義された ISO8601 (`YYYY-MM-DDThh:mm:ssZ`) を生成する補助関数を追加する。  
   - **2.2 ビルダー API 再設計**: `Diagnostic.Builder.state` に `audit_seed : Audit_envelope.t option` と `timestamp_seed : string option` を保持させ、`Builder.attach_audit_metadata` が `Audit_envelope.merge` で必須キー（`cli.audit_id`, `cli.change_set`, `schema.version`）を埋められない場合は `Audit_envelope.metadata["missing"]` に追記し、`Builder.build` 内で `ensure_audit` / `ensure_timestamp` を必ず通す。`merge_audit_metadata`（compiler/ocaml/src/diagnostic.ml:245-273）には `assert (Audit_envelope.has_required_keys audit)` を追加し、CI で違反を即検出できるようにする。  
   - **2.3 呼び出しサイトの順守強化**: `Diagnostic.Builder.create` 直後に `Builder.build` しているパス（Lexer/Parser/LLVM verify 失敗など既存レビューで洗い出した 4 件）に対し、`Builder.with_timestamp now` を差し込み、`attach_audit` 経路を共通化する。とくに `compiler/ocaml/src/llvm_gen/verify.ml:131` の `--verify-ir` 失敗診断は `Diagnostic.Builder` を利用するよう書き換え、`Diagnostic.set_audit_id` / `set_change_set` で CLI 監査キーを埋める（`tooling/ci/collect-iterator-audit-metrics.py` が要求するキー群を満たす）。  
   - **2.4 完了条件**: `rg "audit :.*option"` でヒットが 0 件になること、`rg "timestamp :.*option"` が 0 件になること、`dune build compiler/ocaml` を実行して型エラーが残っていないこと、`scripts/validate-diagnostic-json.sh`（必須フィールドを参照するゴールデンテスト）が通過することを Day2 終了時に確認し、結果を `docs/plans/bootstrap-roadmap/2-5-review-log.md` に追記する。
3. **Legacy / シリアライズ整備（Week31 Day2-3）**  
   - 調査  
     - `diagnostic_of_legacy_internal`（compiler/ocaml/src/diagnostic.ml:213）で既に `Audit_envelope.merge_metadata` が呼ばれていることを確認し、`Legacy.audit_metadata` が空のケースが残っていないか `find compiler -name '*.ml*' | xargs grep -n "Diagnostic.Legacy"` を実行して棚卸しする。新規利用を防ぐため、`[@deprecated]` 警告を CI でエラー扱いにする運用案を決定する。  
     - シリアライズ層（compiler/ocaml/src/diagnostic_serialization.ml:1-120, json_formatter.ml:1-160）で `audit` / `timestamp` がどこで欠落し得るかを洗い出し、`attach_audit`（compiler/ocaml/src/main.ml:640-760 付近）と `collect-iterator-audit-metrics.py` の要求キー一覧を突き合わせてギャップが無いか確認する。  
     - **進捗メモ（本タスク時点）**  
       - `Diagnostic.Legacy` の直接構築箇所は `diagnostic_of_legacy_internal` の 1 箇所のみで、`[@deprecated]` 属性による抑止を維持しつつ Legacy 経路でも `cli.audit_id` / `cli.change_set` を自動補完するよう改修した（compiler/ocaml/src/diagnostic.ml:333-370）。  
       - `Audit_envelope.has_required_keys` を `cli.audit_id` / `cli.change_set` まで拡張し、Builder/Legacy 双方で空値にプレースホルダを付与したうえで必須キーを強制する仕組みを導入した（compiler/ocaml/src/audit_envelope.ml:120-189, compiler/ocaml/src/diagnostic.ml:304-331）。  
       - `Diagnostic_serialization.of_diagnostic` は必須キーと `timestamp` を検証し、欠落時に `[diagnostic_serialization]` でログを出力して `Invalid_argument` を送出するようになった（compiler/ocaml/src/diagnostic_serialization.ml:75-88）。  
   - 実装  
     - `diagnostic_of_legacy_internal` に `Audit_envelope.has_required_keys` のログ出力／例外を追加し、Legacy 変換経路で必須メタデータが欠けた場合は即座に検出できるようにする。必要なら `Audit_envelope.ensure_missing_metadata` の必須キー集合を `cli.audit_id` / `cli.change_set` 以外にも拡張する。  
     - `Diagnostic_serialization.of_diagnostic` 内で `Audit_envelope.has_required_keys diag.audit` と `String.length diag.timestamp > 0` を検査し、違反時に `Invalid_argument` を発生させたうえで CLI/LSP 側でエラーメッセージを出力するフローを定義する。  
     - **監査 ID / change_set ポリシー策定（2025-11-05 合意）**  
       - `auto-*` / `legacy-*` のハッシュプレースホルダを廃止し、`docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` §0.3.1 で管理する `AuditEnvelope.build_id` を基点とした公式フォーマットへ移行する。  
       - CLI/CI から発行される診断は `audit_id = "cli/" ^ build_id ^ "#" ^ sequence` を既定とし、`sequence` は同一ビルド内での発生順を 0 始まりでインクリメントする。LSP セッションは `audit_id = "lsp/" ^ session_uuid ^ "#" ^ sequence`、Legacy 変換は `audit_id = "legacy-import/" ^ build_id` を用いる。  
       - `change_set` 自動補完は以下の JSON テンプレートを基準とし、`origin` に `cli` / `lsp` / `legacy` を記録、`source.commit` に Git SHA、`source.workspace` にリポジトリ内の相対パスを格納する。  
         ```json
         {
           "policy": "phase2.5.audit.v1",
           "origin": "<channel>",
           "source": {"commit": "<git sha>", "workspace": "<relative path>"},
           "items": []
         }
         ```  
       - OCaml 実装では `ensure_audit_id` / `ensure_change_set` を上記フォーマットへ差し替え、`Audit_envelope.metadata["audit.policy.version"] = "phase2.5.audit.v1"` を自動付与する。CLI 側で `cli.audit_id` / `cli.change_set` が渡された場合はそれらを優先し、不足時のみテンプレートを適用する。  
     - **実施状況メモ（2025-11-06 更新）**  
       - `Audit_envelope.has_required_keys` の検査対象を `cli.audit_id` / `cli.change_set` まで広げ、Builder と Legacy 変換の双方で自動補完後に必須キーを強制するよう調整済み。  
       - `Diagnostic_serialization.of_diagnostic` の防御的チェックを実装済みで、欠落発生時には `stderr` へ通知して `Invalid_argument` を送出する。  
       - CLI 経路（`compiler/ocaml/src/main.ml`）とテスト用ビルダー（`test_cli_diagnostics.ml`、`test_ffi_contract.ml`、`test_effect_residual.ml`）を `phase2.5.audit.v1` テンプレートへ移行し、`cli/<build_id>#<sequence>` 形式の監査 ID と `policy/origin/source/items` を備えた change-set を一貫して出力する。  
   - 検証  
     - `compiler/ocaml/tests/test_cli_diagnostics.ml` と `tooling/ci/collect-iterator-audit-metrics.py` のテスト入力を用意し、`dune runtest compiler/ocaml/tests/test_cli_diagnostics.ml` → `scripts/validate-diagnostic-json.sh` → `python3 tooling/ci/collect-iterator-audit-metrics.py --source ...` の順で実行して欠落フィールドが検出されることを確認する。  
     - JSON スキーマは既に `audit` / `timestamp` を必須化済み（tooling/json-schema/diagnostic-v2.schema.json:6-15）であるため、AJV テスト（tooling/lsp/tests/client_compat/validate-diagnostic-json.mjs）に欠落ケースのフィクスチャを追加し、Day3 に差分レビューを実施する。  
     - **検証状況メモ（2025-11-06 更新）**  
       - `dune runtest`（compiler/ocaml）で CLI / FFI / 効果系ゴールデンを再生成し、新しい監査テンプレートへの移行後もテストが通過することを確認。  
       - `python3 tooling/ci/collect-iterator-audit-metrics.py --require-success` をローカル実行し、`iterator.stage.audit_pass_rate`・`typeclass.dictionary_pass_rate`・`ffi_bridge.audit_pass_rate` の各指標が 1.0 を達成することを確認（従来の `auto-*` / `legacy-*` プレースホルダで発生していた欠落は解消済み）。  
   - ドキュメント・フォロー  
     - `docs/plans/bootstrap-roadmap/2-5-review-log.md` に Legacy 経路の監査結果を追記し、`ffi_bridge.audit_pass_rate` のゴールデン再生成手順を整理する。`docs/spec/3-6-core-diagnostics-audit.md` へ Legacy 経路の対応状況を脚注で共有し、Phase 2-7 の監査ダッシュボード更新タスクと連携する。  
     - **ドキュメント更新メモ**  
       - Legacy 経路の現状と TODO を `docs/plans/bootstrap-roadmap/2-5-review-log.md` に追記済み。仕様脚注とメトリクス更新は Week31 Day3 以降のフォローアップとする。
4. **監査メトリクス連携（Week31 Day3-4）**  
   - `tooling/ci/collect-iterator-audit-metrics.py` のエラー報告を強化し、必須フィールド欠落時に `pass_rate` を 0.0 へ落とすだけでなく、欠落行を明示するログを出力する。  
   - `0-3-audit-and-metrics.md` に `diagnostic.audit_presence_rate` を追加入力し、CI でトラッキングする。  
   - GitHub Actions の Linux/Windows/macOS すべてで当該スクリプトを実行し、欠落時はジョブが失敗するゲートを組み込む。  
   - **進捗メモ（2025-11-02）**: スクリプトに `stderr` ログを追加済み。`--require-success` 実行時に不足フィールドがファイル・インデックスと共に出力される。  
   - **進捗メモ（2025-11-07 更新）**: `diagnostic.audit_presence_rate` をスクリプト出力へ追加し、欠落検知時は `pass_rate = 0.0` へ丸める仕様を実装。Linux / macOS / Windows 各ワークフローで `--require-success` を有効化し、`0-3-audit-and-metrics.md` の指標表へ新メトリクスを追記した。  
5. **テストとドキュメント反映（Week31 Day4-5）**  
   - `compiler/ocaml/tests/golden/` 配下の診断 JSON を再生成し、`scripts/validate-diagnostic-json.sh` と `reports/diagnostic-format-regression.md` の手順でレビューする。  
   - `docs/spec/3-6-core-diagnostics-audit.md` に OCaml 実装の必須化完了を脚注として追記し、監査キー一覧の最新版を反映する。  
   - `docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md` に、Phase 2-7 で監査ダッシュボードへ新フィールドを反映するタスクを追加する。  
   - **進捗メモ（2025-11-02）**: Typeclass / FFI / Effects 系ゴールデンを更新し、`dune runtest` が通過することを確認済み。仕様ドキュメントの脚注更新は後続タスクへ引き継ぐ。  

## 残課題
- `Audit_envelope.empty_envelope` に含める既定値（`audit_id` / `change_set` の扱い）について監査チームの合意が必要。  
- LSP セッションおよび Legacy 経路向けの `phase2.5.audit.v1` テンプレート適用タイミングを整理し、CLI と同じビルド ID／シーケンス規約を維持できるようガイドラインを追記する。  
- `timestamp` の生成を `Core.Numeric.now()` へ委譲するか、OCaml 側で `Ptime` / `Unix.gmtime` を利用するかを選定したい。
