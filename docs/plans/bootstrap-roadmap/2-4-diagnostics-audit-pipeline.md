# 2.4 診断・監査パイプライン強化計画

## 目的
- Phase 2 マイルストーン M3 で必要となる `Diagnostic` + `AuditEnvelope` の完全実装を実現し、監査ログのフォーマットを仕様と同期させる。
- 効果システム・FFI 拡張など他タスクのメタデータを統合し、Phase 4 の移行期に備える。

## スコープ
- **含む**: 診断データ構造拡張、`extensions` フィールド設計、JSON/テキスト両方の出力整備、監査ログの永続化、レビューツール。
- **含まない**: 外部監査システム連携、GUI ビューワ。必要に応じて Phase 4 で検討。
- **前提**:
  - Phase 1 の CLI 整備が完了し、診断結果を CLI から閲覧できる状態であること。
  - Phase 2-3 完了報告およびハンドオーバー（`docs/plans/bootstrap-roadmap/2-3-completion-report.md`, `docs/plans/bootstrap-roadmap/2-3-to-2-4-handover.md`）を確認し、`ffi_bridge.audit_pass_rate`・`bridge.*` フィールドが有効であること。
  - 技術的負債 ID 22（Windows Stage 自動検証不足）と ID 23（macOS FFI サンプル自動検証不足）を解消する計画を本フェーズのタスクに組み込むこと。
  - `tooling/runtime/audit-schema.json` v1.1 を基準スキーマとして採用し、差分変更が必要な場合は Phase 2-3 チームと調整する。

## 引き継ぎタスク対応計画

### ID 22: Windows Stage 自動検証不足の解消
- **目的**: GitHub Actions (windows-latest) 上で `tooling/ci/sync-iterator-audit.sh` を実行し、`iterator.stage.audit_pass_rate` および `bridge.platform` が `tooling/runtime/capabilities/default.json` に定義された Stage と整合することを CI で保証する。
- **作業ステップ**:
  1. `tooling/ci/sync-iterator-audit.sh` を Windows bash（GitHub Hosted Agent の `C:\msys64\usr\bin\bash.exe`）で動作するようにパス解決と一時ディレクトリ処理を調整し、`--emit-audit` を Windows 出力パスへ書き出す。
  2. `tooling/ci/collect-iterator-audit-metrics.py` に `--platform windows-msvc` プリセットを追加し、`bridge.platform = windows-msvc` の監査行のみで `ffi_bridge.audit_pass_rate` と `iterator.stage.audit_pass_rate` を算出。失敗時は非ゼロ終了コードでジョブを停止させる。
  3. `/.github/workflows/bootstrap-windows.yml`（または相当の Phase 2 ワークフロー）へ新規ジョブ `audit-ffi-stage` を追加し、`actions/setup-python` と `choco install msys2` を用いた bash 実行環境で上記スクリプト群を呼び出す。成果物（`cli-callconv-*.audit.jsonl`, `iterator-stage-summary.md`）をアップロードし、PR チェックに pass_rate < 1.0 の場合は失敗を返す。
  4. `reports/ffi-bridge-summary.md` と `reports/runtime-capabilities-validation.md` に Windows CI 実行ログの参照リンクを追記し、レビュー時に監査結果を追跡できるよう更新。
- **完了条件**:
  - GitHub Actions Windows ジョブが `ffi_bridge.audit_pass_rate` / `iterator.stage.audit_pass_rate` の両方を 1.0 で確認し、閾値未満の場合に PR をブロックする。
  - 監査ログ成果物のパスと命名規約を `docs/spec/3-6-core-diagnostics-audit.md` 付録、`docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` に反映。
  - `compiler/ocaml/docs/technical-debt.md` の ID 22 を「完了」に更新し、対応コミット・ワークフロー名を記録。

### ID 23: macOS FFI サンプル自動検証不足の解消
- **目的**: `ffi_dispatch_async.reml` と `ffi_malloc_arm64.reml` のビルド・実行を CI に組み込み、`bridge.platform = macos-arm64` の監査ログを `ffi_bridge.audit_pass_rate` に反映させる。
- **作業ステップ**:
  1. `examples/ffi/ffi_dispatch_async.reml`（および `ffi_malloc_arm64.reml`）向けに `scripts/ci-local.sh --target macos --arch arm64` の Test ステップへ統合する実行ルールを追加し、`tmp/cli-callconv-out/macos/` に成果物を保存する。
  2. `tooling/ci/sync-iterator-audit.sh` に macOS arm64 専用ターゲット `--macos-ffi-samples` を追加し、前述成果物から `cli-callconv-macos.audit.jsonl` / `ffi_dispatch_async.audit.jsonl` を `tooling/ci/ffi-audit/macos/` 配下へ同期する。
  3. `collect-iterator-audit-metrics.py` で `macos-arm64` の pass_rate 算出時に `ffi_dispatch_async`・`ffi_malloc_arm64` の監査行を必須にし、欠落または `bridge.status != success` の場合は 0.0 に設定。
  4. `compiler/ocaml/tests/golden/audit` に macOS 専用サンプル用ゴールデン (`ffi-dispatch-async-macos.jsonl.golden` など) を新設し、`dune runtest` に組み込む。
  5. GitHub Actions の macOS ワークフロー（`bootstrap-macos.yml` など）に `audit-ffi-macos` ジョブを追加し、上記スクリプトを実行して生成ログをアーティファクト化。`ffi_bridge.audit_pass_rate` の閾値チェックをジョブの終了条件へ接続する。
  6. `reports/ffi-macos-summary.md` の TODO セクションに自動化完了の記録と最新ログのパスを追記し、Phase 3 での追加サンプル拡張手順を明示する。
- **完了条件**:
  - CI macOS ジョブで `ffi_dispatch_async.reml`／`ffi_malloc_arm64.reml` のビルド・実行が安定し、`ffi_bridge.audit_pass_rate (macos-arm64)` が 1.0 になる。
  - ゴールデンテストが `bridge.return.*` / `bridge.platform` を検証し、macOS 監査ログが欠落した場合に CI が失敗する。
  - 技術的負債 ID 23 が「完了」として更新され、監査ログ保存場所が `docs/plans/bootstrap-roadmap/2-3-to-2-4-handover.md` の参照リストに追加される。

### その他引き継ぎ事項の整理
- **`--verify-ir` 再有効化**: Phase 2-3 で stub 無終端問題が解消されているため、Phase 2-4 では `scripts/ci-local.sh` のデフォルトパスと CLI ドキュメントを更新し、すべてのプラットフォームワークフローで `--verify-ir` を再び必須化する。失敗時には監査ログとともに IR 検証レポートを収集し、`reports/ffi-bridge-summary.md` に参照を追加する。
- **CI ゲート統合**: Linux / Windows / macOS それぞれのワークフローに `ffi_bridge.audit_pass_rate` と `iterator.stage.audit_pass_rate` を共通ゲートとして設定し、閾値・通知先・再実行手順を `docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` のチェックリストへ反映する。
- **ドキュメント反映**: 監査ログ出力の保存先・命名規約・レビューフローを `docs/spec/3-6-core-diagnostics-audit.md` の付録へ追記し、`docs/guides/runtime-bridges.md` に CI 自動化手順を共有する。

## 作業ディレクトリ
- `compiler/ocaml/src` : Diagnostic/AuditEnvelope 生成プログラム
- `tooling/cli` : CLI 出力、`--emit-diagnostic` などの整形
- `tooling/lsp` : 将来の LSP 連携に向けた仕様メモ
- `tooling/ci` : 診断 diff / JSON スキーマ検証ワークフロー
- `docs/spec/3-6-core-diagnostics-audit.md`, `docs/notes/guides-to-spec-integration-plan.md` : スキーマ更新と追跡

## 作業ブレークダウン

### 1. 診断データ構造の再設計（26-27週目）
**担当領域**: 診断基盤設計

**現状整理**
- `compiler/ocaml/src/diagnostic.ml` の `type t` は Phase 1 の最小構成（`span`/`notes`/`fixits` 等）を維持しており、[3-6-core-diagnostics-audit.md](../../spec/3-6-core-diagnostics-audit.md) §1 に定義された `id` / `primary` / `secondary` / `hints` / `expected` / `timestamp` / `audit` を保持できていない。
- `compiler/ocaml/src/audit_envelope.ml` は JSON Lines 用の軽量 `event` レコードのみを提供し、仕様上の `AuditEnvelope`（`audit_id`・`change_set`・`metadata`）および監査イベント列挙と乖離している。
- `tooling/runtime/audit-schema.json` v1.1 は `bridge.*` メタデータを必須化済みだが、`Diagnostic` 側で `audit.metadata` を直列化する導線が未整備である。
- CLI 出力（`Cli.Diagnostic_formatter`、`Cli.Json_formatter`）とゴールデンテストは現行 `Diagnostic.t` のフィールド構成を前提としているため、再設計時は後方互換の時間帯を確保しつつ段階的切り替えが必要。

1.1. **Diagnostic 構造の拡張**
- 仕様差分整理: 既存 `Diagnostic.t` と仕様 `Diagnostic` レコードのフィールド比較表を作成し、欠落要素（`id`, `primary`, `secondary`, `hints`, `expected`, `audit`, `timestamp`, `domain` の Optional 化等）を列挙。結果は `docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` の「診断メトリクス」節へ参照を残す。
- OCaml 型ドラフト: `type span_label`, `type hint`, `type diagnostic_id = Uuidm.t option` など仕様準拠のサブ型を `compiler/ocaml/src/diagnostic.ml` に追加し、`primary: span`, `secondary: span_label list`, `hints: hint list`, `audit: Audit_envelope.t`, `timestamp: Core_time.timestamp` など最終構成をドラフト化。
- 拡張領域の再定義: `extensions` は `module Extensions : sig type t = (string * Yojson.Basic.t) list end` を存続させつつ `Diagnostic.extensions` を仕様上の `metadata` との対応表に整理。`related: Diagnostic_reference list`（ID リンク + フォールバック本文）と `codes: string list`（`primary` と別に複数コードを扱う）を新設し、LSP 連携に向けたキー体系案を提示。
- CLI/LSP 中間層影響: `Cli.Diagnostic_envelope` が新フィールドを受け取れるようインタフェースを起票し、LSP 変換（`diagnostic_to_lsp`）で `secondary` → `related_information`、`hints` → `CodeAction` 下準備ができるか検証する。

#### Diagnostic フィールド比較表（ドラフト） {#diagnostic-field-table-draft}

| 仕様フィールド ([3-6] §1) | 現行 `compiler/ocaml/src/diagnostic.ml` | 状態 | メモ |
|---------------------------|-----------------------------------------|------|------|
| `id: Option<Uuid>` | 未実装 | 不足 | CLI/LSP/監査のトレースキーとして導入予定。 |
| `message: Str` | `message: string` | 同等 | 命名と型は一致、国際化キー導線は別途。 |
| `severity: Severity` | `severity: severity` | 要素名差異 | 列挙子集合を仕様準拠（Error/Warning/Info/Hint）へ調整。 |
| `domain: Option<DiagnosticDomain>` | `domain: error_domain option` | 要素名差異 | 現行列挙値が仕様の `DiagnosticDomain` と不一致（`Runtime`/`Data` など）。統合が必要。 |
| `code: Option<Str>` | `code: string option` | 同等 | 命名統一のみ。 |
| `primary: Span` | `span: span` | 命名差異 | `span` を `primary` に改名し、構造体定義を仕様準拠に再掲。 |
| `secondary: List<SpanLabel>` | `notes: (span option * string) list` | 再設計 | メッセージ付き副位置を `SpanLabel` 型で再定義し、NULL 許容を廃止。 |
| `hints: List<Hint>` | `fixits: fixit list` | 再設計 | `Hint` と `FixIt` を区別し、仕様の `hints`（人間向け提案）と `fixits`（自動修正）を併存させる。 |
| `expected: Option<ExpectationSummary>` | `expected_summary: expectation_summary option` | 同等 | 命名・フィールド構成を仕様に合わせて再公開。 |
| `audit: AuditEnvelope` | `audit_metadata: Extensions.t` | 不足 | 仕様準拠の `AuditEnvelope` 型を保持し、`metadata` との整合を取る。 |
| `timestamp: Timestamp` | 未実装 | 不足 | `Core.Numeric.now()` で生成し、ソートとメトリクスに利用。 |
| `extensions: Map<Str, Json>` | `extensions: Extensions.t` | 同等 | `list` → `Map` 変換ルールを定義。 |
| `related: Diagnostic list` | 未実装 | 不足 | 親子診断、複合エラー向けのリンク機構を追加。 |
| `codes: List<Str>` | 未実装 | 不足 | 単一コード `code` から多重コード併記へ移行。 |

- 追加フィールドの扱い: `severity_hint`（Rollback/Retry 等）は CLI ガイダンスとして残置し、仕様側 `Hint` との位置付けを整理する。`notes` は LSP の `related_information` 用に `secondary` へ移譲し、名称衝突を解消する。
- フィールド比較表は移行ステップ策定時に随時更新し、最終版は Phase 2 終了レビューで確定する。

**進捗状況 (2025-10-21 更新)**
- Diagnostic.V2 型のドラフトを `compiler/ocaml/src/diagnostic.ml` に追加し、既存 `Diagnostic.t` からの変換ユーティリティ（`V2.of_legacy` など）を実装済み。
- `Cli.Json_formatter`／`Cli.Diagnostic_formatter` を V2 フィールドへ切り替え、JSON/テキスト両方で `codes`・`secondary`・`hints`・`timestamp`・`audit` を表示できるよう調整済み。
- `Diagnostic.Builder` を実装し、`Diagnostic.make`／`make_type_error`／Parser エラーパスを新 API 経由で構築するよう更新。既存テストは `dune runtest` で回帰なし。
- `dune runtest`（compiler/ocaml）で回帰がないことを確認済み。

**残タスク**
1. LSP トランスポート（`cli` 以外の JSON 出力、将来の LSP 実装）で V2 フィールドを公開する仕組みを整備し、クライアント側の互換性テストを追加する。
2. `Diagnostic.Builder` の補助関数を拡充し、`type_error.ml` 以外の診断生成サイト（効果・型クラス・CLI サブコマンド等）で複数コード／structured hints を活用できるよう段階移行する。
3. V2 導入に伴うゴールデンファイルの刷新と差分レビュー手順を策定し、`compiler/ocaml/tests/golden/diagnostics/*.json.golden` の更新計画をまとめる。

#### V2 昇格差分計画（2025-10-27 草案）

- **段階 A — 型定義と互換レイヤ整備**  
  - `Diagnostic.t` を V2 準拠フィールド（`id` / `primary` / `secondary` / `hints` / `timestamp` / `audit` 等）へ置換し、既存フィールドは `Legacy` レコードとして退避。  
  - `Diagnostic.V2` を `type t = Diagnostic_core.t` へ単純化し、`of_legacy` は段階的廃止。  
  - `Diagnostic.Builder` / `diagnostic_builder.mli` を新フィールド前提に再生成し、`build` が V2 レコードを直接返すよう調整。  
  - 影響ファイル: `compiler/ocaml/src/diagnostic.ml`, `compiler/ocaml/src/diagnostic_builder.{ml,mli}`, `compiler/ocaml/src/cli/diagnostic_formatter.ml`, `compiler/ocaml/src/cli/json_formatter.ml`

- **段階 B — 主要生成サイトの移行**  
  - `type_error.ml`, `parser_driver.ml`, `effects/type_inference_effect.ml`, `core_ir/iterator_audit.ml`, `tooling/cli/commands/*` で Builder API 生成へ統一。  
  - 既存ヘルパー (`make_type_error` 等) は Builder 呼び出しへ委譲し、戻り値を新 `Diagnostic.t` に更新。  
  - `@@deprecated` 属性を付与したラッパー（`Diagnostic_compat`）を 2 リリース分維持し、CI で使用箇所を警告化。  
  - 移行完了の判定条件: `rg "Diagnostic\.make"` が 0 件、`rg "V2.of_legacy"` が `cli` 層以外で 0 件になること。

- **段階 C — 出力とテストの更新**  
  - JSON/テキストフォーマッタの追加フィールド表示を確定し、`--format lsp-v2` / `--format json` のスナップショットを再取得。  
  - `compiler/ocaml/tests/golden/diagnostics` を 3 バッチ（型エラー → 効果/型クラス → CLI）で更新し、差分は `reports/diagnostic-migration.md` へ記録。  
  - LSP 互換テスト（`tooling/lsp/tests/client_compat`）に V2 フィールド検証ケースを追加し、`npm test` を CI に統合。  
  - フィールド追加後、`docs/spec/3-6-core-diagnostics-audit.md` の表を再生成し、`codes[]`・`hints[]`・`extensions` の例を更新。

#### 実装タスク (diagnostic.ml / CLI) {#diagnostic-migration-plan}

1. **下準備**
   - `compiler/ocaml/src/diagnostic.ml` に `Span_label`, `Hint`, `Audit_envelope` 参照を追加し、新型 `t` のドラフト実装をサンドボックスモジュール（例: `Diagnostic_v2`) として導入。
   - 既存 API (`make`, `of_lexer_error` 等) を `Diagnostic_compat` モジュールに退避し、段階的に新 `builder` API へ委譲するテストベッドを確保。
2. **CLI 出力層の対応順序**
   - `Cli.Json_formatter` → `Cli.Diagnostic_formatter` → `Cli.Diagnostic_envelope` の順で新フィールド対応を実施。最初に JSON 出力を更新し、スナップショットテストで欠落フィールドを検知しやすくする。
   - フォーマッタ更新後に `_build/default/src/main.exe` の出力を比較し、`diagnostic_regressions` 指標を監視。二段階目でテキスト出力の整形（secondary/hints 表示）を調整。
3. **コア処理系の置換**
   - `parser_driver.ml`、`type_error.ml`、`type_inference_effect.ml` の診断生成サイトを新 `builder` に差し替え、`related` / `codes` / `timestamp` を埋める。
   - `tooling/ci` と `compiler/ocaml/tests/golden/diagnostics` を更新し、旧フォーマットとの比較が容易な差分ログを `reports/diagnostic-migration.md`（新規）に記録。
4. **互換期間と削除**
   - CI で旧 API 利用箇所を警告化（`[@alert deprecated]` 等）し、Phase 2 終盤で `Diagnostic_compat` を削除する。削除前に `docs/plans/bootstrap-roadmap/0-4-risk-handling.md` で後方互換注意点を共有。

1.2. **AuditEnvelope との整合**
- 仕様準拠モデル: `compiler/ocaml/src/audit_envelope.ml` を `type t = { audit_id: Uuidm.t option; change_set: Yojson.Basic.t option; capability: Capability_id.t option; metadata: (string * Yojson.Basic.t) list }` に再定義し、`AuditEvent` 列挙（仕様 §1.1.1）および `metadata` の必須キーセットを OCaml のパターンで表現する。
- 共通ユーティリティ: `Diagnostic` から `AuditEnvelope` を直接参照できるよう `Audit_envelope.make` / `Audit_envelope.add_metadata` を整理し、`extensions` と `audit.metadata` の境界を明文化。`tooling/runtime/audit-schema.json` とのキー命名規約（`bridge.*`, `effects.*`, `cli.*` 等）を一覧化し、Phase 2-1/2-2 タスクへフィードバックする。
- スキーマバージョン管理: `audit_schema_version: string` を `Diagnostic` または `CliDiagnosticEnvelope` 側に保持し、`AuditEnvelope.metadata["schema.version"]` と同期させる運用を定義。更新履歴は `docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` へ追記する。
- Phase 2 他タスク連携: 型クラス（`extensions.typeclass.*`）、効果（`extensions.effect.*`）、FFI（`extensions.bridge.*`）で必要なメタデータをヒアリングし、フィールド拡張案を共有。必要に応じて `docs/spec/3-6` の付録へ追記するタスクリストを生成。

#### AuditEnvelope 再定義草案（OCaml） {#audit-envelope-draft}

```ocaml
module Audit_envelope = struct
  type metadata = (string * Yojson.Basic.t) list

  type t = {
    audit_id : Uuidm.t option;
    change_set : Yojson.Basic.t option;
    capability : Capability_id.t option;
    metadata : metadata;
  }

  type event =
    | Pipeline_started of pipeline_context
    | Pipeline_completed of pipeline_context
    | Pipeline_failed of pipeline_failure
    | Capability_mismatch of capability_mismatch
    | Async_supervisor_restarted of async_supervisor
    | Async_supervisor_exhausted of async_supervisor
    | Config_compat_changed of config_change
    | Env_mutation of env_mutation
    | Custom of string * Yojson.Basic.t

  val make :
    ?audit_id:Uuidm.t ->
    ?change_set:Yojson.Basic.t ->
    ?capability:Capability_id.t ->
    ?metadata:metadata ->
    unit ->
    t

  val add_metadata : t -> key:string -> Yojson.Basic.t -> t
  val merge_metadata : t -> metadata -> t
  val to_json : t -> Yojson.Basic.t
end
```

- `pipeline_context` / `capability_mismatch` 等のサブ型は仕様 3-6 §1.1.1 の必須キーをカバーするフィールド（`pipeline.id`, `capability.expected_stage` 等）を保持する。
- `Custom` にはイベント種別（`snake_case`）と任意 JSON を受け取り、プラグイン拡張が `AuditPolicy.include_patterns` で制御できるようにする。
- 実装時は JSON Lines への書き出しで `metadata["event.kind"]` / `metadata["event.id"]` を自動生成し、診断側 `audit.metadata` とキー体系を共通化する。

#### AuditEnvelope 移行ステップ案

1. **型定義の導入**: `audit_envelope.ml` に上記 `t` / `event` / サブ型を追加し、既存 `event` レコード利用箇所を段階的に `Audit_envelope.Event` API へ置換。
2. **コンストラクタ移行**: `main.ml`, `type_error.ml`, `core_ir/iterator_audit.ml` で `Audit_envelope.make` の引数を新型に合わせて更新し、`metadata` に `Assoc` ではなく `metadata` リストを渡すよう統一。
3. **JSON エンコード統合**: `Audit_envelope.to_json` を利用する書き込みパス（`append_events` 等）を更新し、`metadata` の必須キー検証を `tooling/runtime/audit-schema.json` と同期。CI でスキーマ v1.1 を読み込み、`schema.version` との差分を検出するテストを追加。
4. **バージョン通知**: CLI 生成時に `audit_schema_version` を `Cli.Diagnostic_envelope` に埋め込み、`tooling/runtime/audit-schema.json` 更新時は `CHANGELOG` と `0-3-audit-and-metrics.md` にリンクを残す。

**進捗状況 (2025-10-21 更新)**
- 新しい `Audit_envelope.t` を `compiler/ocaml/src/audit_envelope.ml` に導入し、`audit_id` / `change_set` / `capability` を保持可能な構造へ再定義済み。`metadata_pairs` API でリスト渡しに対応。
- `main.ml`／`test_effect_residual.ml`／FFI 関連テストで `~metadata_pairs` を使用するよう更新し、`Ffi_contract.bridge_audit_metadata_pairs` を追加済み。
- `dune build`／`dune runtest` で回帰なし。

#### AuditEnvelope 再定義とスキーマ検証計画（2025-10-27 草案）

1. **型レベル整備**  
   - `Audit_envelope.t` を仕様記述に合わせて `Uuidm.t option` / `Change_set.t option` / `Capability_id.t option` / `Metadata.t` で構成し、`type event` を §1.1.1 の列挙で網羅。  
   - `Audit_envelope.Event.to_json` を追加し、カテゴリごとの必須キー検証（`bridge.platform`, `effect.stage.required` など）をパターンマッチで行う。  
   - 影響ファイル: `compiler/ocaml/src/audit_envelope.{ml,mli}`, `compiler/ocaml/src/core_ir/iterator_audit.ml`, `compiler/ocaml/src/cli/diagnostic_envelope.ml`

2. **書き込みパイプライン移行**  
  - `tooling/runtime/audit-schema.json` v1.1 をソースオブトゥルースとし、`schema.version` を `Audit_envelope` 生成時に必ずメタデータへ注入。  
  - `main.ml`, `tooling/cli/commands/diagnostics_emit.ml`, `tooling/ci/sync-iterator-audit.sh` などの書き込み点を `Audit_envelope.Event` API 経由に統一。  
  - JSON Lines 生成箇所で `Audit_envelope.Event.to_json` を呼び出すよう変更し、旧 `category` 文字列ベースのコードパスを削除。

3. **CI スキーマ検証**  
   - `scripts/ci/verify-audit-schema.sh`（新規）を追加し、`ajv` 互換チェッカまたは `python -m jsonschema` で `tooling/runtime/audit-schema.json` を検証。  
   - GitHub Actions（Linux/Windows/macOS）の `audit-*` ジョブで、生成された `.audit.jsonl` をスキーマ検証し、違反時に失敗させる。  
  - `tooling/ci/collect-iterator-audit-metrics.py` に `schema_version` フィールドチェックを組み込み、`ffi_bridge.audit_pass_rate` / `iterator.stage.audit_pass_rate` のレポートにバージョンを併記。

4. **移行完了条件**  
   - `rg "Audit_envelope.make" compiler/ocaml/src | grep metadata_pairs` が 0 件になり、全て新 API を利用。  
   - 3 ターゲット（Linux, Windows, macOS）の CI でスキーマ検証が緑クリア、`tooling/ci/artifacts/` に `schema-report.json` が保存される。  
   - `docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` に `schema.version` 更新履歴と CI 実行ログの参照が追記され、`compiler/ocaml/docs/technical-debt.md` の ID 22/23 が「監査パイプライン移行完了」に更新される。

### Stage B タスクボード（2025-10-27 着手）

#### B-1 LSP / CI 向け V2 トランスポート整備
- `tooling/lsp/diagnostic_transport.ml`（新規）で `Diagnostic.t` → LSP `PublishDiagnostics` 変換を実装し、`secondary` / `hints` / `audit` / `timestamp` を直接マッピングする。  
- V1 互換レイヤーは `tooling/lsp/tests/client_compat/` に集約し、`client-v1.ts` の `notes` 依存を `secondary` ベースへ改修。fixtures（`diagnostic-sample.json`, `diagnostic-v2-sample.json`）を V2 構造で再生成し、`npm test` に V2 エクスポート検証を追加。  
- CI では `tooling/ci/collect-iterator-audit-metrics.py` / `tooling/ci/sync-iterator-audit.sh` を更新し、`Diagnostic.audit` と `Diagnostic.timestamp` の必須チェックを導入。`iterator-stage-summary.md` / `ffi-bridge-summary.md` の生成スクリプトへ `schema.version` と V2 サマリを併記する。

#### B-2 ゴールデン更新ワークフローとレビュー手順
- `reports/diagnostic-migration.md` を新設し、各バッチ（型エラー → 効果/型クラス → CLI 補助診断）の差分記録・チェックリスト・検証ログ欄を用意する。  
- `scripts/update-diagnostics-golden.sh` を V2 JSON 出力に対応させ、`tooling/ci/collect-diagnostic-diff.py` と連携して差分要約を PR コメントへ投稿するワークフローを GitHub Actions に追加。  
- ゴールデン変更を含む PR は `reports/diagnostic-migration.md` の該当バッチ節を更新し、`dune runtest`・CI 成果物 URL・レビューポイント（`codes[]` 並び、`secondary` 追加、`audit` 埋め込みなど）を記録する運用を定める。
 - **現在の実装状況**: `scripts/update-diagnostics-golden.sh`（V2 対応版）・`tooling/ci/collect-diagnostic-diff.py` を追加済み。`--diff` オプションで Markdown サマリを生成し、`schema_version` と `timestamp` 欠落を検出する。
  - **2025-10-27 追記**: `type_error.ml` の監査メタデータ組み立てを `Diagnostic.merge_audit_metadata` 経由に刷新し、効果診断・FFI 診断の `Audit_envelope` を一括で登録。CLI `remlc` では実行単位の `audit_id` / `change_set` を生成して診断出力と監査イベントに伝播させ、CI スクリプト（`collect-iterator-audit-metrics.py`、`sync-iterator-audit.sh`）を新フィールド検証に対応させた。

#### B-3 Legacy / Builder API 拡張と段階的削除
- `Diagnostic.Builder` に `set_id` / `add_secondary` / `merge_secondary` / `set_timestamp` など補助関数を追加し、structured hints へ `id` / `title` / `payload` を直接設定できる API を整備する。  
- `Diagnostic.Legacy` の `diagnostic_of_legacy` / `legacy_of_diagnostic` に `[@alert deprecated]` を付与し、新規利用を CI で検出可能にする（削除目標: Phase 2-5 開始時）。  
- `type_error.ml` / `parser_driver.ml` / `core_ir/iterator_audit.ml` などレガシ API が残る箇所を段階的に Builder ベースへ移行し、LSP / CLI 以外の JSON レポート生成スクリプトでも `Diagnostic.t` を直接利用するよう统合を進める。

**残タスク**
1. **完了(2025-10-27)** Type エラー生成箇所（`type_error.ml`）で `Audit_envelope.merge_metadata` を使い、効果・型クラス診断の追加キーを新構造へ統一。
2. **完了(2025-10-27)** CLI 出力で `AuditEnvelope` の `audit_id`／`change_set` を埋める設計を詰め、`tooling/runtime/audit-schema.json` の `schema.version` を明示的に付与する運用を確立。
3. **完了(2025-10-27)** 監査ログ生成スクリプト（`tooling/ci/sync-iterator-audit.sh` 等）を新 JSON フィールドに対応させ、ゴールデン更新と差分ツール調整を実施。

#### メタデータ拡張要件まとめ

- **型クラス (`extensions.typeclass.*`)**
  - 必須キー: `typeclass.constraint`, `typeclass.resolution_state`, `typeclass.candidates[]`, `typeclass.dictionary`, `typeclass.pending[]`, `typeclass.generalized_typevars[]`, `typeclass.graph.export_dot`。
  - 監査連携: `AuditEnvelope.metadata["typeclass"]` に `TypeclassExtension` を JSON 化して格納し、辞書導出ログ（2-1-typeclass-strategy.md §5）と同期。
  - CI 影響: 将来的な `typeclass.audit_pass_rate` 指標追加を想定し、欠落キー検出ルールを `tooling/ci/collect-iterator-audit-metrics.py` に準備。

- **効果システム (`extensions.effect.*`)**
  - 必須キー: `effect.stage.required`, `effect.stage.actual`, `effect.stage.source`, `effect.stage.residual`, `effect.stage_trace[]`, `effect.attribute[]`, `effect.capability`。
  - 監査連携: `AuditEnvelope.metadata.stage_trace` と配列を共有し、`iterator.stage.audit_pass_rate` の閾値判定に利用。`effects.contract.*` 診断と `RuntimeCapabilityResolver` の結果が一致しているか CI で検証。
  - 追加検討: Stage 残余 (`effect.residual.*`) の経路情報を `extensions.effect.residual.trace` に記録し、メモリリーク調査や Phase 3 の async audit と連携させる。

- **FFI (`extensions.bridge.*`)**
  - 必須キー: `bridge.status`, `bridge.target`, `bridge.arch`, `bridge.platform`, `bridge.abi`, `bridge.expected_abi`, `bridge.ownership`, `bridge.extern_symbol`, `bridge.return.ownership`, `bridge.return.status`, `bridge.return.wrap`, `bridge.return.release_handler`, `bridge.return.rc_adjustment`, `bridge.source_span`。
  - 監査連携: `AuditEnvelope.metadata["bridge"]` を同一キーで保持し、`ffi_bridge.audit_pass_rate` のゲート条件と一致させる。Windows/macOS CI で欠落時にフェイルさせるジョブを Phase 2 中盤までに整備。
  - ドキュメント: 更新内容を `reports/ffi-bridge-summary.md` と `docs/spec/3-9-core-async-ffi-unsafe.md` §2.6 に反映し、監査ログスキーマ v1.1 の必須項目表と同期する。

1.3. **既存コードのマイグレーション**
- 影響範囲棚卸し: `compiler/ocaml/src/parser_driver.ml` / `type_checker` / `cli` / `tooling/ci` など診断生成サイトを列挙し、旧 API (`Diagnostic.make`, `Diagnostic.of_*`) を新 `builder` API に差し替える順序計画を作成。棚卸し表は `docs/plans/bootstrap-roadmap/2-4-diagnostics-audit-pipeline.md` 付録として公開。
- 段階的移行: 第一段階で新レコードを導入し旧フィールドから移行用変換関数を提供、第二段階で CLI フォーマッタを新フィールドに切替、第三段階で旧型を廃止するタイムラインを策定。互換期間中は変換ロジックを `Deprecated` モジュールに隔離し、Phase 2 内で削除できるよう TODO を付与。
- テスト更新: `compiler/ocaml/tests/golden/diagnostics/*.json.golden`、`tooling/ci/ffi-audit/*.jsonl`、`_build/default/src/main.exe` のスナップショット出力を新構造に合わせて更新。差分確認用に比較スクリプトを追加し、`dune runtest` の失敗理由がフィールド差分である場合に明示されるよう改善。
- ドキュメント同期: `docs/spec/3-6`・`docs/guides/ai-integration.md`・`docs/plans/bootstrap-roadmap/2-3-to-2-4-handover.md` の関連節を更新し、再設計後のフィールド仕様と運用手順（CI ゲート条件・監査ログ参照方法）を反映する。

1.4. **`Diagnostic.Builder` 補助関数拡充ロードマップ**
- API 設計: `builder` に対し複数コード（`push_code`, `set_primary_code`）と構造化ヒント（`add_structured_hint ~kind ~payload`）を扱う補助関数群を追加し、`docs/spec/3-6-core-diagnostics-audit.md` §2.3 の命名規約と整合させる。補助関数の導入案を `compiler/ocaml/src/diagnostic_builder.mli` のドラフトとして 27 週目までに共有。
- 適用優先順位: `type_error.ml` を出発点に、効果系（`effects/effect_error.ml`）、型クラス（`typeclass/diagnostic.ml`）、CLI サブコマンド（`tooling/cli/commands/*`）の順で `builder` API に移行。各モジュールに TODO コメントを残し、移行完了条件（複数コードと structured hints の両対応）を明示。
- テスト戦略: 既存の `dune runtest --force` 実行で拾えるスナップショットに加え、`compiler/ocaml/tests/unit/diagnostic_builder_tests.ml`（新規）で補助関数の組み合わせを検証。`--promote` を禁止した CI でも差分検出できるよう、補助関数単体の JSON シリアライズ期待値を用意。
- 互換運用: 旧来の `Diagnostic.make_*` は Phase 2 中は非推奨扱いとして残置し、`Deprecated` 名前空間で `builder` 呼び出しへ委譲。移行状況を `docs/plans/bootstrap-roadmap/2-4-status.md`（週次ログに追加予定）で追跡し、削除タイムラインを記録。

1.5. **ゴールデンファイル刷新と差分レビュー手順**
- ベースライン取得: `scripts/update-diagnostics-golden.sh`（既存スクリプトを拡張）で V1→V2 変換後の JSON を一括生成し、`tmp/diagnostics-v2-baseline/` に保存。生成タイムスタンプと `git describe` を `reports/diagnostic-migration.md`（新規レポート）へ記載する。
- レビュー手順: `compiler/ocaml/tests/golden/diagnostics/*.json.golden` の更新時は、`tools/diagnostics-diff.py --before <old> --after <new>`（差分抽出ツールを本フェーズで整備）を用いてフィールド追加/削除を分類。レビューでは `codes[]`・`structured_hints[]` の並び替えが意図通りかチェックリスト化し、PR テンプレートに貼り付ける。
- 段階導入: ① `type_error` 系 ② 効果/型クラス ③ CLI 系補助診断 の 3 バッチに分割してゴールデン置換を実施。各バッチ完了後に `docs/plans/bootstrap-roadmap/2-4-diagnostics-audit-pipeline.md` 本文へ進捗記録を追記し、対応コミットを `docs/migrations/diagnostic-golden.log`（新設）で追跡。
- CI 連携: `tooling/ci/collect-diagnostic-diff.py` を追加し、GitHub Actions で `--compare-golden` を実行して差分サマリ（追加/削除フィールド件数）をアーティファクト化。差分が許容範囲内かを自動判定し、閾値超過時はレビュー前に失敗させる。

**成果物**: 拡張 Diagnostic 型ドラフト、AuditEnvelope 仕様整合法、移行ステップ表・テスト更新計画

### 2. シリアライズ統合（27週目）
**担当領域**: 出力フォーマット

Reml の診断/監査情報を CLI・LSP・CI の各チャネルで同一仕様として扱うため、Phase 2 ではシリアライズ層の再設計を 27 週目のマイルストーンとして統合的に進める。`docs/spec/3-6-core-diagnostics-audit.md` で定義された必須フィールドと Phase 2-3 で拡張した `AuditEnvelope.metadata` のキーセットを前提に、以下の作業を完了させる。

2.1. **共通シリアライズレイヤ設計**
- `compiler/ocaml/src/diagnostic_serialization.ml(.mli)`（新規）で `Diagnostic.t` / `AuditEnvelope.t` から中間表現 `SerializedDiagnostic` を生成するユーティリティを定義し、CLI・LSP・CI で共有する。中間表現は JSON 向けフィールド名を正規化し、`extensions`/`metadata` のキー衝突を検出するバリデータを同梱する。
- フォーマット切替を `compiler/ocaml/src/cli/options.ml` の `--format` フラグに集約し、`cli/json_formatter.ml`・`cli/diagnostic_formatter.ml`・`tooling/lsp/diagnostic_transport.ml` から共通レイヤを呼び出す構成へリファクタリングする。既存利用箇所（`main.ml`, `tooling/ci/collect-iterator-audit-metrics.py`）の影響範囲を棚卸し、移行スケジュールを週次ログ（[`2-4-status.md`](2-4-status.md)）に記録する。
- 拡張ポイントは `Diagnostic_serializer.register`（仮称）として公開し、プラグインが独自トランスポートを追加できるようにする。`docs/spec/3-6` の `extensions.*` 命名規約と `docs/guides/runtime-bridges.md` の監査拡張ポリシーを参照し、追加フィールドが UTF-8 エンコーディングを維持することを lint で確認する。
- **完了条件**: すべてのフォーマッタが共通レイヤ経由で動作し、`dune runtest` の既存スナップショットが `SerializedDiagnostic` 由来の JSON/Text 表現へ更新される。移行後の API 仕様を `compiler/ocaml/docs/technical-debt.md` に追記し、旧 API の削除予定を明記する。

2.2. **JSON 出力の実装**
- `cli/json_formatter.ml` を共通レイヤ対応に刷新し、`tooling/json-schema/diagnostic-v2.schema.json`（スキーマ v2）と `tooling/runtime/audit-schema.json` v1.1 を同時検証するシリアライザを実装。`AuditEnvelope.metadata` の `bridge.*` / `effect.*` / `typeclass.*` を JSON Schema に従って整形し、欠落キーは `Result.Error` で検出する。
- `scripts/validate-diagnostic-json.sh`（新規）を追加し、`dune runtest` 後に生成される JSON を JSON Schema で検証する。CI では Linux/Windows/macOS 全てでスキーマ検証ジョブを追加し、`ffi_bridge.audit_pass_rate` と同じ閾値ファイルに JSON 検証結果を記録する。
- `--format json` は `--json-mode={pretty,compact,lines}` の派生フラグを受け付け、Phase 1 互換（pretty）、CI 向け（compact）、ストリームログ（lines）の 3 モードを提供。モード切替仕様を `docs/guides/ai-integration.md` の診断取得セクションへ追記し、CLI ヘルプと README（`docs/spec/0-0-overview.md`）にも反映する。
- **完了条件**: `compiler/ocaml/tests/golden/diagnostics/*.json.golden`・`compiler/ocaml/tests/golden/audit/*.jsonl.golden` が新シリアライザで更新され、`npm run test`（`tooling/lsp/tests`）および `tooling/ci` ディレクトリで追加する JSON バリデーションスクリプトがスキーマ検証を通過する。

2.3. **テキスト出力の実装**
- `cli/diagnostic_formatter.ml` を `SerializedDiagnostic` ベースに改修し、`cli/color.mli` の ANSI 強調表示と `docs/spec/1-1-syntax.md` の Unicode 表記規約を満たすカラーハイライトを再構成する。Grapheme クラスタ単位でスライスできるよう `Core.Text` 由来のユーティリティ（`unicode_segment.ml` を新設）を導入する。
- ソースコードスニペット抽出は `parser_driver.ml` の既存ロジックを `compiler/ocaml/src/cli/snippet_provider.ml`（新規）に切り出し、`Result` で失敗時のフォールバックを明示。CLI では `--format text --no-snippet` オプションを追加し、CI ログの簡略化ニーズに応える。
- Phase 1 の診断フォーマットとの互換性検証として、`reports/diagnostic-format-regression.md`（新設）に差分サマリを保存し、重大なメッセージ変更は Phase 2-0 指針の「分かりやすいエラーメッセージ」基準に照らして承認プロセスを記録する。
- **完了条件**: `dune runtest` のテキストスナップショットが更新され、`docs/spec/3-6` に記載された例示出力が新フォーマットへ差し替えられる。CLI で `--format text` を指定した場合も `ffi_bridge.audit_pass_rate` 集計が従来通り行えることを `tooling/ci/collect-iterator-audit-metrics.py` のテストで確認する。

2.4. **LSP トランスポート V2 フィールド公開と互換性検証**
- 既存の `tooling/lsp/diagnostic_transport.ml` を V2 対応へ拡張し、`SerializedDiagnostic` から LSP エンコード用構造体へ写像する関数を `tooling/lsp/lsp_transport.mli`（新設）に定義。同時に V1 互換レイヤを `tooling/lsp/compat/diagnostic_v1.ml`（新設）へ分離し、`--format lsp-v1` / `--format lsp-v2` の明示的制御を LSP サーバー起動スクリプト（`tooling/lsp/README.md` 掲載の `npm start` シナリオ）に反映する。
- LSP 仕様（3.17 以降）と `docs/spec/3-6` の新規フィールド（`codes[]`, `structured_hints[]`, `extensions`）を照合し、`codeDescription`・`relatedInformation` へのマッピング表を本計画書付録へ掲載。`structured_hints` の `command`/`data` 変換は `tooling/lsp/jsonrpc_server.ml`（新設）で `Result` を返すようにし、エラーは監査ログ `extensions.lsp.compat_error` に落とす。
- 互換性テストは既存の `tooling/lsp/tests/client_compat/` に追加ケースを投入し（`client-v1.ts`, `client-v2.ts` が FFI/効果診断を取り込む想定）、`tooling/lsp/tests/fixtures/*.json` を更新して CLI 生成 JSON との差異を検知する。GitHub Actions には `lsp-contract` ジョブを追加し、V1/V2 双方の JSON を `tooling/json-schema/diagnostic-v2.schema.json` と照合する。
- ドキュメントは `docs/spec/2-0-parser-api-overview.md` の LSP 節、および `docs/guides/ai-integration.md` の API 連携節に V2 フィールド導入と移行手順を追記。LSP クライアントによる受信確認手順は `docs/guides/plugin-authoring.md` へ簡易チュートリアルとして掲載する。
- **完了条件**: LSP サーバーを経由した CLI 実行で V1/V2 が切り替わり、`tooling/lsp/tests`・`npm test`・GitHub Actions `lsp-contract` がすべて成功する。`compiler/ocaml/docs/technical-debt.md` では「LSP V2 対応」を完了扱いとして更新し、関連 TODO をクローズする。

**成果物**: シリアライズレイヤ、JSON/テキスト出力、LSP 互換性検証、スキーマ検証パイプライン

### 3. 監査ログ永続化（27-28週目）
**担当領域**: ログ管理

3.1. **CLI インターフェース拡張**
- Phase 2-3 時点で暫定導入済みの `--emit-audit` を恒久フラグとして位置付け、`compiler/ocaml/src/cli/options.ml`・`main.ml` で `AuditEmitter.{format,level}` の既定値を `docs/spec/3-6-core-diagnostics-audit.md` §2.2 のキー集合に合わせて整理する。既存呼び出し (`tmp/cli-callconv-out/<platform>/`) を壊さないため、従来の `tmp/` 出力は `--audit-store=tmp` 指定時のみ生成する後方互換レイヤとして残す。
- 永続化用に `--audit-store=<profile>` と `--audit-dir=<path>` を追加し、`profile = ci` の場合は `reports/audit/<target>/<YYYY>/<MM>/<DD>/`、`profile = local` の場合は `tooling/audit-store/local/<timestamp>/` に書き出す。パス解決は `compiler/ocaml/src/cli/audit_path_resolver.ml`（新設）へ切り出し、テストでは `TMPDIR` を上書きできるよう抽象化する。
- 詳細度は `--audit-level={summary,full,debug}` に統一し、`summary` では `AuditEnvelope.metadata` の必須キーのみ、`full` で Phase 2-3 までのメタデータ、`debug` で `extensions.*` をすべて含む。CLI ヘルプと `docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` §0.3.1 の収集手順を同期し、古い `--audit-output`・`--audit-format` の書式は非推奨として警告ログを出す。
- **実装イテレーション案**  
  - *Iteration A（Week 27 前半）*: `options.ml` と `main.ml` でフラグ定義と既存 `--emit-audit` の既定値変更、`tmp/` 互換モード実装、単体テスト整備。  
  - *Iteration B（Week 27 後半）*: `audit_path_resolver.ml`・`persistence.ml` の新設、`--audit-store` / `--audit-dir` の分岐実装、ローカルストア (`tooling/audit-store/local/`) の E2E テスト追加。  
  - *Iteration C（Week 28 前半）*: `--audit-level` の導入、JSON Lines 書き出しのフィルタリングロジック実装、`collect-iterator-audit-metrics.py` との統合テスト。  
  - *Iteration D（Week 28 後半）*: CI プロファイル (`reports/audit/`) の命名規約・インデックス更新・圧縮履歴生成、`audit-retention.toml` を読み込む CLI API の追加、ドキュメント更新とヘルプテキスト反映。

3.2. **永続ストアと命名規約**
- 恒久保存先はリポジトリ再編計画 (`docs/plans/repository-restructure-plan.md` §5.1) に従い `reports/audit/` 配下へ集約する。`AuditEnvelope.build_id`（UTC タイムスタンプ + Git commit）をキーに `reports/audit/<commit>_<target>_<build-id>.jsonl` を生成し、`tooling/ci/collect-iterator-audit-metrics.py` がインデックス化できる CSV/JSON インデックス（`reports/audit/index.json`）を併せて更新する。
- `tooling/ci/sync-iterator-audit.sh` と連動し、CI 成功時は最新 20 ビルド分を `reports/audit/history/<target>.jsonl.gz` として圧縮保存、失敗時は `reports/audit/failed/<commit>/` へフルログを退避する。アーティファクトの TTL は GitHub Actions の保持期間（既定 90 日）を前提にしつつ、ローカル永続ストアでは `~/.cache/reml/audit/` へシンボリックリンクを張って参照できるよう README に追記する。
- 永続化処理は `compiler/ocaml/src/audit/persistence.ml`（新設）にまとめ、JSON Lines 書き込み・gzip 圧縮・インデックス更新を `Result` で返却する。スキーマ破壊を防ぐため `tooling/json-schema/audit-envelope.schema.json` にバージョン 1.1 対応の定義を追加し、書き込み前にローカル検証を実施する。

3.3. **ローテーションと容量管理**
- `reports/audit/index.json` に `retained_entries` フィールドを追加し、ターゲットごとの保持件数・バイト数を記録する。`collect-iterator-audit-metrics.py --prune` で最大件数（CI: 100、ローカル: 30）を超えた古いログを自動削除し、削除されたログ ID を `docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` のレビュー記録テンプレートに追記する運用とする。
- ローテーション設定は `tooling/ci/audit-retention.toml`（新設）でターゲット別に管理し、Phase 3 のマルチターゲット拡張を想定して `linux-amd64` / `windows-msvc` / `macos-arm64` の 3 セクションを初期定義する。CI ではこの設定ファイルを読み込んで pruning を実行し、ローカルでは `--retain <n>` オプションで一時的に保持件数を上書きできるよう CLI 側で指定可能にする。
- 監査ログの容量を定期集計するため、`reports/audit/usage.csv` に日別容量を追記し、`docs/notes/core-library-outline.md` や `docs/spec/3-6-core-diagnostics-audit.md` の参考値にフィードバックする。容量が 500 MB を超える場合は `0-4-risk-handling.md` へ記録し、圧縮アルゴリズム変更や S3 連携などのフォローアップを検討する。

3.4. **レビューフローとアクセス経路**
- `reports/diagnostic-format-regression.md` に監査ログ永続化のレビューチェックリストを追加し、PR で `reports/audit/index.json` が更新された場合に確認すべき項目（保持件数、必須フィールド、スキーマバージョン）を明文化する。
- `docs/spec/3-6-core-diagnostics-audit.md` 付録へ永続ストアの命名規約と CLI フラグ一覧を追記し、`AuditPolicy.exclude_patterns` を更新して永続化不要なテレメトリを除外できるようにする。差分が発生した場合は `docs/plans/bootstrap-roadmap/2-3-to-2-4-handover.md` のフォローアップ節に記録してレビュー共有する。
- チームレビュー用に `tooling/ci/collect-iterator-audit-metrics.py --summary reports/audit/index.json` で生成する Markdown サマリ (`reports/audit/summary.md`) を定義し、週次レビューで `ffi_bridge.audit_pass_rate` と `iterator.stage.audit_pass_rate` の推移を確認する運用を Phase 2-4 全期間で維持する。

**成果物**: 監査ログ永続化、CLI フラグ、ログ管理

**進捗状況 (2025-10-29 更新)**
- `Cli.Audit_persistence` に永続ストア初期化と付随ファイル更新を実装し、`summary.md`・`index.json` が `audit_store` ごとに自動更新されるよう調整。型/FFI/Iterator の失敗時に `~outcome:Failure` が確実に記録されるよう `main.ml` のエラーハンドラを統合。
- CI プロファイルでは最新 20 件を gzip 化した `reports/audit/history/<target>.jsonl.gz` を生成し、`~outcome:Failure` 指定時に `reports/audit/failed/<build-id>/` へ監査ログと `entry.json` を退避。履歴生成で必要だった `Gzip.output_string` 呼び出しは camlzip 1.13 対応のラッパー (`gzip_output_string`) に置き換えてビルドを回復。
- `reports/audit/index.json` に `retained_entries` フィールドを追加し、プロファイル × ターゲット単位の保持件数と推定サイズ（合計バイト数）を自動集計できるようにした。`collect-iterator-audit-metrics.py --prune` からの集計で件数/容量を確認できる基盤が整備済み。
- 監査ログ付き診断の JSON スナップショット（残余効果リーク／Iterator Stage ミスマッチ／FFI ABI 不一致）を更新し、`domain`・`audit_metadata`・`codes` の整合性を Phase 2 仕様の表現に合わせて検証 (`dune runtest` 済)。
- README と `docs/guides/cli-workflow.md` に `reports/audit/` 配下の生成物仕様と `camlzip` 依存を追記し、開発者と CI が `opam install . --deps-only --with-test` を通じて環境を同期できるよう文書化。

**次のステップ候補**
1. `tooling/ci/collect-iterator-audit-metrics.py --prune` を CI ワークフローに組み込み、`audit-retention.toml` の初期値と連動した自動 pruning / `retained_entries` 更新を毎ビルドで行う（dry-run + 成功時書き込みのフローを整理）。
2. CI 成功時の `reports/audit/summary.md` 自動生成と `docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` のレビューチェックリスト更新。`reports/diagnostic-format-regression.md` への差分チェックリスト追記も同タイミングで対応。
3. `tooling/ci/sync-iterator-audit.sh` と Windows/macOS ワークフローに `camlzip` 依存導入を反映し、`reports/audit/history/*.jsonl.gz`・`failed/<build-id>/` がアーティファクトとして保持されることを確認。必要に応じて `docs/spec/3-6-core-diagnostics-audit.md` 付録へサンプルパスを追記。
### 4. メタデータ統合（28-29週目）
**担当領域**: 拡張メタデータ

**全体方針**
- `Diagnostic.extensions`（V2）と `AuditEnvelope.metadata` のキー体系を統合し、`tooling/runtime/audit-schema.json` v1.1 の必須キーと可変フィールドを横断的に管理する。キー定義表は `docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` に追記し、更新時は `schema.version` を同時に昇格させる。
- `docs/spec/3-6-core-diagnostics-audit.md` §1.1.1/§2.4 の必須セット（`bridge.*`, `effects.*`, `typeclass.*`, `cli.*` 等）を再確認し、Phase 2-1（型クラス）、2-2（効果）、2-3（FFI）で導入したメタデータを欠落なく集約する。差分が生じた場合は該当タスクのテーブルへフィードバックして整合性を取る。
- `tooling/ci/collect-iterator-audit-metrics.py` と `reports/ffi-bridge-summary.md` のメタデータ依存を棚卸しし、Linux/Windows/macOS いずれの CI でも `schema_version` と拡張キーが一致することを検証対象に追加する。
- 監査永続化（`Cli.Audit_persistence`）が生成する `reports/audit/index.json` / `history/*.jsonl.gz` については `tooling/ci/verify-audit-metadata.py` で拡張キーの欠落チェックと `retained_entries` の再計算を実施する。CI では `verify-audit-metadata.py --index reports/audit/index.json --root . --history-dir reports/audit/history` をゲート条件に追加し、欠落キーと要約不整合を即時検出する。

4.1. **型クラスメタデータ**
- **情報設計**: `docs/spec/1-2-types-Inference.md` §D および `docs/spec/3-6-core-diagnostics-audit.md` §3.2 を反映し、`extensions.typeclass` を以下の構造へ拡張する。`resolution_state`, `dictionary`, `candidates[]`, `graph.export_dot` の有無を表形式で整理し、辞書渡し／モノモルフィゼーション双方の経路を記述する。
- **実装**: `compiler/ocaml/src/typeclass_resolution.ml`, `diagnostic.ml`, `audit_envelope.ml` を対象に `Diagnostic.merge_typeclass_extension`（仮）と `Audit_envelope.add_metadata` を共通呼び出しに統一。辞書エンコード部では `Typeclass.Diagnostic_payload` を JSON 化し、`AuditEnvelope.metadata["typeclass.dictionary"]` へ転写する。
- **検証**: `compiler/ocaml/tests/golden/diagnostics/typeclass-*` と `reports/diagnostic-migration.md` の該当節を更新し、`dune runtest` で `extensions.typeclass.*` キーが全サンプルに含まれることを確認。CI では `collect-iterator-audit-metrics.py --section typeclass` で `schema_version` とキー数を検証する。

4.2. **効果システムメタデータ**
- **情報設計**: `docs/spec/1-3-effects-safety.md` §I と `docs/spec/3-6-core-diagnostics-audit.md` §3.4 をもとに、Stage 判定と残余効果を `extensions.effect` および `AuditEnvelope.metadata["effect.*"]` へマッピングする。`required_stage`, `actual_stage`, `residual`, `handler_stack`, `unhandled_operations` を最小セットとし、Capability Registry から得た `capability_descriptor` を併記する。
- **実装**: `compiler/ocaml/src/effect_checker.ml`, `diagnostic.ml`, `core_ir/iterator_audit.ml` に `Effect_metadata.Builder` を導入し、Stage mismatch（`effects.contract.stage_mismatch`）発生時に `Audit_envelope.Event.Capability_mismatch` と同期したメタデータを書き出す。`tooling/runtime/audit-schema.json` の `effect.stage.*` 必須マークを更新する。
- **検証**: `tests/golden/diagnostics/effects-*` と `reports/audit/history/effects/*.jsonl.gz` のサンプルを再生成し、`effects.contract.stage_mismatch` / `effects.contract.reordered` が要求フィールドを保持することを `jsonschema` チェックに追加。CI では Windows プロファイル（技術的負債 ID 22）を含めて Stage 検証ログを比較する。

4.3. **FFI メタデータ**
- **情報設計**: `docs/spec/3-9-core-async-ffi-unsafe.md` と `reports/ffi-bridge-summary.md` の項目を突き合わせ、`extensions.bridge` に `platform`, `abi`, `ownership`, `callconv`, `capability.stage` を含むキーセットを定義。`AuditEnvelope.metadata` には `bridge.platform`, `bridge.status`, `bridge.retry_count`, `bridge.audit_pass_rate` を揃える。
- **実装**: `compiler/ocaml/src/ffi_contract.ml`, `diagnostic.ml`, `cli/audit_persistence.ml` を対象にメタデータ構築関数を一本化し、Windows/macOS の CI で生成する監査ログへ同一キーを反映する。`tooling/ci/sync-iterator-audit.sh` は `extensions.bridge` の欠落チェックを追加し、技術的負債 ID 22/23 の完了条件と連動させる。
- **検証**: GitHub Actions の Linux/Windows/macOS 各ジョブで `ffi_bridge.audit_pass_rate` と `iterator.stage.audit_pass_rate` を監査し、`reports/ffi-bridge-summary.md` に `schema_version` とキー充足率を自動追記する。`compiler/ocaml/tests/golden/audit/ffi-*` を更新し、`jsonschema` 検証のスコープへ追加する。

4.4. **整合性レビューとドキュメント**
- **レビュー**: メタデータキー追加時は `docs/spec/0-3-code-style-guide.md` の命名規則と `docs/spec/0-2-glossary.md` の用語統一を確認する。差分レビュー用に `reports/audit/summary.md` と `reports/diagnostic-migration.md` の該当節へチェックリストを追記。
- **周知**: 新規キーとバージョン更新を `README.md`（監査項目索引）および `docs/guides/ai-integration.md`（AI 連携メタデータ）に反映し、Phase 3 で LSP/AI プラグインが参照するフィールドを確定させる。
- **フォローアップ**: `compiler/ocaml/docs/technical-debt.md` の ID 22/23 を更新し、キー統合が完了したタイミングで「監査メタデータ統一済み」と記録する。必要に応じて `docs/notes/dsl-plugin-roadmap.md` に Capability 渡しの監査観点を追加する。

**成果物**: 統合メタデータ定義表、キー命名規約、CI 検証タスク、関連ドキュメント更新

### 5. レビュー支援ツール（29週目）
**担当領域**: ツール開発

5.1. **監査ログ差分ツール**
- 2つの監査ログの差分抽出
- 診断の追加/削除/変更の検出
- マークダウン/HTML レポート生成
- CI での自動実行

5.2. **統計ダッシュボード**
- 監査ログからの統計抽出
- エラー/警告の推移グラフ
- ビルド時間の推移
- 視覚化（グラフ生成）

5.3. **クエリツール**
- 監査ログの検索・フィルタリング
- 診断コードでの絞り込み
- メタデータでのクエリ
- jq 風の DSL 検討

**成果物**: 差分ツール、ダッシュボード、クエリツール

### 6. CI/CD 統合（29-30週目）
**担当領域**: 自動化

6.1. **CI での監査ログ生成**
- GitHub Actions での `--emit-audit` 実行
- 監査ログのアーティファクト保存
- PR ごとの監査ログ差分レポート
- コメント自動投稿（新規エラー/警告）
- Linux / Windows / macOS 各ワークフローで `iterator.stage.audit_pass_rate` と `ffi_bridge.audit_pass_rate` をゲート条件として導入（技術的負債 ID 22 の解消）

6.2. **スキーマ検証**
- JSON スキーマでの検証自動化
- スキーマ違反の検出とエラー報告
- スキーマバージョンの管理
- Phase 1/2 の CI との統合

6.3. **レグレッション検出**
- 診断の予期しない増加の検出
- ビルド時間の劣化検出
- 閾値設定と通知
- `0-3-audit-and-metrics.md` との連携

**成果物**: CI 統合、スキーマ検証、レグレッション検出

### 7. ドキュメント更新（30週目）
**担当領域**: 仕様整合

7.1. **仕様書フィードバック**
- [3-6-core-diagnostics-audit.md](../../spec/3-6-core-diagnostics-audit.md) への実装差分の反映
- メタデータキー一覧の追加
- 診断フォーマットの例示
- 新規サンプルコードの追加

7.2. **ガイド更新**
- `docs/guides/ai-integration.md` の監査ログ連携を追記
- ツール使用例の追加
- トラブルシューティング情報
- ベストプラクティスの文書化

7.3. **メトリクス記録**
- `0-3-audit-and-metrics.md` に診断システムの性能記録
- スキーマバージョンの履歴
- CI レポートの自動生成設定
- 監査ポリシーの更新履歴

**成果物**: 更新仕様書、ガイド、メトリクス

### 8. 統合テストと安定化（30-31週目）
**担当領域**: 品質保証

8.1. **スナップショットテスト**
- 診断出力のゴールデンテスト
- 監査ログのゴールデンテスト
- スキーマ検証テスト
- Phase 1/2 の全テストでの監査ログ生成

8.2. **統合テスト**
- 型クラス + 効果 + FFI の診断統合テスト
- メタデータの一貫性検証
- 差分ツールの動作テスト
- ダッシュボードの生成テスト

8.3. **安定化とバグ修正**
- テスト失敗の原因調査と修正
- エッジケースの追加テスト
- 既知の制限事項の文書化
- Phase 3 への引き継ぎ準備（macOS FFI サンプル自動検証の進捗を技術的負債 ID 23 と照合）

**成果物**: スナップショットテスト、統合テスト、安定版

## 成果物と検証
- 診断/監査ログが全テストケースで期待フォーマットになることをスナップショットテストで確認。
- CLI で `--emit-audit` を指定した際に JSON が出力され、CI でスキーマ検証が行われる。
- 監査ログ差分ツールを docs に記載し、レビュー手順が共有される。

## リスクとフォローアップ
- フィールド追加によりテストが脆くなる恐れがあるため、スキーマ検証を導入しレグレッションを防止。
- 監査ログの出力量が多くなる場合、サマリ統計と詳細ログの二段構えに切り替える検討を行う。
- AI 支援関連の要件は `docs/guides/ai-integration.md` と調整し、外部公開範囲を明示。

## 参考資料
- [2-0-phase2-stabilization.md](2-0-phase2-stabilization.md)
- [3-6-core-diagnostics-audit.md](../../spec/3-6-core-diagnostics-audit.md)
- [guides/ai-integration.md](../../guides/ai-integration.md)
- [0-3-audit-and-metrics.md](0-3-audit-and-metrics.md)
- [2-3-to-2-4-handover.md](2-3-to-2-4-handover.md)
- [compiler/ocaml/docs/technical-debt.md](../../../compiler/ocaml/docs/technical-debt.md)
