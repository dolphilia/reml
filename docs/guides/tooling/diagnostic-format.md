# 診断出力フォーマット仕様

**対象フェーズ**: Phase 1-6 開発者体験整備
**最終更新**: 2025-10-10

## 概要

Reml コンパイラ (`remlc`) は、エラー、警告、情報メッセージを含む診断情報を複数の形式で出力できます。このドキュメントでは、テキスト形式とJSON形式の診断出力仕様を説明します。

## テキスト形式

### 基本構造

診断メッセージは以下の要素で構成されます：

```
<ファイル名>:<行>:<列>: <重要度>[<コード>] (<ドメイン>): <メッセージ>
   <行番号> | <ソースコード行>
            <ポインタ>
補足: <追加情報>
```

### 例

```
/tmp/test_error.reml:2:3: エラー[E7006] (型システム): 条件式がBool型ではありません
   1 | fn main() -> i64 =
   2 |   if 42 then 1 else 0
         ^^
補足: 期待される型: Bool
補足: 実際の型:     i64
```

### カラー出力

`--color=always` オプションを使用すると、ANSI エスケープシーケンスによる色付け出力が有効になります：

- **エラー**: 赤 (ANSI 91)
- **警告**: 黄 (ANSI 93)
- **情報**: 青 (ANSI 94)
- **行番号**: シアン (ANSI 36)
- **ポインタ**: 重要度と同じ色

カラーモードは以下の優先順位で決定されます：

1. `NO_COLOR` 環境変数が設定されている場合は常に無効
2. `--color=always` の場合は常に有効
3. `--color=never` の場合は常に無効
4. `--color=auto` の場合（デフォルト）:
   - `FORCE_COLOR` が設定されている場合は有効
   - 出力先が TTY の場合は有効
   - それ以外は無効

### ソースコードスニペット

診断メッセージには、エラー位置の前後1行を含むソースコードスニペットが表示されます：

```
   1 | fn main() -> i64 =
   2 |   if x then 1 else 0
         ^^
   3 |   end
```

## JSON 形式

### 基本構造

`--format=json` オプションを使用すると、機械判読可能な JSON 形式で診断が出力されます。

### Reml 独自形式（デフォルト）

```json
{
  "diagnostics": [
    {
      "severity": "error",
      "code": "E7001",
      "domain": "型システム",
      "message": "型が一致しません",
      "location": {
        "file": "/path/to/file.reml",
        "line": 1,
        "column": 18,
        "endLine": 1,
        "endColumn": 24
      },
      "notes": [
        "期待される型: i64",
        "実際の型:     String"
      ],
      "expected": [
        "型 'i64'"
      ]
    }
  ]
}
```

### フィールド説明

- **severity**: 重要度（`"error"`, `"warning"`, `"note"`）
- **code**: エラーコード（例: `"E7001"`）
- **domain**: 診断ドメイン（`"構文解析"`, `"型システム"` など）
- **message**: 1行要約メッセージ
- **location**: エラー位置情報
  - **file**: ファイルパス
  - **line**: 開始行番号（1始まり）
  - **column**: 開始列番号（1始まり）
  - **endLine**: 終了行番号
  - **endColumn**: 終了列番号
- **notes**: 追加情報のリスト（オプション）
- **expected**: 期待される入力のリスト（オプション）
- **fixits**: 修正提案のリスト（オプション）

### LSP 互換形式

LSP（Language Server Protocol）互換の JSON 形式も将来的にサポート予定です（Phase 2）。

## コマンドラインオプション

### 診断関連オプション

| オプション | 説明 | デフォルト |
|-----------|------|-----------|
| `--format <text\|json>` | 出力形式 | `text` |
| `--color <auto\|always\|never>` | カラーモード | `auto` |

### 環境変数

| 変数名 | 説明 |
|--------|------|
| `NO_COLOR` | 設定されている場合、カラー出力を無効化 |
| `FORCE_COLOR` | 設定されている場合、カラー出力を強制的に有効化 |
| `REMLC_LOG` | ログレベル（`error`, `warn`, `info`, `debug`） |

## 使用例

### テキスト形式（デフォルト）

```bash
remlc input.reml
```

### JSON 形式

```bash
remlc input.reml --format=json
```

### カラー出力を常に有効

```bash
remlc input.reml --color=always
```

### カラー出力を無効

```bash
remlc input.reml --color=never
```

### JSON 形式でパイプライン処理

```bash
remlc input.reml --format=json 2>&1 | jq '.diagnostics[0].message'
```

## 関連仕様書

- [3-6-core-diagnostics-audit.md](../../spec/3-6-core-diagnostics-audit.md) - 診断システムの仕様
- [2-5-error.md](../../spec/2-5-error.md) - エラー設計
- [1-6-developer-experience.md](../../plans/bootstrap-roadmap/1-6-developer-experience.md) - 開発者体験整備計画

## 実装

診断フォーマット出力は以下の実装で構成されています：

- `compiler/frontend/src/bin/reml_frontend.rs` - テキスト/JSON 形式の出力組み立て

---

**作成日**: 2025-10-10
**Phase**: 1-6 開発者体験整備
