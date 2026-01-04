# Phase4: Native Escape Hatches 実装計画

## 背景と決定事項
- `docs/notes/ffi/native-escape-hatches-research.md` で示した通り、`Core.Ffi` だけでは SIMD/低レベル最適化/埋め込み用途にギャップがある。
- `docs/spec/0-1-project-purpose.md` の「実用に耐える性能」「エコシステム統合」達成には、Rust 実装でのネイティブ拡張の足場が必要。
- Phase 4 の実用シナリオ回帰に接続できる最小スコープを定義し、過度に危険な機能（全面的な asm/syscall）を段階導入で扱う。
- Inline ASM / LLVM IR の本格実装は `docs/plans/bootstrap-roadmap/4-2-native-escape-hatches-asm-llvm-implementation-plan.md` に分離し、Phase 4 での前倒し実装へ移行した。

## 目的
1. `Core.Native`（または `Core.Intrinsics`）の仕様・監査・Capability 方針を整理し、Rust 実装に落とし込む。
2. Rust 実装において「intrinsic 連携」「埋め込み API」の最小実装を行い、Phase 4 シナリオへ接続する。
3. インライン ASM / LLVM IR 直書きは **設計 + ガード付きプロトタイプ** までを Phase 4 に含め、正式実装は後続フェーズへ引き継ぐ。

## スコープ
- **含む**: `@intrinsic` 属性の設計/検証、LLVM intrinsic マッピング、`Core.Native` の最小 API、埋め込み API の最小 C ABI、監査ログ/Capability 整合、Phase 4 シナリオ登録。
- **含まない**: 汎用 ASM の正式仕様化、syscall のフルサポート、OS 別 ABI 完全対応、LLVM IR 直書きの本実装。

## 成果物
- `docs/spec/1-1-syntax.md` と `docs/spec/1-3-effects-safety.md` に `effect {native}` と `@intrinsic` の最小仕様が反映される。
- `compiler/backend/llvm` に LLVM intrinsic マッピングが入り、`compiler/frontend` で属性検証と診断が整備される。
- `compiler/runtime` に `Core.Native` の最小 API が追加され、監査キーが `docs/spec/3-6-core-diagnostics-audit.md` と一致する。
- 埋め込み API の最小 C ABI (`libreml` 相当) が Rust 実装側に実装され、簡易サンプルが動作する。
- `docs/plans/bootstrap-roadmap/assets/phase4-scenario-matrix.csv` に Native 系シナリオが追加される。

## 作業ステップ

### フェーズA: 仕様・監査・Capability 整合
1. `docs/spec/1-1-syntax.md` に `@intrinsic` 属性の構文と制約を追記する。
   - 設計決定: `@intrinsic("llvm.sqrt.f64")` 形式のみ許可し、識別子式は不可とする。
   - 記述範囲: 関数宣言への付与条件、引数リテラルの型制約、`@intrinsic` と `@cfg` の併用可否。
   - 診断: 構文違反時に `native.intrinsic.invalid_syntax` を出す前提を明記。
   - 参照更新: `docs/spec/1-0-language-core-overview.md` に簡潔な導入文を追加する。
2. `docs/spec/1-3-effects-safety.md` に `effect {native}` の意味、`unsafe` との関係、`@cfg` 要件を追記する。
   - 役割定義: `effect {native}` は ABI/メモリ境界に触れる操作を含むことを明示。
   - 使い分け: `unsafe` は局所的な危険区画、`effect {native}` はモジュール/関数単位の監査対象と整理。
   - 互換条件: `@intrinsic` 付与時は `effect {native}` 必須、`@cfg` でターゲット限定を推奨。
   - ガイド連動: `docs/guides/runtime/runtime-bridges.md` から相互参照リンクを追加。
3. `docs/spec/3-6-core-diagnostics-audit.md` に `native.intrinsic.*` / `native.embed.*` の監査キーを定義する。
   - キー定義: `native.intrinsic.used` / `native.intrinsic.invalid_type` / `native.intrinsic.signature_mismatch`。
   - 埋め込み用: `native.embed.entrypoint` / `native.embed.abi_mismatch` / `native.embed.unsupported_target`。
   - メタデータ: `AuditEnvelope.metadata` に `intrinsic.name` / `intrinsic.signature` / `embed.abi.version` を追加。
   - 監査粒度: 関数単位・モジュール単位の記録範囲を明記する。
4. `docs/spec/3-8-core-runtime-capability.md` に `native.intrinsic` / `native.embed` の Capability を追加する。
   - Stage: 初期値は `Experimental` とし、昇格条件を `docs/notes/dsl/dsl-plugin-roadmap.md` に合わせて追記。
   - 監査キー連動: Capability と監査キーの対応表を追加する。
   - ブリッジ整合: `RuntimeBridgeAuditSpec` の key 体系と同一になるよう表記を統一する。

### フェーズB: Rust 実装 - Intrinsics
1. `compiler/frontend` に `@intrinsic` の AST/パーサ対応を追加し、型検証・診断 (`native.intrinsic.*`) を実装する。
   - パーサ: 属性引数の文字列リテラルのみ許可し、その他は構文診断にする。
   - AST: `Attribute::Intrinsic { name }` を追加し、ソース位置を保持する。
   - セマンティック: `effect {native}` 未付与時は `native.intrinsic.missing_effect` を出す。
   - 型検証: `Copy` 制約と ABI 安全型のホワイトリストを参照し、違反は `native.intrinsic.invalid_type`。
   - テスト: 既存の `frontend` 診断テストに `@intrinsic` 成功/失敗ケースを追加。
2. `compiler/backend/llvm` に LLVM intrinsic マッピングを追加し、未対応ターゲットではポリフィルにフォールバックする。
   - マッピング: 最小セットと拡張セットのテーブルを分離し、feature flag で切替。
   - 検証: 期待型と IR の整合チェックを行い、`native.intrinsic.signature_mismatch` を出す。
   - フォールバック: 未対応ターゲットでは `Core.Native` のポリフィル関数へ置換し、監査キーを記録。
   - 最適化属性: `readonly`/`readnone` 等は許可表に基づき限定的に付与。
3. 監査ログに `intrinsic` 名と引数型情報を記録し、`AuditEnvelope` に `native.intrinsic` メタデータを付与する。
   - 収集点: IR 生成時に `intrinsic.name` と `intrinsic.signature` を収集する。
   - 変換: 文字列表記を `docs/spec/3-6-core-diagnostics-audit.md` に合わせて正規化する。
   - 出力: 既存の監査ログ形式に合わせ、`native.intrinsic.used` を必ず出力する。

#### フロントエンド分解（`compiler/frontend`）
- `@intrinsic` 属性のパース追加（関数宣言のみ許可、引数は単一の文字列リテラル）。
- AST への属性情報付与（既存の attributes 構造に `Intrinsic` を追加）。
- セマンティック検証: `@intrinsic` は `extern` との併用禁止、`effect {native}` 必須。
- 型検証: intrinsic 宣言に不許可の型（非 `Copy`/未定義 ABI 型）が含まれる場合は `native.intrinsic.invalid_type` を出す。
- 解決フェーズで intrinsic 名の正規化（`llvm.sqrt.f64` など）と内部 ID への変換を行う。
- 監査ログ: `native.intrinsic.used` を関数単位で記録できるよう、IR へのメタデータ伝搬を追加する。

#### バックエンド分解（`compiler/backend/llvm`）
- intrinsic 対応表の土台追加（`llvm.sqrt.f64` / `llvm.ctpop.*` / `llvm.memcpy.*` を最小セットにする）。
- 型シグネチャ検証: intrinsic 期待型と IR の一致チェック（不一致時は `native.intrinsic.signature_mismatch`）。
- 未対応ターゲットのフォールバック: ポリフィル関数呼び出しへ差し替え、`native.intrinsic.polyfill` を監査に記録。
- 最適化属性の付与（`readonly`/`readnone`/`noalias` など）を慎重に設定し、監査メタデータに反映する。
- `feature = "native-unstable"` で拡張セット（SIMD/ベクタ）を限定的に解放する。

### フェーズC: Rust 実装 - Core.Native API
1. `compiler/runtime/src` に `native` モジュールを追加し、`Core.Native` として公開する。
   - モジュール設計: `core/native/mod.rs` を新設し、公開 API を `pub(crate)` と `pub` で分離。
   - 安全境界: `unsafe` を最小限に閉じ込め、呼び出し側は `effect {native}` を強制。
   - 監査連携: 各 API 呼び出しで `native.intrinsic.used` と `native.embed.entrypoint` を記録できる構造にする。
2. 最小 API（`memcpy`/`ctpop`/`sqrt` など）を `Core.Native` から呼べるようにし、`effect {native}` を要求する。
   - API 定義: 返却型と引数型の制約を明記し、非 `Copy` 型は拒否する。
   - 実装: LLVM intrinsic への直接マッピングかポリフィルへ分岐する。
   - ドキュメント: `docs/spec/3-0-core-library-overview.md` に `Core.Native` の概要を追加。
3. `examples/native/intrinsics` を追加し、`expected/` と監査ログを整備する。
   - サンプル内容: `sqrt` と `ctpop` を使う最小例を用意。
   - 期待値: `expected/` に実行ログと監査ログのスナップショットを追加。
   - シナリオ: `docs/plans/bootstrap-roadmap/assets/phase4-scenario-matrix.csv` と連携する ID を埋め込む。

### フェーズD: Rust 実装 - 埋め込み API
1. `compiler/runtime/native` もしくは `compiler/runtime` 配下に埋め込み用 C ABI 層（`reml_create_context` など）を実装する。
   - ABI 方針: C99 互換で `extern "C"` の関数群を定義し、`reml_*` 命名に統一。
   - ライフサイクル: `create`/`load`/`run`/`dispose` の最小フローを定義。
   - 安全性: 失敗時は `Result` 相当のエラーコードを返し、監査ログに `native.embed.*` を記録。
2. `docs/guides/runtime/runtime-bridges.md` に埋め込み API の最小利用手順を追記する。
   - 章追加: 「埋め込み API (Phase 4)」節を追加し、C からの呼び出し例を示す。
   - 互換性: ABI バージョンと互換性ルールを明記する。
   - 参照: `docs/spec/3-8-core-runtime-capability.md` と相互リンクを張る。
3. `examples/native/embedding` と `expected/` を追加し、Phase 4 シナリオへ登録する。
   - サンプル: 最小の C ホストから `reml_run` を呼び出す例を用意。
   - 期待値: 実行ログと監査ログを `expected/` に追加。
   - 参照更新: シナリオ ID とログ保存先を `reports/spec-audit/ch5` の README に追記。

### フェーズE: 研究プロトタイプ（ASM / LLVM IR）
1. `docs/notes/ffi/native-escape-hatches-research.md` の「Inline ASM」「LLVM IR」節を更新し、Rust 実装でのガード条件（feature flag / `@cfg`）を明記する。
   - 位置づけ: Phase 4 では「設計 + ガード付き PoC」に限定することを明記。
   - ガード: `feature = "native-unstable"` と `@cfg(target)` の併用要件を追記。
   - 監査: `native.intrinsic.unstable_used` を追加するか検討し、必要なら TODO を残す。
2. `compiler/backend/llvm` に `feature = "native-unstable"` のプロトタイプを追加し、サンプルを `examples/native/unstable` に隔離する。
   - 実装範囲: Inline ASM は解析のみ、LLVM IR 直書きはビルドガードで無効化。
   - サンプル: 実行不能であることを README に明記し、監査ログのみ確認可能にする。
   - 退避策: フィーチャ無効時に明示エラーを返し、クラッシュを防ぐ。

### フェーズF: Phase 4 回帰接続
1. `docs/plans/bootstrap-roadmap/assets/phase4-scenario-matrix.csv` に `NATIVE-INTRINSIC-001` / `NATIVE-EMBED-001` を追加する。
   - 追記事項: 依存 Capability、監査キー、対象ターゲットを列に追加する。
   - 参照: `docs/plans/bootstrap-roadmap/4-0-phase4-migration.md` のマイルストーンに紐づける。
2. `docs/plans/bootstrap-roadmap/4-1-spec-core-regression-plan.md` に関連シナリオの実行手順を追記する。
   - 手順: `examples/native/*` の実行と `expected/` 差分確認のステップを追加。
   - 診断: `native.intrinsic.*` と `native.embed.*` のログが出ることを確認項目にする。
3. `reports/spec-audit/ch5` にログが蓄積できるよう、実行手順と KPI を追記する。
   - KPI: 成功率、監査キー欠落率、フォールバック発生率を追跡指標にする。
   - 保存先: ログ命名規則と格納ディレクトリを統一する。

## 作業チェックリスト

### フェーズA: 仕様・監査・Capability 整合
- [x] `docs/spec/1-1-syntax.md` に `@intrinsic` の構文・制約・診断前提を追記
- [x] `docs/spec/1-3-effects-safety.md` に `effect {native}` の意味・`unsafe` との関係・`@cfg` 要件を追記
- [x] `docs/spec/3-6-core-diagnostics-audit.md` に `native.intrinsic.*` / `native.embed.*` の監査キーを定義
- [x] `docs/spec/3-8-core-runtime-capability.md` に `native.intrinsic` / `native.embed` を追加し Stage と対応表を整備
- [x] `docs/spec/1-0-language-core-overview.md` と `docs/spec/3-0-core-library-overview.md` の概要追記を確認

### フェーズB: Rust 実装 - Intrinsics
- [x] `compiler/frontend` に `@intrinsic` パース・AST・セマンティック検証を追加
- [x] `compiler/frontend` の型検証に `native.intrinsic.invalid_type` を追加
- [x] `compiler/backend/llvm` に LLVM intrinsic マッピングと署名検証を追加
- [x] 未対応ターゲットのポリフィル切替と `native.intrinsic.polyfill` 監査記録を確認
- [x] 監査ログに `intrinsic.name` / `intrinsic.signature` が出力されることを確認
- [x] `frontend` 診断テストに `@intrinsic` 成功/失敗ケースを追加

### フェーズC: Rust 実装 - Core.Native API
- [x] `compiler/runtime/src` に `core/native` モジュールを追加
- [x] `Core.Native` の最小 API（`memcpy`/`ctpop`/`sqrt`）を実装
- [x] `effect {native}` が必須になることを確認
- [x] `examples/native/intrinsics` と `expected/` を整備
- [x] `docs/spec/3-0-core-library-overview.md` に概要を追記

### フェーズD: Rust 実装 - 埋め込み API
- [x] 埋め込み用 C ABI 層（`reml_create_context` など）を実装
- [x] 失敗時のエラーコードと `native.embed.*` 監査記録を確認
- [x] `docs/guides/runtime/runtime-bridges.md` に埋め込み API 手順と互換性ルールを追記
- [x] `examples/native/embedding` と `expected/` を整備
- [x] `abi_mismatch` / `unsupported_target` の埋め込みサンプルを追加
- [x] `reports/spec-audit/ch5/logs/native-embed-*.md` に実行ログを記録
- [x] `reports/spec-audit/ch5` のログ保存ルールを更新

### フェーズE: 研究プロトタイプ（ASM / LLVM IR）
- [x] `docs/notes/ffi/native-escape-hatches-research.md` にガード条件と位置づけを追記
- [x] `feature = "native-unstable"` のプロトタイプを追加
- [x] `examples/native/unstable` を隔離し README で実行不能を明記
- [x] `native.intrinsic.unstable_used` の扱いを検討し TODO を残す

### フェーズF: Phase 4 回帰接続
- [x] `docs/plans/bootstrap-roadmap/assets/phase4-scenario-matrix.csv` にシナリオを追加
- [x] `docs/plans/bootstrap-roadmap/4-1-spec-core-regression-plan.md` に実行手順を追記
- [x] `reports/spec-audit/ch5` の KPI とログ命名規則を追記

## タイムライン（目安）

| 週 | タスク |
| --- | --- |
| 76 週 | フェーズA: 仕様・監査・Capability 整合 |
| 77 週 | フェーズB: Rust 実装 - Intrinsics |
| 78 週 | フェーズC: Rust 実装 - Core.Native API |
| 79 週 | フェーズD: Rust 実装 - 埋め込み API |
| 80 週 | フェーズE/F: ASM/LLVM IR プロトタイプと回帰接続 |

## リスクと緩和策

| リスク | 影響 | 緩和策 |
| --- | --- | --- |
| Intrinsic 名や ABI の不一致 | 実装と仕様の乖離、回帰失敗 | `docs/spec/1-1-syntax.md` と `compiler/backend/llvm` の対応表を同時更新し、`expected/` で差分監査 |
| `effect {native}` の乱用 | 安全性と監査の崩壊 | `native.intrinsic.*` / `native.embed.*` を必須監査キー化し、Capability で段階ゲート |
| 埋め込み API の互換性不足 | 既存ホストアプリとの統合が困難 | `docs/guides/runtime/runtime-bridges.md` に ABI 互換性ルールを明記し、最小 API から段階拡張 |

## 進捗状況
- 2025-12-XX: 計画作成（未着手）
- 2025-12-22: フェーズD 完了、埋め込み API の追加サンプルと実行ログを反映
- 2025-12-23: フェーズE の研究プロトタイプ（ASM / LLVM IR）を反映
- 2026-02-XX: フェーズF の回帰接続ドキュメント更新を反映

## 参照
- `docs/notes/ffi/native-escape-hatches-research.md`
- `docs/spec/0-1-project-purpose.md`
- `docs/spec/1-1-syntax.md`
- `docs/spec/1-3-effects-safety.md`
- `docs/spec/3-6-core-diagnostics-audit.md`
- `docs/spec/3-8-core-runtime-capability.md`
- `docs/guides/runtime/runtime-bridges.md`
