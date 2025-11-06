# Rust フロントエンド用クレート評価メモ（W1 草稿）

W1「Rust フロントエンド骨格の用意」で洗い出した依存候補クレートの比較メモ。  
採用可否は P1 W2 の AST/IR 対応表作業および W3 の型推論移植に向けて見直す。

## 1. Lexer 系候補

| クレート | 評価ポイント | 懸念点 | 方針 |
| --- | --- | --- | --- |
| `logos` | - マクロベースで高速な DFA を生成。<br>- `Source` トレイトでバイト列／UTF-8 を柔軟に扱える。<br>- `logos::Span` を `rem_frontend::span::Span` に写像しやすい。 | - 状態遷移のカスタマイズが限定的。<br>- packrat メトリクスと連携するには独自ラッパが必要。 | Lexing PoC で最優先に評価。`lexer::SourceBuffer` から `logos::Source` へ移譲するアダプタを作成。 |
| `rowan` | - トークンツリー構造を構築しながらノード再利用が可能。<br>- LSP/エディタ連携向けには有利。 | - 本フェーズでは重すぎる。<br>- AST/IR 設計と二重管理になる。 | P2 以降でシンタックスツリー最適化が必要になった場合に再評価。 |
| `logos-derive` | - カスタム derive で `logos` の列挙飽和を補助。 | - メンテナンス性が `logos` 本体に依存。 | `logos` 採用時に必要なら導入。現時点では候補保持のみ。 |

## 2. Parser 系候補

| クレート | 評価ポイント | 懸念点 | 方針 |
| --- | --- | --- | --- |
| `chumsky` | - コンビネータスタイルで Menhir の規則を直接写しやすい。<br>- エラー回復 API が豊富。 | - Packrat キャッシュは自前実装が必要。<br>- 巨大ファイルでの性能実績が限定的。 | `parser::ParserDriver` のプロトタイプで使用。性能確認までは optional 依存扱い。 |
| `pomelo` | - LR(1) ジェネレータ。Menhir 互換の宣言記法。<br>- 生成後のテーブルを Rust コードに埋め込める。 | - ドキュメントが少なく、コミュニティサポートが限定的。<br>- Windows でのビルド検証報告が不足。 | `docs/notes/parser-generator-survey.md` が準備できるまでは保留。必要に応じて PoC 分岐を切る。 |
| `lalrpop` | - 実績豊富。 | - 生成物のサイズが大きい。<br>- `no_std` 未対応。<br>- エラー復旧が弱い。 | `dual-write` の互換性維持が難しいため、現時点では除外。 |

## 3. 診断・メトリクス関連

| クレート | 評価ポイント | 懸念点 | 方針 |
| --- | --- | --- | --- |
| `serde` / `serde_json` | - 現行 OCaml 実装と同等の JSON 構造を作成可能。<br>- `#[serde(flatten)]` 等で拡張フィールドを扱いやすい。 | - 有無を言わさず採用（懸念なし）。 | `Cargo.toml` の必須依存として追加予定。 |
| `indexmap` | - JSON 直列化時にキー順序を固定可能。<br>- Packrat メトリクスの保持にも流用できる。 | - バージョンアップで `serde` 連携 API が変わる可能性。 | 診断 JSON の順序制御に利用。W4 までに導入。 |

## 4. 今後のアクション
- `logos` × `chumsky` の組み合わせで PoC を作成し、`compiler/ocaml/tests/parser_driver.ml` 相当のケースを入力して AST/診断の差分を観測する。
- Packrat キャッシュ層は既存評価でライブラリが見つからないため、自前実装（`streaming::StreamingState`）を前提に設計を進める。
- 上記 PoC 結果は `docs/notes/frontend-parser-poc.md`（仮称）として W2 の途中レビューまでに共有する。
