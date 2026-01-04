# ガイド→仕様統合計画（Draft）

## 1. 目的と背景
- `docs/guides/` 配下で仕様レベルの API や契約を定義している文書を公式仕様書（通し番号付き章）へ組み込む。
- 仕様とガイドに分散している情報を統合し、関連章からの参照を簡素化するとともに整合性レビューを容易にする。
- 本計画書は統合完了後に破棄する。

## 2. スコープ
- 対象ガイド: `core-parse-streaming`, `core-unsafe-ptr-api-draft`, `data-model-reference`, `runtime-bridges`, `reml-ffi-handbook`, `DSL-plugin`。
- 影響章: 2 系 (Parser), 3 系 (Core Library), 4 系 (公式プラグイン), README と 0 系概要資料、用語集。
- 対象外: 運用ベストプラクティス中心のガイド（例: `ci-strategy`, `collection-pipeline-guide`）は据え置き。

## 3. 統合対象別計画

### 3.1 Core.Parse.Streaming 拡張 (`docs/guides/compiler/core-parse-streaming.md:1`)
- **現状**: `run_stream`/`resume` など公式 API を定義し、仕様上は `2-6-execution-strategy.md:261` が詳細をガイドへ委譲。
- **収容先**: Parser 章に「2-7 Core.Parse.Streaming (Draft→正式)」節を新設。`2-1-parser-type.md:171` の `extensions["stream"]` 説明と連動させる。
- **主な作業**:
  - API 定義・型 (`StreamOutcome`, `FlowController`) を新節へ移設。✅ 完了（`2-7-core-parse-streaming.md` 作成）
  - RunConfig 統合要件を `2-6-execution-strategy.md` 本文へ昇格し、バッチ実行との整合テーブルを追加。✅ 完了
  - IDE/CLI 事例など運用寄りの記述はガイドへ縮約し、相互リンクを更新。✅ 完了（`docs/guides/compiler/core-parse-streaming.md` を運用ガイド化）
- **検証ポイント**: `Diagnostic` と監査ログ統合（`StreamEvent`）が現行仕様の効果タグ表と矛盾しないか確認。✅ 完了（`2-7 §G`, `3-8-core-runtime-capability.md:283` 更新）

### 3.2 Core.Unsafe.Ptr API (`docs/guides/ffi/core-unsafe-ptr-api-draft.md:1`)
- **現状**: unsafe ポインタ型と操作 API を詳細に列挙。章末 TODO で仕様編入を明示（`docs/guides/ffi/core-unsafe-ptr-api-draft.md:210`）。
- **収容先**: `3-9-core-async-ffi-unsafe.md` 内に `Core.Unsafe.Ptr` 節を追加し、効果タグ・Capability 契約を正式化。
- **主な作業**:
  - 型・API を仕様へ移植し、効果タグ (`effect {memory}`) と Capability 要件を明文化。✅ 完了（`3-9-core-async-ffi-unsafe.md` §3 更新）
  - 監査テンプレートと CI スモークテスト要求を 3-9 のテスト節へ統合。✅ 完了（`3-9` §3.7 へ追加）
  - ガイド側には運用メモと追加 TODO のみ残し、「仕様へ統合済み」脚注を追加。✅ 完了（`docs/guides/ffi/core-unsafe-ptr-api-draft.md` を運用ガイド化）
- **検証ポイント**: `3-9` に既出の `Core.Async` / FFI セクションとの重複排除、用語と命名規則の統一。✅ 完了（`UnsafeError`/`TaggedPtr` を統一）

### 3.3 Nest.Data リファレンス (`docs/guides/ecosystem/data-model-reference.md:1`)
- **現状**: `Schema.build` 応用、QualityReport JSON スキーマ、監査整合テストを定義。
- **収容先**: `3-7-core-config-data.md` に「データ品質・監査」節を追加し、QualityReport スキーマと CLI/API フローを正式化。
- **主な作業**:
  - QualityReport JSON スキーマとテストケースを仕様へ組み込み。✅ 完了（`3-7-core-config-data.md` §4.4）
  - CLI サンプルは仕様ではコマンド概要のみ残し、詳細手順をガイドへ残留。✅ 完了（`3-7-core-config-data.md` §4.6 とガイド側注記）
  - `docs/guides/dsl/constraint-dsl-best-practices.md` や `docs/guides/runtime/runtime-bridges.md` からの参照を、新章節に貼り直し。✅ 完了（引用先を `3-7-core-config-data.md` §4 へ更新）
- **検証ポイント**: 監査ログ項目が `3-6-core-diagnostics-audit.md` の命名規約と揃っているか確認。✅ 完了（`3-7-core-config-data.md` §4.5）

### 3.4 Runtime Bridges (`docs/guides/runtime/runtime-bridges.md:1`)
- **現状**: Runtime Bridge の Stage 運用、`reload` 手順、WASI/コンテナ運用まで仕様レベルの契約を記述。`3-9-core-async-ffi-unsafe.md:12` が参照。
- **収容先**: 3 系に「Runtime Bridge 契約」節を追加し、Stage/Capability 契約を正式化。
- **主な作業**:
  - Stage 管理、Capability 契約、`reload` API を仕様本文に移植。✅ 完了（`3-8-core-runtime-capability.md` §10 追加）
  - 実装例（ゲームホットリロード等）は概要と要件に絞り、ガイドへ抜粋を残す。✅ 完了（ガイド冒頭に仕様参照脚注を追加）
  - WASI/コンテナ手順は portability ガイドと重複しないよう整合。✅ 完了（§10.4 でターゲット互換性とチェックリストを定義）
- **検証ポイント**: `3-8-core-runtime-capability.md` の特権 Capability 記述と整合する Stage 要件か確認。✅ Stage テーブルを更新し `verify_capability_stage` と連携を明文化。

### 3.5 FFI ハンドブック (`docs/guides/ffi/reml-ffi-handbook.md:1`)
- **現状**: ABI/データレイアウト、所有権契約、監査テンプレートなど Core.Ffi の基礎仕様を包含。
- **収容先**: `3-9` に `Core.Ffi` 節を整備し、ABI 表やエラーモデルを正式化。LLVM 詳細は `docs/guides/compiler/llvm-integration-notes.md` へリンク。
- **主な作業**:
  - ✅ ABI テーブル、所有権契約、効果タグ整理を仕様へ移植（`3-9-core-async-ffi-unsafe.md` §2.1–§2.7）。
  - ✅ 監査テンプレートを `3-6` 参照付きで統合し、`CapabilitySecurity` チェックリストを 3-8 に反映。
  - ✅ ガイド側は多言語サンプル・段階的ロードマップに縮約し、仕様への参照を追記。
- **検証ポイント**: ✅ `docs/guides/ffi/core-unsafe-ptr-api-draft.md` と重複するポインタ運用記述を整理（仕様に統合した内容へ誘導）。

### 3.6 DSL プラグイン & Capability (`docs/guides/dsl/DSL-plugin.md:1`)
- **現状**: `ParserPlugin` 構造、署名検証、CLI プロトコルを定義。`docs/notes/dsl/dsl-plugin-roadmap.md` と連携。
- **収容先**: Chapter 5 に「5.7 Core.Parse.Plugin 仕様」節を追加し、CLI フローは付録化。
- **主な作業**:
  - ✅ プラグインメタデータ、`register_plugin`/`register_bundle` API を [5-7-core-parse-plugin.md](../spec/5-7-core-parse-plugin.md) に仕様化し、Bundle 識別子・エラーモデルまで定義。
  - ✅ 署名検証ワークフローを Stage/Capability 監査と接続（`5-7` §3、`3-8-core-runtime-capability.md` §1.2 と連動）。
  - ✅ ガイド側には導入手順・ベストプラクティスを残し、仕様へのリンクを追記（`docs/guides/dsl/DSL-plugin.md`、`docs/notes/dsl/dsl-plugin-roadmap.md`）。
- **検証ポイント**: ✅ `5-0-official-plugins-overview.md` と `README.md`、`0-0-overview.md` のリンク構成を更新済み。

## 4. 横断作業
- ✅ README (`README.md`)、概説 (`0-0-overview.md`, `0-1-project-purpose.md`) に統合ハイライト節を追加し、2-7/3-7/3-8/3-9/5-7 への導線を明示。
- ✅ 用語集 (`0-2-glossary.md`) に DemandHint／FlowController／RuntimeBridge／StageRequirement を追記し、Capability Stage の説明を診断メタデータと連携させた。
- ✅ `3-6-core-diagnostics-audit.md` に Runtime Bridge 診断セクション（§8）を新設し、`3-8-core-runtime-capability.md` の Capability 表へ `RuntimeCapability::ExternalBridge` を追加。
- ✅ `docs/notes/dsl/dsl-plugin-roadmap.md` に統合ステータス節を設け、ガイド→仕様移管の完了と監査チェックリストの参照先を明記。

## 5. 実施順序（提案）
1. Core.Parse.Streaming と Core.Unsafe.Ptr を先行統合（Parser/Unsafe API の基盤整備）。
2. Nest.Data リファレンスを 3-7 に組み込み、データ品質仕様を確定。
3. Runtime Bridges と FFI 契約を 3-8/3-9 に編入し、ランタイム全体の整合性を確認。
4. DSL プラグイン仕様を Chapter 5 へ導入し、関連ガイド・ノートのリンクを更新。
5. 横断作業（README・用語集・参照リンク）を一括で更新し、整合テストを実施。

## 6. レビューとフォローアップ
- 各統合ステップ後に該当章の整合レビューを実施（Parser, Core Library, Plugins の各担当）。
- 影響範囲説明と変更理由を `docs/notes/` 配下に履歴として残し、統合完了後に本計画書を削除。
- 未統合の運用ガイドは更新方針のメモを残し、将来の仕様拡張を追跡。
