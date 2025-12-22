# Phase4: Native Escape Hatches 実装計画

## 背景と決定事項
- `docs/notes/native-escape-hatches-research.md` で示した通り、`Core.Ffi` だけでは SIMD/低レベル最適化/埋め込み用途にギャップがある。
- `docs/spec/0-1-project-purpose.md` の「実用に耐える性能」「エコシステム統合」達成には、Rust 実装でのネイティブ拡張の足場が必要。
- Phase 4 の実用シナリオ回帰に接続できる最小スコープを定義し、過度に危険な機能（全面的な asm/syscall）を段階導入で扱う。

## 目的
1. `Core.Native`（または `Core.Intrinsics`）の仕様・監査・Capability 方針を整理し、Rust 実装に落とし込む。
2. Rust 実装において「intrinsic 連携」「埋め込み API」の最小実装を行い、Phase 4 シナリオへ接続する。
3. インライン ASM / LLVM IR 直書きは **設計 + ガード付きプロトタイプ** までを Phase 4 に含め、正式実装は後続フェーズへ引き継ぐ。

## スコープ
- **含む**: `@intrinsic` 属性の設計/検証、LLVM intrinsic マッピング、`Core.Native` の最小 API、埋め込み API の最小 C ABI、監査ログ/Capability 整合、Phase 4 シナリオ登録。
- **含まない**: 汎用 ASM の正式仕様化、syscall のフルサポート、OS 別 ABI 完全対応、LLVM IR 直書きの本実装。

## 成果物
- `docs/spec/1-1-syntax.md` と `docs/spec/1-3-effects-safety.md` に `effect {native}` と `@intrinsic` の最小仕様が反映される。
- `compiler/rust/backend/llvm` に LLVM intrinsic マッピングが入り、`compiler/rust/frontend` で属性検証と診断が整備される。
- `compiler/rust/runtime` に `Core.Native` の最小 API が追加され、監査キーが `docs/spec/3-6-core-diagnostics-audit.md` と一致する。
- 埋め込み API の最小 C ABI (`libreml` 相当) が Rust 実装側に実装され、簡易サンプルが動作する。
- `docs/plans/bootstrap-roadmap/assets/phase4-scenario-matrix.csv` に Native 系シナリオが追加される。

## 作業ステップ

### フェーズA: 仕様・監査・Capability 整合
1. `docs/spec/1-1-syntax.md` に `@intrinsic` 属性の構文と制約を追記する。
2. `docs/spec/1-3-effects-safety.md` に `effect {native}` の意味、`unsafe` との関係、`@cfg` 要件を追記する。
3. `docs/spec/3-6-core-diagnostics-audit.md` に `native.intrinsic.*` / `native.embed.*` の監査キーを定義する。
4. `docs/spec/3-8-core-runtime-capability.md` に `native.intrinsic` / `native.embed` の Capability を追加する。

### フェーズB: Rust 実装 - Intrinsics
1. `compiler/rust/frontend` に `@intrinsic` の AST/パーサ対応を追加し、型検証・診断 (`native.intrinsic.*`) を実装する。
2. `compiler/rust/backend/llvm` に LLVM intrinsic マッピングを追加し、未対応ターゲットではポリフィルにフォールバックする。
3. 監査ログに `intrinsic` 名と引数型情報を記録し、`AuditEnvelope` に `native.intrinsic` メタデータを付与する。

### フェーズC: Rust 実装 - Core.Native API
1. `compiler/rust/runtime/src` に `native` モジュールを追加し、`Core.Native` として公開する。
2. 最小 API（`memcpy`/`ctpop`/`sqrt` など）を `Core.Native` から呼べるようにし、`effect {native}` を要求する。
3. `examples/native/intrinsics` を追加し、`expected/` と監査ログを整備する。

### フェーズD: Rust 実装 - 埋め込み API
1. `runtime/native` もしくは `compiler/rust/runtime` 配下に埋め込み用 C ABI 層（`reml_create_context` など）を実装する。
2. `docs/guides/runtime-bridges.md` に埋め込み API の最小利用手順を追記する。
3. `examples/native/embedding` と `expected/` を追加し、Phase 4 シナリオへ登録する。

### フェーズE: 研究プロトタイプ（ASM / LLVM IR）
1. `docs/notes/native-escape-hatches-research.md` の「Inline ASM」「LLVM IR」節を更新し、Rust 実装でのガード条件（feature flag / `@cfg`）を明記する。
2. `compiler/rust/backend/llvm` に `feature = "native-unstable"` のプロトタイプを追加し、サンプルを `examples/native/unstable` に隔離する。

### フェーズF: Phase 4 回帰接続
1. `docs/plans/bootstrap-roadmap/assets/phase4-scenario-matrix.csv` に `NATIVE-INTRINSIC-001` / `NATIVE-EMBED-001` を追加する。
2. `docs/plans/bootstrap-roadmap/4-1-spec-core-regression-plan.md` に関連シナリオの実行手順を追記する。
3. `reports/spec-audit/ch4` にログが蓄積できるよう、実行手順と KPI を追記する。

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
| Intrinsic 名や ABI の不一致 | 実装と仕様の乖離、回帰失敗 | `docs/spec/1-1-syntax.md` と `compiler/rust/backend/llvm` の対応表を同時更新し、`expected/` で差分監査 |
| `effect {native}` の乱用 | 安全性と監査の崩壊 | `native.intrinsic.*` / `native.embed.*` を必須監査キー化し、Capability で段階ゲート |
| 埋め込み API の互換性不足 | 既存ホストアプリとの統合が困難 | `docs/guides/runtime-bridges.md` に ABI 互換性ルールを明記し、最小 API から段階拡張 |

## 進捗状況
- 2025-12-XX: 計画作成（未着手）

## 参照
- `docs/notes/native-escape-hatches-research.md`
- `docs/spec/0-1-project-purpose.md`
- `docs/spec/1-1-syntax.md`
- `docs/spec/1-3-effects-safety.md`
- `docs/spec/3-6-core-diagnostics-audit.md`
- `docs/spec/3-8-core-runtime-capability.md`
- `docs/guides/runtime-bridges.md`
