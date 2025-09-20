# ランタイム連携ガイド（Draft）

> 目的：FFI・ホットリロード・差分適用など実行基盤との橋渡しを行う際の指針を示す。

## 1. FFI 境界の設計

- 効果タグ `runtime`, `network`, `gpu`, `config`, `audit` を組み合わせ、関数ごとの副作用を明示する。
- `unsafe` ブロック内ではリソース解放を `defer` で保証し、`audit` ログへ操作履歴を記録する。
- クラウド API は `network` 効果を要求し、署名・リトライ戦略を `audit_id` と共に記録する。

## 2. ホットリロード

```kestrel
fn reload<T>(parser: Parser<T>, state: ReloadState<T>, diff: SchemaDiff<Old, New>)
  -> Result<ReloadState<T>, ReloadError>
```

- 差分適用後に `audit.log("parser.reload", diff)` を呼び出す。失敗時は `rollback` 情報を返却。
- `runtime` 効果を含む関数のみホットリロード対象にする。

## 3. 差分適用ワークフロー

1. `schema`（2-7）で定義された設定に対し `Config.compare` を実行。
2. 差分 (`change_set`) を `kestrel-run diff old new` で可視化し、レビューを経て `Config.apply_diff`。
3. 適用結果を `audit_id` とともに永続化し、`undo` 操作のための履歴を保持。

## 4. CLI 統合

| コマンド | 目的 | 代表オプション |
| --- | --- | --- |
| `kestrel-run lint <file>` | 構文/設定検証 | `--format json`, `--domain config`, `--fail-on-warning` |
| `kestrel-run diff <old> <new>` | スキーマ差分 | `--format table`, `--apply`, `--audit` |
| `kestrel-run reload <state> <diff>` | ランタイム更新 | `--dry-run`, `--audit` |

## 5. 監査ログ出力

- 構造化ログ例：`{"event":"kestrel.reload", "audit_id":..., "change_set":...}`。
- CLI と IDE/LSP で共通の `audit_id` を活用し、エラー追跡と承認フローを一体化する。

> 詳細はフェーズ3で加筆予定。
