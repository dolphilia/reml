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
  - `compiler/rust/runtime/src/parse/combinator.rs` の `is_ident_continue` の現行分岐と使用箇所を確認する（キーワード境界・トークン境界の呼び出し経路を洗い出す）。
  - `U+200D` / `U+FE0F` / `Extended_Pictographic` / `Emoji_Component` を追加する実装方針を整理する（`unicode-ident` を使うか、既存の判定関数に合流させるかを決める）。
  - Frontend の識別子継続判定と同じ条件式になるよう実装を追加する。
  - 既存の `IdentifierProfile::AsciiCompat` は変更せず、AsciiCompat のままでも期待通りに境界が保たれることを確認する。

2) テスト追加
- 目的: キーワード境界が emoji/ZWJ を含む識別子で誤認識しないことを確認する。
- 作業ステップ:
  - Runtime の既存テストモジュールを調査し、識別子境界/キーワード判定に近いテストファイルを特定する。
  - `keyword` + emoji 継続（`keyword🚀` / `keyword👨‍💻` / `keyword\u{200D}` / `keyword\u{FE0F}`）の入力を追加し、キーワード判定が発火しないことを確認する。
  - `bidi` 制御文字を混在させた入力を追加し、拒否診断が返ることを確認する。
  - 既存の ASCII-only テストが影響を受けないことを確認する（境界判定の既存挙動の差分チェック）。

進捗メモ:
- [x] `is_ident_continue` に `U+200D` / `U+FE0F` / `Extended_Pictographic` / `Emoji_Component` を追加
- [x] キーワード境界テストに emoji/ZWJ/VS16 と bidi 制御のケースを追加
- [ ] Runtime のテスト実行（未実施）

### フェーズ 2: Backend の LLVM 名サニタイズ
1) LLVM IR 名の正規化関数を追加
- 目的: 非 ASCII 識別子を LLVM IR で安全に扱える形式へ変換する。
- 作業ステップ:
  - `compiler/rust/backend/llvm/src/codegen.rs` の命名処理（`LlvmBuilder` 周辺）を確認し、識別子を加工している箇所を洗い出す。
  - `sanitize_llvm_ident` の仕様を決める（`[A-Za-z0-9_]` のみ許可、その他は `_uXXXX` へ置換、先頭が数字の場合は `_` を付与する等）。
  - サニタイズの実装を追加し、変換後も空文字にならないようフォールバック名（例: `_u0000`）を用意する。
  - 仕様（`docs/spec/1-1-syntax.md`）の識別子範囲と差分が出ることを明記し、Backend 側の正規化が内部表現であることを説明する。

2) 変換適用箇所の整理
- 目的: ローカル変数名・関数名・補助シンボル名の破綻を防ぐ。
- 作業ステップ:
  - `LlvmBuilder::new_tmp` の `hint` 生成をサニタイズし、既存の一時名規則（連番・スコープ情報）を維持する。
  - `intrinsic_is_ctor` / `intrinsic_ctor_payload` に渡す `name` をサニタイズし、既存の `intrinsic` 名の組み立てが壊れないことを確認する。
  - `ModuleIr` / `Function` 名など、ユーザー識別子を含む経路を確認し、適用対象の一覧を作る。
  - 変換適用後の LLVM IR で名前衝突が発生する可能性を検討し、必要なら衝突回避（サフィックス付与など）を追加する。

### フェーズ 3: 検証
- `cargo test --manifest-path compiler/rust/runtime/Cargo.toml` を実行し、識別子境界テストの追加分を確認する。
- `cargo test --manifest-path compiler/rust/backend/llvm/Cargo.toml` を実行し、LLVM 名サニタイズの変更で既存テストが落ちないことを確認する。
- emoji を含む識別子のサンプルを用意し、IR 出力でサニタイズ結果が期待通りの形式になることを確認する（必要なら簡易サンプルを追加）。
- 仕様上の識別子範囲と Backend 内部名の差分がドキュメント化されていることを確認する。

#### 実施結果
- Runtime: `cargo test --manifest-path compiler/rust/runtime/Cargo.toml` は `text::pretty::tests::render_group_uses_flat_layout_when_it_fits` が失敗（`compiler/rust/runtime/src/text/pretty.rs:279`）。
- Backend: `cargo test --manifest-path compiler/rust/backend/llvm/Cargo.toml` は成功。
- emoji 識別子の IR 出力確認は未実施（簡易サンプルの追加が必要）。
- 仕様差分の注記は `docs/spec/1-1-syntax.md` に記載済み。

## 受け入れ基準
- Runtime の `keyword` が emoji/ZWJ を含む識別子で誤検出しない。
- Backend の LLVM IR 出力が非 ASCII 識別子を含んでも生成エラーにならない。
- 既存テストにリグレッションがない。

## 進捗管理
- 本計画書作成日: 2025-12-26
- 進捗欄（運用用）:
  - [x] フェーズ 1 完了
  - [x] フェーズ 2 完了
  - [ ] フェーズ 3 完了

## 関連リンク
- `docs/plans/docs-examples-audit/1-2-impl-gap-plan-20251224-3.md`
- `docs/plans/docs-examples-audit/1-2-impl-gap-backend-runtime-impact-note-20251226.md`
- `docs/spec/1-1-syntax.md`
