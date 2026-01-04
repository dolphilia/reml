# Phase 4: Inline ASM / LLVM IR 本格実装計画

## 背景と決定事項
- `docs/notes/ffi/native-escape-hatches-research.md` で示した通り、`Core.Ffi` だけでは SIMD/低レベル最適化/埋め込み用途に不足があり、Inline ASM と LLVM IR 直書きの本格実装が必要。
- `docs/plans/bootstrap-roadmap/4-1-native-escape-hatches-plan.md` では研究プロトタイプに限定していたが、Phase 4 での実用回帰の早期検証を優先し **本格実装へ前倒し**する。
- 研究ノート/仕様の「PoC 解析のみ」方針は **本格実装**へ更新する（本計画の完了後に `docs/notes` / `docs/spec` を同期更新する）。
- Rust 実装は `native-unstable` のガードと監査ログは用意済みだが、構文・型検証・LLVM IR 生成が未実装であるため、仕様 → フロントエンド → バックエンド → 監査の順に拡張する。

## 目的
1. Rust 実装で Inline ASM / LLVM IR 直書きを **実行可能なレベル**まで実装し、Phase 4 実用シナリオに接続する。
2. `!{native}` と Capability / 監査ログを統合し、移植性と安全性のリスクを明確化する。`unsafe` は **ブロック境界**として扱い、`native` 効果の代替にしない。
3. LLVM IR 直書きの構文・検証ルールを整理し、Rust バックエンドの **新しい検証ステップ**で検査できる形で実装する。

## 現状調査（Rust 実装の状態）
- **フロントエンド**: `compiler/frontend/src/parser/mod.rs` には Inline ASM / LLVM IR 直書きの構文が存在しない。`@unstable("inline_asm")` は属性として保持されるのみで、専用の診断や型検証がない。
- **Typed/MIR**: `compiler/frontend/src/semantics/typed.rs` / `compiler/frontend/src/semantics/mir.rs` に Inline ASM / LLVM IR の式ノードが存在しない（`MirExprKind` は `Unknown` で退避）。
- **バックエンド**: `compiler/backend/llvm/src/unstable.rs` が `unstable:inline_asm` / `unstable:llvm_ir` を検出し、`verify.rs` が `native.unstable.disabled` を出すのみ。IR 生成は未実装。
- **サンプル**: `examples/native/unstable/inline-asm-prototype.reml` は「解析のみ」前提の素振りで、実行不能を `examples/native/unstable/README.md` が明記。
- **監査**: `native.intrinsic.unstable_used` の監査はあるが、Inline ASM / LLVM IR 専用の監査キーは未定義（`docs/spec/3-6-core-diagnostics-audit.md` に未反映）。

## 設計方針
- **安全性**: Inline ASM / LLVM IR は `!{native}` を必須とし、`unsafe` ブロック外での利用を禁止する。`unsafe` は **局所境界**であり `native` 効果の代替にはならない。
- **移植性**: Inline ASM は `@cfg(target_arch, target_os)` を必須とし、LLVM IR 直書きも `@cfg(target)` を要求する。
- **ガード**: Phase 4 では `feature = "native-unstable"` を維持し、Capability Stage を `Experimental` として明示。将来の昇格条件は `docs/spec/3-8-core-runtime-capability.md` に明記する。
- **監査**: 監査ログは関数単位で「何が」「どのターゲットで」「どの制約で」使われたかを残す。
- **Cranelift の扱い**: Cranelift は **JIT バックエンド枠**で検討し、Inline ASM / LLVM IR のエスケープハッチ用途は **別計画**で扱う（本計画には含めない）。

## スコープ
- **含む**:
  - Inline ASM / LLVM IR 直書きの構文と型検証
  - MIR/LLVM IR 生成パイプラインの拡張
  - 監査キー・Capability の追加
  - Phase 4 シナリオと回帰ログの整備
- **含まない**:
  - syscall のフルサポート
  - LLVM 最適化パスの自動選別（手動フラグは別タスク）
  - OS 別 ABI の完全対応

## 成果物
- `docs/spec/1-1-syntax.md` / `docs/spec/1-5-formal-grammar-bnf.md` に Inline ASM / LLVM IR 構文を追加。
- `docs/spec/1-3-effects-safety.md` に `!{native}` と `unsafe` ブロック境界の扱い、`@cfg` 要件を追記。
- `docs/spec/3-6-core-diagnostics-audit.md` に `native.inline_asm.*` / `native.llvm_ir.*` の監査キーを追加。
- `docs/spec/3-8-core-runtime-capability.md` に `native.inline_asm` / `native.llvm_ir` の Capability を追加。
- Rust 実装で Inline ASM / LLVM IR 直書きが実行可能になり、`examples/native/asm` / `examples/native/llvm_ir` が動作する。

## 作業フェーズ

### フェーズA: 仕様整備
1. `docs/spec/1-1-syntax.md` に Inline ASM / LLVM IR 構文を追加する。
   - Inline ASM: `inline_asm("rdtsc", outputs(...), inputs(...), clobbers(...), options(...))` の形式を採用。
   - LLVM IR: `llvm_ir!(i32) { "...", inputs(a, b) }` の形式を採用。
   - `@unstable("inline_asm")` / `@unstable("llvm_ir")` の要件を明記する。
2. `docs/spec/1-5-formal-grammar-bnf.md` に BNF を追加する。
3. `docs/spec/1-3-effects-safety.md` に `!{native}` と `unsafe` ブロック境界、`@cfg` 要件を追加する。
4. `docs/spec/3-6-core-diagnostics-audit.md` に監査キーを定義する。
   - Inline ASM: `native.inline_asm.used`, `native.inline_asm.disabled`, `native.inline_asm.invalid_constraint`
   - LLVM IR: `native.llvm_ir.used`, `native.llvm_ir.verify_failed`, `native.llvm_ir.invalid_placeholder`
   - メタデータ: `asm.template_hash`, `asm.constraints`, `llvm_ir.template_hash`, `llvm_ir.inputs`
5. `docs/spec/3-8-core-runtime-capability.md` に Capability を追加する。
   - Stage: `Experimental`
   - 監査キーと 1:1 対応表を追加

### フェーズB: フロントエンド構文と型検証
1. `compiler/frontend/src/parser/mod.rs` に Inline ASM / LLVM IR の式構文を追加する。
2. `compiler/frontend/src/semantics/typed.rs` と `compiler/frontend/src/semantics/mir.rs` に式ノードを追加する。
3. `compiler/frontend/src/typeck/driver.rs` で型制約を検証する。
   - `!{native}` がない場合は `native.inline_asm.missing_effect` / `native.llvm_ir.missing_effect`
   - 引数/戻り値の ABI 安全性検証（`Copy` + プリミティブ / `ptr` のみ）
   - `@cfg` 未指定時は `native.inline_asm.missing_cfg` / `native.llvm_ir.missing_cfg`
4. `@unstable` を MIR 属性へ流す変換を `@intrinsic` と同様に追加する。
5. 解析・型検証のテストを追加する。
   - `frontend` 診断テストに成功/失敗ケースを追加

### フェーズC: MIR / 変換パス拡張
1. `compiler/frontend/src/semantics/mir.rs` の JSON 形式に Inline ASM / LLVM IR ノードを追加する。
2. `compiler/backend/llvm/src/integration.rs` の MIR ローダに新ノードを反映する。
3. 新ノードに合わせた `MirExprKind` を `compiler/backend/llvm/src/codegen.rs` に追加する。

### フェーズD: LLVM IR 生成
1. Inline ASM を LLVM IR の `call asm` へ変換する。
   - `outputs` / `inputs` / `clobbers` / `options` を制約文字列に変換する。
   - `volatile` / `sideeffect` の付与ルールを整理する。
2. LLVM IR 直書きのテンプレートを構文解析し、SSA 名の衝突を防ぐ。
   - `$0` / `$1` のようなプレースホルダを入力/出力へマッピングする。
   - Rust バックエンドの新しい検証ステップで失敗した場合は `native.llvm_ir.verify_failed` を出す。
3. Rust バックエンドの検証ステップを追加し、監査ログへメタデータを出力する。

### フェーズE: 監査ログ / Capability / Runtime 連携
1. `compiler/runtime/src/native` に Inline ASM / LLVM IR の監査メタデータ収集を追加する。
2. `AuditEnvelope.metadata` への追加キーを `docs/spec/3-6-core-diagnostics-audit.md` と整合させる。
3. Capability Registry へ `native.inline_asm` / `native.llvm_ir` を登録する。

### フェーズF: サンプルと回帰接続
1. `examples/native/asm` / `examples/native/llvm_ir` を追加する。
   - `inline_asm_rdtsc.reml` / `llvm_ir_add_i32.reml` などの最小例
2. `expected/native/` に stdout / audit JSONL を追加する。
3. `docs/plans/bootstrap-roadmap/assets/phase4-scenario-matrix.csv` に `NATIVE-ASM-001` / `NATIVE-LLVMIR-001` を追加する。
4. `docs/plans/bootstrap-roadmap/4-1-spec-core-regression-plan.md` に実行手順とログ保存先を追記する。

## 作業チェックリスト

### フェーズA: 仕様整備
- [x] `docs/spec/1-1-syntax.md` に Inline ASM / LLVM IR 構文を追加
- [x] `docs/spec/1-5-formal-grammar-bnf.md` に BNF を追加
- [x] `docs/spec/1-3-effects-safety.md` に `!{native}` と `unsafe` ブロック境界、`@cfg` 要件を追加
- [x] `docs/spec/3-6-core-diagnostics-audit.md` に監査キーを追加
- [x] `docs/spec/3-8-core-runtime-capability.md` に Capability を追加

### フェーズB: フロントエンド構文と型検証
- [ ] Parser に Inline ASM / LLVM IR を追加
- [ ] Typed/MIR ノードを追加
- [ ] 型検証（effect/cfg/ABI 制約）を追加
- [ ] `@unstable` を MIR 属性へ流す変換を追加
- [ ] 診断テストを追加

### フェーズC: MIR / 変換パス拡張
- [ ] MIR JSON のスキーマ更新
- [ ] MIR ローダ更新
- [ ] バックエンドの MirExprKind 追加

### フェーズD: LLVM IR 生成
- [x] Inline ASM の `call asm` 生成
- [x] LLVM IR 直書きテンプレート変換
- [x] Rust バックエンド検証ステップの診断/監査更新

### フェーズE: 監査ログ / Capability / Runtime 連携
- [x] `compiler/runtime/src/native` の監査連携
- [x] `AuditEnvelope.metadata` へのキー追加
- [x] Capability Registry 更新

### フェーズF: サンプルと回帰接続
- [x] `examples/native/asm` / `examples/native/llvm_ir` を追加
- [x] `expected/native/` のゴールデン更新
- [x] `phase4-scenario-matrix.csv` にシナリオ追加
- [x] `4-1-spec-core-regression-plan.md` に実行手順を追記

## タイムライン（目安）

| 週 | タスク |
| --- | --- |
| 81 週 | フェーズA: 仕様整備 |
| 82 週 | フェーズB: フロントエンド構文と型検証 |
| 83 週 | フェーズC: MIR / 変換パス拡張 |
| 84-85 週 | フェーズD: LLVM IR 生成 |
| 86 週 | フェーズE/F: 監査連携と回帰接続 |

## リスクと緩和策

| リスク | 影響 | 緩和策 |
| --- | --- | --- |
| Inline ASM の制約記法が曖昧 | バックエンドが不正な `call asm` を生成 | `docs/guides/compiler/llvm-integration-notes.md` の Inline ASM 仕様に準拠し、`native.inline_asm.invalid_constraint` を追加する |
| LLVM IR 直書きが LLVM バージョン差に影響 | `opt -verify` が失敗 | `llvm_ir.template_hash` を監査に記録し、失敗例を `reports/spec-audit/ch5/logs/` に保存 |
| `!{native}` の乱用 | 安全性の崩壊 | `native.inline_asm.used` / `native.llvm_ir.used` を必須監査キー化し、Capability でゲート |

## TODO（調査メモ付き）
- Inline ASM の constraint 仕様は `docs/notes/ffi/native-escape-hatches-research.md` と `docs/guides/compiler/llvm-integration-notes.md` の差分を確認して決定する。
- LLVM IR 直書きのテンプレート構文は `compiler/backend/llvm/src/codegen.rs` の IR 生成方針と整合させる。

## 参照
- `docs/notes/ffi/native-escape-hatches-research.md`
- `docs/spec/0-1-project-purpose.md`
- `docs/spec/1-1-syntax.md`
- `docs/spec/1-3-effects-safety.md`
- `docs/spec/1-5-formal-grammar-bnf.md`
- `docs/spec/3-6-core-diagnostics-audit.md`
- `docs/spec/3-8-core-runtime-capability.md`
- `docs/guides/compiler/llvm-integration-notes.md`
- `docs/plans/bootstrap-roadmap/4-1-native-escape-hatches-plan.md`
