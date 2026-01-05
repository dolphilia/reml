# 第4章 調査メモ: 字句解析 (Lexical Analysis)

## 参照した資料
- `compiler/frontend/src/lexer/mod.rs:1-1018`（字句解析の実装、RawToken 定義、lex_source など）
- `compiler/frontend/src/token.rs:1-237`（TokenKind / Token / LiteralMetadata）
- `compiler/frontend/src/span.rs:1-61`（Span）
- `compiler/frontend/src/unicode.rs:1-124`（UnicodeDetail と診断コード）
- `compiler/runtime/src/text/identifier.rs:1-95`（identifier 正規化と bidi 制御）
- `compiler/frontend/src/error.rs:1-59`（FrontendError / Recoverability）
- `compiler/frontend/src/bin/reml_frontend.rs:628-645`（CLI での lex オプション適用）
- `compiler/frontend/src/parser/mod.rs:344-368`（パーサでの lex 呼び出しと診断変換）
- `docs/spec/2-3-lexer.md`（字句レイヤ仕様）
- `docs/spec/1-4-test-unicode-model.md`（Unicode/Span 仕様）

## 調査メモ

### 入口と責務
- `lex_source_with_options` が字句解析の本体。`consume_skippable` で空白/コメントを先に除去し、`logos` で 1 トークンずつ読み進める。(`compiler/frontend/src/lexer/mod.rs:472-803`)
- `LexOutput` は `tokens` と `errors` を同時に返す設計で、以降のパーサで診断に昇格される。(`compiler/frontend/src/lexer/mod.rs:318-322`, `compiler/frontend/src/parser/mod.rs:361-368`)
- `Lexer` 構造体は `SourceBuffer` を受け取って `lex_source` を呼ぶ薄いラッパー。(`compiler/frontend/src/lexer/mod.rs:1001-1018`)

### トークン定義と RawToken
- `TokenKind` はキーワード/記号/リテラルを網羅し、`Token` に `Span` と `lexeme`/`LiteralMetadata` を付与する。(`compiler/frontend/src/token.rs:35-237`)
- `RawToken` は `logos` 生成用の内部列挙で、キーワードや記号、リテラルの正規表現を定義。(`compiler/frontend/src/lexer/mod.rs:83-315`)
- `RawToken` に `Skip` と `BlockComment` がある一方で、本体は `consume_skippable` で空白/コメントを手動で読み飛ばす。ネストコメントもここで処理する。(`compiler/frontend/src/lexer/mod.rs:324-383`, `compiler/frontend/src/lexer/mod.rs:477-483`)

### リテラルと識別子
- 数値: `FloatLiteral`/`IntLiteral` の正規表現を `logos` に定義し、整数は `detect_int_base` で基数を付与して `LiteralMetadata::Int` に落とす。(`compiler/frontend/src/lexer/mod.rs:292-299`, `compiler/frontend/src/lexer/mod.rs:676-685`, `compiler/frontend/src/lexer/mod.rs:881-892`)
- 文字列/文字: `lex_string_literal` / `lex_raw_string` / `lex_multiline_string` / `lex_char_literal` が閉じクォートを探索し、`lexeme` はソースの範囲からそのまま切り出す（アンエスケープは後段想定）。(`compiler/frontend/src/lexer/mod.rs:386-463`, `compiler/frontend/src/lexer/mod.rs:694-723`, `compiler/frontend/src/lexer/mod.rs:806-808`)
- 識別子: `IdentifierProfile`（Unicode/AsciiCompat）で ASCII 互換の制限を切り替える。`prepare_identifier_token` が `reml_runtime::text` の `prepare_identifier` を使って NFC/Bidi を検証する。(`compiler/frontend/src/lexer/mod.rs:33-81`, `compiler/frontend/src/lexer/mod.rs:648-675`, `compiler/frontend/src/lexer/mod.rs:810-875`, `compiler/runtime/src/text/identifier.rs:14-55`)

### エラーと診断
- 未知トークンは `FrontendErrorKind::UnknownToken` を生成し、トークン列には `TokenKind::Unknown` を混ぜる。(`compiler/frontend/src/lexer/mod.rs:789-795`, `compiler/frontend/src/error.rs:15-58`)
- Unicode 識別子エラーは `UnicodeDetail` に変換し、`UnexpectedStructure` として回復可能エラーで返す。(`compiler/frontend/src/lexer/mod.rs:831-875`, `compiler/frontend/src/unicode.rs:1-124`)
- ASCII 互換モードでは非 ASCII が含まれると `push_ascii_error` が `Unknown` トークンとエラーを追加する。(`compiler/frontend/src/lexer/mod.rs:650-653`, `compiler/frontend/src/lexer/mod.rs:894-928`)

### 他モジュールとの接続
- CLI の `--emit-tokens` では `run_config` 由来の `lex_identifier_profile/locale` を `LexerOptions` に入れて字句解析を実行。(`compiler/frontend/src/bin/reml_frontend.rs:638-644`)
- パーサは `LexerOptions` を構成し、`LexOutput.errors` を診断に変換していく。(`compiler/frontend/src/parser/mod.rs:344-368`)

### 仕様との照合メモ
- 対応 spec: `docs/spec/2-3-lexer.md`（字句レイヤ全体）、`docs/spec/1-4-test-unicode-model.md`（Unicode/Span）。
- `Span` はバイトオフセットのみ。spec 側は `line/column`（グラフェム）も含む定義なので、実装との差分がある。(`compiler/frontend/src/span.rs:6-33`, `docs/spec/1-4-test-unicode-model.md`)
- `TokenKind` に `KeywordOperation` / `KeywordPattern` / `Comment` / `Whitespace` があるが、`RawToken` では生成されていない（未実装か予約）。(`compiler/frontend/src/token.rs:69-153`, `compiler/frontend/src/lexer/mod.rs:83-315`)
- 識別子正規表現は `XID_Start/Continue` に加えて絵文字・ZWJ・Bidi 制御を許容しており、`prepare_identifier` が NFC/Bidi を検証するのみなので、仕様上の識別子制約と完全一致しているか要確認。(`compiler/frontend/src/lexer/mod.rs:310-315`, `compiler/runtime/src/text/identifier.rs:24-45`, `docs/spec/2-3-lexer.md`, `docs/spec/1-4-test-unicode-model.md`)
- 改行の LF 正規化は spec で言及があるが、字句解析コード内に明示的な変換は見当たらない（入力は UTF-8 文字列前提）。(`compiler/frontend/src/lexer/mod.rs`, `docs/spec/1-4-test-unicode-model.md`)

### 未確認事項 / TODO
- `KeywordOperation` / `KeywordPattern` が lexer で無視されているのが意図的かを確認する（parser 側の期待トークンと合わせて確認が必要）。
- 文字列/文字リテラルのエスケープ妥当性の検証位置（lexer 以降か）を追跡する。
