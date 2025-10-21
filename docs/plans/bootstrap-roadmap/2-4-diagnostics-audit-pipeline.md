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

**残タスク**
1. Type エラー生成箇所（`type_error.ml`）で `Audit_envelope.merge_metadata` を使い、効果・型クラス診断の追加キーを新構造へ統一。
2. CLI 出力で `AuditEnvelope` の `audit_id`／`change_set` を埋める設計を詰め、`tooling/runtime/audit-schema.json` の `schema.version` を明示的に付与する運用を確立。
3. 監査ログ生成スクリプト（`tooling/ci/sync-iterator-audit.sh` 等）を新 JSON フィールドに対応させ、ゴールデン更新と差分ツール調整を実施。

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

**成果物**: 拡張 Diagnostic 型ドラフト、AuditEnvelope 仕様整合法、移行ステップ表・テスト更新計画

### 2. シリアライズ統合（27週目）
**担当領域**: 出力フォーマット

2.1. **共通シリアライズレイヤ設計**
- JSON/テキスト/構造化ログの共通抽象化
- フォーマット切替の設計（`--format` フラグ）
- カスタムフォーマッタの拡張ポイント
- エンコーディング処理（UTF-8 保証）

2.2. **JSON 出力の実装**
- `Diagnostic` → JSON のシリアライザ
- `AuditEnvelope` → JSON のシリアライザ
- JSON スキーマの定義（JSON Schema 形式）
- Pretty print/Compact のモード切替

2.3. **テキスト出力の実装**
- カラー出力対応（ANSI エスケープ）
- ソースコードスニペットの抽出
- Unicode 対応（Grapheme 単位の表示）
- Phase 1 の診断フォーマットとの統合

**成果物**: シリアライズレイヤ、JSON/テキスト出力、スキーマ

### 3. 監査ログ永続化（27-28週目）
**担当領域**: ログ管理

3.1. **CLI フラグの実装**
- `--emit-audit` フラグの追加
- `--audit-output=<path>` での出力先指定
- `--audit-level=<level>` での詳細度制御
- `--audit-format=<format>` でのフォーマット指定

3.2. **ログ永続化ロジック**
- ビルドごとの監査ログファイル生成
- ファイル名の命名規約（タイムスタンプ付き）
- ログローテーション機能
- ディスク容量管理（古いログの削除）

3.3. **ログ構造の設計**
- ビルドメタデータ（日時、バージョン、ターゲット）
- フェーズごとのログ分離（Parser/Typer/LLVM）
- 診断の重要度レベル（Error/Warning/Info）
- サマリ統計（エラー数、警告数、ビルド時間）

**成果物**: 監査ログ永続化、CLI フラグ、ログ管理

### 4. メタデータ統合（28-29週目）
**担当領域**: 拡張メタデータ

4.1. **型クラスメタデータ**
- `extensions.typeclass.*` キーの定義
- 辞書引数の型情報記録
- 制約解決の詳細ログ
- Phase 2 型クラスタスクとの連携

4.2. **効果システムメタデータ**
- `extensions.effect.*` キーの定義
- Stage 検証結果の記録
- 効果タグの伝播トレース
- Phase 2 効果タスクとの連携

4.3. **FFI メタデータ**
- `extensions.bridge.*` キーの定義
- ABI 種別・所有権注釈の記録
- FFI 呼び出しのトレース
- Phase 2 FFI タスクとの連携（技術的負債 ID 22/23 の解消を含む）

**成果物**: 統合メタデータ、キー命名規約、連携実装

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
