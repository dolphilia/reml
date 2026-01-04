# 1.2 実装ギャップ対応計画（Rust Frontend / 2025-12-23）

`docs/spec/1-1-syntax.md` のサンプル修正で判明した **仕様と実装のギャップ** を整理し、Rust Frontend 側で追随するための対応計画を定義する。

## 目的
- 仕様に合わせて Rust Frontend の受理範囲を拡張する。
- 仕様サンプルの簡略化を段階的に解消し、正準例へ戻す。

## 対象範囲
- 仕様章: `docs/spec/1-1-syntax.md`
- サンプル: `examples/docs-examples/spec/1-1-syntax/*.reml`
- 監査ログ: `reports/spec-audit/ch1/docs-examples-fix-notes-20251223.md`

## ギャップ一覧（簡略化／回避済み）

### 1. `conductor` 宣言が未対応
- 影響: B.1.1 / B.8.3.2 の仕様は `conductor` をエントリポイントとして想定しているが、実装は構文受理できない。
- 該当サンプル:
  - `examples/docs-examples/spec/1-1-syntax/sec_b_1_1.reml`
  - `examples/docs-examples/spec/1-1-syntax/sec_b_8_3_2.reml`
- 現状の回避: `fn` 宣言へ置換。

### 2. `unsafe` ブロックが未対応
- 影響: 仕様は `unsafe` ブロックを前提とするが、実装では構文エラーとなる。
- 該当サンプル:
  - `examples/docs-examples/spec/1-1-syntax/sec_c_7.reml`
  - `examples/docs-examples/spec/1-1-syntax/sec_section-b.reml`
- 現状の回避: `unsafe` を削除し、コメントで注意書きを追加。

### 3. C 可変長引数 (`...`) が未対応
- 影響: 仕様の FFI 例に含まれる `printf(fmt: Ptr<u8>, ...)` を実装が受理できない。
- 該当サンプル:
  - `examples/docs-examples/spec/1-1-syntax/sec_b_4-f.reml`
- 現状の回避: `...` を削除した簡略宣言。

### 4. トップレベル式（制御構文・handle・match など）が未受理
- 影響: 仕様は式例をトップレベルで提示するが、実装はトップレベルで宣言のみを許容する。
- 該当サンプル:
  - `examples/docs-examples/spec/1-1-syntax/sec_b_5-c.reml`
  - `examples/docs-examples/spec/1-1-syntax/sec_c_4-a.reml`
  - `examples/docs-examples/spec/1-1-syntax/sec_c_4-b.reml`
  - `examples/docs-examples/spec/1-1-syntax/sec_c_4-c.reml`
  - `examples/docs-examples/spec/1-1-syntax/sec_c_4-d.reml`
  - `examples/docs-examples/spec/1-1-syntax/sec_c_4-e.reml`
  - `examples/docs-examples/spec/1-1-syntax/sec_e_2.reml`
- 現状の回避: `fn` でラップ。

## 実装修正計画（Rust Frontend）

### フェーズ 1: 構文受理（最小限のパース対応）
1) `conductor` 宣言のパーサ追加
- 目的: トップレベルで `conductor` を受理し、AST へ格納できるようにする。
- 成果物: `conductor` を含むサンプルが `--emit-diagnostics` で 0 件になる。
- 対象サンプル: `sec_b_1_1`, `sec_b_8_3_2`

2) `unsafe` ブロックのパーサ追加
- 目的: `unsafe { ... }` 構文を式として受理。
- 成果物: `sec_c_7`, `sec_section-b` が構文エラーにならない。

3) 外部関数宣言での `...` 受理
- 目的: `extern "C" fn printf(fmt: Ptr<u8>, ...)` の可変長引数を許容。
- 成果物: `sec_b_4-f` が構文エラーにならない。

4) トップレベル式の許可（暫定）
- 目的: 仕様サンプルの検証用に、トップレベルで `match`/`if`/`while`/`for`/`loop`/`handle` 等の式を受理。
- 成果物: 対象サンプルが `fn` ラップ無しで解析できる。
- 注意: 恒久対応が難しい場合は「サンプル検証モード」などの RunConfig 拡張を検討。

### フェーズ 2: AST/診断の整合
1) `conductor` の意味解析を追加
- 目的: `@dsl_export` や `Parser<T>` との連携を見据え、宣言の型情報を保持。
- 成果物: 章別監査ログに `conductor` 由来の診断が追加されない。
- 進捗: 実装済み（Typed/MIR に conductor 定義の型情報を保持、dsl_id 重複診断を追加）

2) `unsafe` ブロックの制約チェック
- 目的: `unsafe` が許容される位置や属性制約を明示。
- 成果物: `1-3-effects-safety.md` との矛盾がない診断ルール。
- 進捗: 実装済み（`@pure` 文脈での `unsafe` 使用を診断化）

3) 可変長引数の型制約
- 目的: `...` の後続引数禁止、`extern` のみ許可などの制約を実装。
- 成果物: 不正構文に対して適切な診断が出る。
- 進捗: 実装済み（`extern "C"` 以外の varargs/固定引数欠落を診断化）

4) トップレベル式の扱い方針を確定
- 目的: 仕様上の許容範囲と実装上の制約を整理。
- 成果物: `docs/spec/1-1-syntax.md` に実装差分注記が不要になるか、または `RunConfig` で明示。
- 方針: `RunConfig` で明示する（通常モードでは拒否、サンプル検証モードで許可）。
- 進捗: 解析/型推論の受理は追加済み。`RunConfig` 連携は未対応。
- 進捗: 解析/型推論の受理と `RunConfig` 連携、診断化まで対応済み。
- TODO:
  - `RunConfig` にトップレベル式の許可フラグを追加し、CLI/ドライバ設定で切り替え可能にする（対応済み）。
  - 解析時はフラグ未設定ならトップレベル式を診断化し、サンプル検証モードでのみ許可する（診断化も対応済み）。

### フェーズ 3: サンプル復元と再検証
1) サンプルを仕様寄りに戻す
- `conductor` や `unsafe` を元の表現へ復元。
- `printf(... )` の可変長引数表現を復元。
- トップレベル式の例を関数ラップ無しに戻す（可能なら）。

2) 監査ログ更新
- `reports/spec-audit/summary.md` に再検証結果を記録。
- `reports/spec-audit/ch1/docs-examples-fix-notes-YYYYMMDD.md` を更新。

## Backend / Runtime 影響の再確認（2025-12-24 追記）
- `conductor` / `unsafe` / トップレベル式は **Rust Frontend 側の受理・診断が中心**であり、現行のドキュメント監査用途では Backend / Runtime の追加修正は不要。
- `conductor` は実行フェーズで `CapabilityRegistry::verify_conductor_contract` と連動する設計のため、CLI/実行系の統合フェーズでは **Runtime 連携の再点検が必須**（仕様: `docs/spec/3-8-core-runtime-capability.md`）。
- C 可変長引数 (`...`) は **Frontend では構文・型診断まで対応済み**だが、`compiler/backend/llvm/src/ffi_lowering.rs` の `FfiCallSignature` に variadic 情報が無く、バックエンド lower で反映できない。
- Runtime 側には `compiler/runtime/src/ffi/dsl/mod.rs` に `FfiFnSig.variadic` が存在するため、**実行系で varargs を使う場合は Frontend → Backend/Runtime のシグネチャ伝搬を追加**する必要がある（サンプルが宣言のみの場合は当面不要）。

## 進捗管理
- 本計画書作成日: 2025-12-23
- 進捗欄（運用用）:
  - [x] フェーズ 1 完了
  - [x] フェーズ 2 完了
  - [x] フェーズ 3 完了

## 関連リンク
- `docs/spec/1-1-syntax.md`
- `docs/plans/docs-examples-audit/1-2-spec-sample-fix-plan.md`
- `reports/spec-audit/ch1/docs-examples-fix-notes-20251223.md`
