# Reml クロスコンパイル仕様組み込み計画

> 判定基準は `0-2-project-purpose.md` の性能・安全性を最優先しつつ、書きやすさとエコシステム整合性を高優先で考慮する。

## 1. 要素分類サマリ

| 分類 | 反映対象ドキュメント | 主な内容 | 判断根拠 (0-2 指針) |
| --- | --- | --- | --- |
| 言語仕様 | `1-1-syntax.md`, `1-2-types-Inference.md`, `2-6-execution-strategy.md` | `@cfg` のターゲットキー正式化、`RunConfigTarget` と `TargetProfile` の相互運用、クロスビルド診断コードの追加 | 性能 1.1: ターゲット整合で線形処理維持、安全性 1.2: 型安全な条件分岐と ABI ミスマッチ防止 |
| 標準 API | `3-10-core-env.md`, `3-8-core-runtime-capability.md`, `3-6-core-diagnostics-audit.md`, `3-5-core-io-path.md` | `TargetCapability` グループと `has_capability` 拡張、`infer_target_from_env` のフィールド追加、クロスビルド監査イベント、ターゲット別パス/FS 補助 | 安全性 1.2: Capability によるフォールバック保証、エラー品質 2.2: 診断強化 |
| エコシステム | `4-1-package-manager-cli.md`, `4-2-registry-distribution.md`, `4-3-developer-toolchain.md`, `4-0-ecosystem-integration-plan.md` | `reml target list/show/validate` サブコマンド、標準ライブラリ/ランタイムのターゲット別配布、レジストリメタデータ (targets, runtime_revision) | 性能 1.1: 事前ビルド配布でビルド時間短縮、エコシステム統合 3.2: CLI/レジストリ連携 |
| ガイド | `docs/guides/runtime/portability.md`, `docs/guides/tooling/ci-strategy.md`, （新規）`docs/guides/runtime/cross-compilation.md`, `docs/guides/runtime/runtime-bridges.md` | クロスビルド手順、CI マトリクス例、エミュレーション/リモートテスト運用、Runtime/FFI との整合チェックリスト | 書きやすさ 2.1: パターン化による導入容易化、エラー品質 2.2: 運用手順でトリアージ時間短縮 |

### 1.1 詳細項目

- **言語仕様**
  - `@cfg` における `target_profile`/`profile_id`/`capability` キーを正式キーとして追加し、未定義時の `target.profile.missing` 診断を規定。
  - `RunConfig.extensions["target"]` の最小フィールド集合 (os/family/arch/abi/features/profile_id/diagnostics フラグ) をコア仕様に昇格。
  - クロスビルド時のコンパイラ出力メタデータ（ターゲット triple, data layout, runtime_revision）を仕様化し、`2-6-execution-strategy.md` に反映。

- **標準 API**
  - `Core.Env.infer_target_from_env` に `abi`, `vendor`, `capabilities` を追加し、戻り値を `Result<TargetProfile, EnvError>` へ昇格。
  - `Core.Runtime` に `TargetCapability::{UnicodePolicy, FilesystemCase, HasThreadLocal}` 等を定義し、`has_capability` の初期化を `TargetProfile` から自動注入。
  - `Core.Diagnostics` に `DiagnosticDomain::Target` と `target.abi.mismatch` 等のメッセージキーを追加、監査ログには `AuditEvent::TargetProfile` を導入。

- **エコシステム**
  - CLI: `reml target list`, `reml target show <id>`, `reml target scaffold`, `reml target validate`, `reml build --target <id>`, `reml test --target <id>` の仕様項目を追記。
  - Toolchain: `artifact/std/<triple>/<hash>` と `runtime/<profile>` などのディレクトリ構造と署名検証フローを `4-3` に定義。
  - Registry: パッケージメタデータに `targets` 配列、`runtime_revision`、`requires_capabilities` を追加し、互換チェック失敗時のエラーを規定。

- **ガイド**
  - `docs/guides/runtime/portability.md` に TargetProfile のセットアップ手順を追加し、既存チェックリストを更新。
  - `docs/guides/tooling/ci-strategy.md`（執筆予定）にマトリクス例 (`os`, `arch`, `emulator`) とキャッシュ戦略を追加。
  - 新規 `docs/guides/runtime/cross-compilation.md` を作成し、CLI/Toolchain/Registry/CI の統合手順を包括的に解説。
  - `docs/guides/runtime/runtime-bridges.md` にクロスビルド成果物のリンク/エミュレーション連携を追記。

## 2. 仕様更新計画

### フェーズA: 言語仕様と中核データモデルの確立 (優先度: 高)
1. `1-1-syntax.md` に `@cfg` キー拡張と診断仕様を追加。
2. `1-2-types-Inference.md` へ `RunConfigTarget` を参照する型/効果分岐の安全条件を追記。
3. `2-6-execution-strategy.md` にターゲットメタデータ生成フローと LLVM Target 連携を記述。
4. レビュー観点: 性能 1.1 の線形処理維持、エラー品質 2.2 の診断明確化。

### フェーズB: 標準 API 拡張 (優先度: 高)
1. `3-10-core-env.md` の API シグネチャ更新と TargetProfile 変換ロジックを記述。
2. `3-8-core-runtime-capability.md` に `TargetCapability` 群と初期化手順を追加。
3. `3-6-core-diagnostics-audit.md` で `DiagnosticDomain::Target` と関連 AuditEvent を定義。
4. 依存: フェーズAで定義した TargetProfile 項目。

### フェーズC: エコシステム仕様整備 (優先度: 中)
1. `4-1-package-manager-cli.md` へターゲット管理サブコマンドと `build/test --target` の動作を追記。
2. `4-2-registry-distribution.md` にターゲットメタデータと署名検証フローを追加。
3. `4-3-developer-toolchain.md` へ標準ライブラリの事前ビルド構造と取得コマンド (`reml toolchain install`) を定義。
4. 依存: フェーズBの API 拡張結果。

### フェーズD: ガイド更新・新規作成 (優先度: 中)
1. `docs/guides/runtime/portability.md` の更新（TargetProfile 設定と診断活用）。
2. `docs/guides/tooling/ci-strategy.md` にクロスマトリクス/キャッシュ/エミュレーション例を追記（初稿が未整備の場合は同フェーズでドラフト作成）。
3. 新規 `docs/guides/runtime/cross-compilation.md` を執筆し、CLI 操作例・ターゲットプロファイル作成・CI テンプレート・検証手順をまとめる。
4. `docs/guides/runtime/runtime-bridges.md` にクロスリンカ設定と FFI シムの ABI 検証手順を追加。

### フェーズE: 整合性レビューとクロスリファレンス更新 (優先度: 中)
1. すべての文書の相互リンク (`[ターゲットプロファイル](4-1-package-manager-cli.md#target-profile)` など) を整備。
2. `README.md` と `4-0-ecosystem-integration-plan.md` に進捗サマリを追記。
3. クロスコンパイル関連の TODO を `todo-dsl-integration.md` 以外のタスク一覧にも追加し、作業管理を容易にする。
4. 最終レビューで `0-2-project-purpose.md` の指針に照らして性能・安全性評価を実施。

## 3. リスクとフォローアップ

- **ABI ミスマッチリスク**: フェーズA/B で `TargetProfile` と `RuntimeCapability` を明記し、CLI/レジストリ側で検証を必須化する。
- **ドキュメント負荷**: フェーズ分割と優先度設定により、先行して必要な言語・API 仕様を固めた上でガイドを段階的に更新する。
- **CI/エミュレーション依存**: `docs/guides/tooling/ci-strategy.md` と新規ガイドで QEMU/リモート実行の推奨設定を示し、性能 1.1 の基準を守る。

---

この計画書は `docs/notes/backend/cross-compilation-spec-intro.md` の調査内容を起点とし、Reml 仕様全体にクロスコンパイル機能を組み込むための作業順序と担当文書を明確化したものである。各フェーズ完了後には `0-2-project-purpose.md` の価値観チェックリストで検証を行い、性能・安全性・エコシステム統合の目標に沿った改善を継続する。
