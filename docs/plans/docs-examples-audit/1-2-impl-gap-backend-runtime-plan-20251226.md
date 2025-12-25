# 1.2 実装ギャップ後続対応計画（Backend / Runtime / 2025-12-26）

`docs/plans/docs-examples-audit/1-2-impl-gap-plan-20251224-3.md` で実施した識別子受理拡張（emoji/ZWJ/bidi 取り込み）に伴い、Runtime と Backend の整合作業を切り出す。

## 目的
- Runtime の識別子境界判定を Frontend/仕様と一致させる。
- Backend の LLVM IR 名生成を非 ASCII 識別子に対応させる。

## 対象範囲
- Runtime: `compiler/rust/runtime/src/parse/combinator.rs`
- Backend: `compiler/rust/backend/llvm/src/codegen.rs`
- 仕様: `docs/spec/1-1-syntax.md`

## 背景
- Frontend の識別子 lex を拡張し、`Extended_Pictographic` / `Emoji_Component` / `U+200D` / `U+FE0F` を識別子継続として受理した。
- 仕様を更新し、上記の受理範囲と bidi 制御の拒否を明記した。
- Runtime/Backend には同等の受理・整形処理が存在しない。

## 実装修正計画

### フェーズ 1: Runtime の識別子境界整合
1) `is_ident_continue` の拡張
- 目的: Frontend と同じ継続判定でキーワード境界を評価する。
- 作業ステップ:
  - `compiler/rust/runtime/src/parse/combinator.rs` の `is_ident_continue` に `U+200D` / `U+FE0F` / `Extended_Pictographic` / `Emoji_Component` を追加する。
  - 既存の `IdentifierProfile::AsciiCompat` は変更しない。

2) テスト追加
- 目的: キーワード境界が emoji/ZWJ を含む識別子で誤認識しないことを確認する。
- 作業ステップ:
  - Runtime のテストに `keyword` + emoji 継続を含むケースを追加する。
  - `bidi` の拒否が診断として返ることを確認する。

### フェーズ 2: Backend の LLVM 名サニタイズ
1) LLVM IR 名の正規化関数を追加
- 目的: 非 ASCII 識別子を LLVM IR で安全に扱える形式へ変換する。
- 作業ステップ:
  - `compiler/rust/backend/llvm/src/codegen.rs` に `sanitize_llvm_ident` を追加する。
  - 文字集合は `[A-Za-z0-9_]` へ正規化し、それ以外は `_uXXXX` 形式で置換する方針を検討する。

2) 変換適用箇所の整理
- 目的: ローカル変数名・関数名・補助シンボル名の破綻を防ぐ。
- 作業ステップ:
  - `LlvmBuilder::new_tmp` の `hint` をサニタイズする。
  - `intrinsic_is_ctor` / `intrinsic_ctor_payload` に渡す `name` をサニタイズする。
  - `ModuleIr` / `Function` 名にユーザー識別子を含む経路を洗い出し、必要に応じてサニタイズする。

### フェーズ 3: 検証
- `cargo test --manifest-path compiler/rust/runtime/Cargo.toml`
- `cargo test --manifest-path compiler/rust/backend/llvm/Cargo.toml`
- emoji を含む識別子で Backend の IR 出力が崩れないことを確認する（必要なら簡易サンプルを追加）。

## 受け入れ基準
- Runtime の `keyword` が emoji/ZWJ を含む識別子で誤検出しない。
- Backend の LLVM IR 出力が非 ASCII 識別子を含んでも生成エラーにならない。
- 既存テストにリグレッションがない。

## 進捗管理
- 本計画書作成日: 2025-12-26
- 進捗欄（運用用）:
  - [ ] フェーズ 1 完了
  - [ ] フェーズ 2 完了
  - [ ] フェーズ 3 完了

## 関連リンク
- `docs/plans/docs-examples-audit/1-2-impl-gap-plan-20251224-3.md`
- `docs/plans/docs-examples-audit/1-2-impl-gap-backend-runtime-impact-note-20251226.md`
- `docs/spec/1-1-syntax.md`
