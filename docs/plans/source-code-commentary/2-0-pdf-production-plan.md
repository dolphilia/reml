# Remlソースコード完全解説: PDF制作計画

## 1. 目的

`docs/plans/source-code-commentary/drafts/` 配下の原稿を元に、商業出版レベル（O'Reilly Japan相当）の品質を持つPDFを生成する。本計画書では、使用するツールチェーン、ビルドプロセス、およびスタイリング仕様を確定させる。

## 2. ツールチェーン構成

再現性と品質を確保するため、以下のオープンソースソフトウェア群を標準ツールチェーンとして採用する。

| カテゴリ | ツール | バージョン要件 | 役割 |
| --- | --- | --- | --- |
| 変換エンジン | **Pandoc** | v3.1以上 | MarkdownからLaTeXへの変換、フィルタ処理 |
| 組版処理 | **LuaLaTeX** | TeX Live 2023以上 | 日本語禁則処理、フォント管理、PDF生成 |
| ビルド自動化 | **GNU Make** | Any | ビルドフローの定義と依存関係解決 |
| 図版生成 | **Mermaid CLI** | v10以上 (`mmdc`) | Mermaid記法からPDF/SVGへのベクター変換 |
| 相互参照 | **pandoc-crossref** | Pandocと互換 | 図表番号の自動採番と参照解決 |
| 付随ツール | **Node.js** | LTS | Mermaid CLIの実行環境 |
| フォント | **Noto CJK** | 2023以降 | 日本語本文・見出し・コードの統一 |

## 3. ディレクトリ構造と成果物

ビルド成果物はリポジトリを汚染しないよう `dist/` ディレクトリ（.gitignore対象）に出力する。

```text
docs/plans/source-code-commentary/
├── drafts/               # 原稿 (入力)
├── templates/            # テンプレート
│   └── eisvogel.latex    # カスタムLaTeXテンプレート (ベース)
├── output/               # 中間生成物
│   ├── images/           # 変換された図版 (PDF/SVG)
│   └── full_draft.md     # 結合された単一Markdown
├── dist/                 # 最終成果物
│   └── reml-commentary.pdf
└── Makefile              # ビルドスクリプト
```

`.gitignore` に `docs/plans/source-code-commentary/dist/` と `docs/plans/source-code-commentary/output/` を追記する。

## 4. ビルドプロセス (Makefile)

以下のターゲットを定義し、コマンド一つでPDFが生成できるようにする。

1. **`make setup`**: 必要なツールバージョン確認とディレクトリ作成。
2. **`make images`**: `drafts/*.md` 内の Mermaid コードブロックを検出し、`mmdc` で PDF 画像ファイルに変換する。Markdown 内のリンクをその画像ファイルへの参照に置換する（Pandoc フィルタまたは前処理スクリプトを使用）。
3. **`make concat`**: `drafts/*.md` をファイル名順（章順）に結合し、単一の `output/full_draft.md` を生成する。この際、ファイルヘッダのメタデータ調整（タイトル・著者・lang など）を行う。
4. **`make pdf`**: Pandoc を呼び出し、`full_draft.md` を PDF に変換する。
5. **`make clean`**: 生成物を削除する。

### Pandoc 実行コマンド例

```bash
pandoc output/full_draft.md \
  --pdf-engine=lualatex \
  --template=templates/eisvogel.latex \
  --filter=pandoc-crossref \
  --top-level-division=chapter \
  --toc \
  --number-sections \
  --resource-path=.:output/images \
  --highlight-style=breezedark \
  -V documentclass=bxjsbook \
  -V classoption=pandoc \
  -V classoption=jafont=noto \
  -o dist/reml-commentary.pdf
```

## 5. スタイリング仕様

### 5.1 判型とレイアウト

- **判型**: **B5変形** (182mm x 257mm) - 技術書の標準フォーマット。
- **余白**: 製本マージンを考慮し、内側（ノド）を広めに取る。
  - Inner: 25mm, Outer: 15mm, Top: 20mm, Bottom: 20mm
- **段組**: 1段組み。
  - 実装はテンプレート側の `geometry` 指定で行い、B5変形の実寸は明示的に設定する（`papersize={182mm,257mm}`）。

### 5.2 フォント設定 (LuaLaTeX + Noto)

Google Noto CJK フォントファミリーを採用し、モダンで可読性の高い紙面とする。

- **明朝体 (本文)**: `Noto Serif CJK JP`
  - ウェイト: Regular, Bold
- **ゴシック体 (見出し)**: `Noto Sans CJK JP`
  - ウェイト: Bold, Black (章タイトル用)
- **等幅フォント (コード)**: `Noto Sans Mono CJK JP`
  - リガチャ（合字）はコード解説の誤認を防ぐため**無効化**する。

### 5.3 コードハイライト

- Pandoc の `--highlight-style` オプションを使用。
- 配色テーマ: `monochrome` を採用し、印刷所向けの白黒原稿に合わせる。

### 5.4 図版のモノクロ化

- Mermaid 図版は `mmdc` のカスタム設定でモノクロ配色に固定する。
- 画像生成時に `--configFile` を指定し、線・塗り・文字色をグレースケールに統一する。

### 5.5 見出し・相互参照の運用ルール

- 図・表の参照は `pandoc-crossref` のラベル形式（例: `fig:lexer-flow`）に統一する。
- 章・節番号の自動採番を有効化し、`--number-sections` を必須とする。
- 章順は `drafts/` のファイル名で管理し、`00-` からのゼロパディングで固定する。

## 6. 今後のタスク

1. **環境構築**: Dockerコンテナ (`pandoc/latex:latest` ベース) を用意し、CI/CD での自動ビルドを可能にする。
2. **テンプレート作成**: Pandoc のデフォルトテンプレートを拡張し、奥付（裏表紙）や扉ページを追加可能にする。
3. **画像変換スクリプト**: Mermaid を抽出して `mmdc` に渡すプリプロセッサの実装（Rust または Python）。
4. **校正支援**: リンク切れ・重複見出し・図表参照不足を検知する軽量チェックを追加する。
