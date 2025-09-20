# ランタイム連携ガイド（Draft）

> 目的：FFI・ホットリロード・差分適用など実行基盤との橋渡しを行う際の指針を示す。

## 1. FFI 境界の設計

| 対象 | 推奨効果 | 安全対策 |
| --- | --- | --- |
| クラウド API / REST | `network`, `audit` | 署名・リトライ・`audit_id` で追跡 |
| データベース | `db`, `audit` | トランザクション境界を型で明示、ロールバックログを出力 |
| GPU / アクセラレータ | `gpu`, `runtime` | `unsafe` 内でハンドル管理、`defer` で解放 |
| 組み込み I/O | `runtime` | レジスタアクセスを DSL 化、割込み制御のチェックリスト |

- `unsafe` ブロックではリソース管理 (`defer`) と `audit` ログを必須とする。
- 効果タグの組み合わせは `1-3-effects-safety.md` の表を参照。

## 2. ホットリロード

```kestrel
fn reload<T>(parser: Parser<T>, state: ReloadState<T>, diff: SchemaDiff<Old, New>)
  -> Result<ReloadState<T>, ReloadError>
```

| ステップ | 説明 |
| --- | --- |
| 1 | `diff` を検証 (`Config.compare`) し、危険な変更を弾く |
| 2 | `applyDiff` で新しいパーサ/設定を構築 |
| 3 | `audit.log("parser.reload", diff)` を出力 |
| 4 | 失敗時は `RollbackInfo` を返却し、`kestrel-run reload --rollback` で復旧 |

## 3. 差分適用ワークフロー

1. `schema`（2-7）で定義された設定に対し `Config.compare` を実行。
2. 差分 (`change_set`) を `kestrel-config diff old new` で可視化し、レビューを経て `Config.apply_diff` を実行。
3. `audit_id` を発行し、`guides/config-cli.md` に記載された CLI でログを残す。
4. ランタイム側は `reload` API で新設定を適用、監査ログと照合する。

## 4. CLI 統合

| コマンド | 目的 | 代表オプション |
| --- | --- | --- |
| `kestrel-run lint <file>` | 構文/設定検証 | `--format json`, `--domain config`, `--fail-on-warning` |
| `kestrel-run diff <old> <new>` | スキーマ差分 | `--format table`, `--apply`, `--audit` |
| `kestrel-run reload <state> <diff>` | ランタイム更新 | `--dry-run`, `--rollback`, `--audit` |

```bash
kestrel-run reload runtime.state diff.json --audit   | jq '.result | {status, audit_id}'
```

## 5. 監査ログ出力

- 構造化ログ例：`{"event":"kestrel.reload", "audit_id":..., "change_set":...}`。
- CLI と LSP/IDE の診断が同じ `audit_id` を共有することで、エラー追跡と承認フローを一体化できる。

## 6. チェックリストとメトリクス

| 項目 | 内容 | 備考 |
| --- | --- | --- |
| GPU チェック | メモリ割当/解放のペア、カーネル境界での `unsafe` 区切り、`audit_id` を記録 | GPU 温度・エラーイベントを構造化ログに追加 |
| 組み込みチェック | レジスタマップと DSL の整合性、割込みマスクの設定確認、フェイルセーフ手順 | `Config.compare` と `SchemaDiff` を使って差分を検証 |
| ロールバック | `RollbackInfo` を保存し、`kestrel-run reload --rollback` で復旧する | 監査ログにロールバック結果 (`status`, `audit_id`) を記録 |
| メトリクス統合 | 遅延 (`latency_ms`), エラー率 (`error_rate`), スループットなどを構造化ログに出力 | 監視ツール（Prometheus等）と連携し SLA を監視 |

> 詳細はフェーズ3でさらに事例を追加予定です。
