# 2-4 レビュー支援ツール設計メモ

> 作成日: 2025-10-29  
> 対象タスク: Phase 2-4 §5 「レビュー支援ツール」  
> 関連文書: [2-4 診断・監査パイプライン強化計画](2-4-diagnostics-audit-pipeline.md#5-レビュー支援ツール)

本メモは `tooling/review/` 配下に導入する差分・ダッシュボード・クエリ各ツールの共通設計を定義する。特に `audit_shared.py` と `audit-diff.py` の正規化データ構造と CLI インターフェースを確定し、CI 連携 (`collect-iterator-audit-metrics.py`) の実装方針を記録する。

## 1. 正規化データ構造

### 1.1 `NormalizedAuditEntry`

```python
@dataclass
class NormalizedAuditEntry:
    category: str
    code: Optional[str]
    severity: Optional[str]
    timestamp: datetime
    audit_id: Optional[str]
    cli_audit_id: Optional[str]
    change_set: Optional[str]
    pass_rate: Optional[float]
    metadata: Dict[str, Any]
    extensions: Dict[str, Any]
    source: Path
    raw: Dict[str, Any]
```

- `category`: 監査ログ (`metadata["category"]` または `entry["category"]`) を正規化した識別子。`ffi.bridge.callconv` 等のプレフィックスで揃える。
- `code`: `Diagnostic.code` または `metadata["code"]` を格納し、診断差分と監査差分の双方でキーとする。
- `pass_rate`: `metadata["bridge.audit_pass_rate"]` 等の数値フィールドを `float` に変換して保持。差分・ダッシュボードで共通利用。
- `metadata`, `extensions`: 元の辞書をシリアライズ順に `dict` で保持し、差分抽出時に `json.dumps(..., sort_keys=True)` を利用。
- `raw`: 入力の辞書をそのまま保持し、出力再構築やデバッグに用いる。

### 1.2 正規化ヘルパ

`tooling/review/audit_shared.py` に以下のユーティリティを提供する：

- `load_entries(path: Path) -> List[NormalizedAuditEntry]`: JSON / JSONL を自動判別し、上記構造体に変換。
- `index_by_category(entries: Iterable[NormalizedAuditEntry]) -> Dict[str, List[NormalizedAuditEntry]]`
- `flatten_metadata(entry: NormalizedAuditEntry, prefix: str = "") -> Dict[str, str]`: `metadata` と `extensions` をフラット化し、差分・クエリ両方で利用。
- `load_diff_manifest(path: Path) -> AuditDiffSummary`: `diff.json`（§2.3 参照）の読み書き。
- `CoverageReport` / `DashboardManifest` の軽量データクラスを併設し、CI 集計 (`collect-iterator-audit-metrics.py --section review`) で再利用する。

## 2. `audit-diff.py` CLI

### 2.1 コマンドラインインターフェース

```
usage: audit-diff.py --base BASE --target TARGET [--format {md,html,json}] \
                     [--output DIR] [--query FILE] [--threshold FLOAT]
```

| オプション | 説明 |
|-----------|------|
| `--base` | 比較対象（従来ログ）の JSON/JSONL パス。 |
| `--target` | 比較対象（新規ログ）の JSON/JSONL パス。 |
| `--format` | 出力フォーマット。複数指定時はカンマ区切り (`md,html,json`)。既定は `md,json`。 |
| `--output` | 出力ディレクトリ。省略時は `reports/audit/review/<commit>/` に自動配置。 |
| `--query` | DSL ファイル（`audit_query.dsl`）を差分前処理に適用し、対象ログを絞り込む。 |
| `--threshold` | `diagnostic.regressions` や `pass_rate.delta` の警告閾値を上書き。0.0 未満で警告抑止。 |

### 2.2 出力成果物

| ファイル | 内容 |
|----------|------|
| `diff.json` | 差分サマリ。`collect-iterator-audit-metrics.py` が読み取る。 |
| `diff.md` | Markdown レポート。レビューコメントで参照。 |
| `diff.html` | HTML レポート。CI アーティファクト化し、詳細閲覧に使用。 |

#### 2.2.1 `diff.json` スキーマ（v1）

```json5
{
  "schema_version": "audit-diff.v1",
  "base": {"path": "...", "entry_count": 120},
  "target": {"path": "...", "entry_count": 118},
  "diagnostic": {
    "regressions": 2,
    "new": 1,
    "improved": 0,
    "details": [...]
  },
  "metadata": {
    "changed": 3,
    "added_keys": ["bridge.return.wrap"],
    "removed_keys": []
  },
  "pass_rate": {
    "previous": 1.0,
    "current": 0.95,
    "delta": -0.05
  },
  "failures": [
    {"category": "ffi.bridge", "code": "ffi.contract.abi", "reason": "..."}
  ],
  "generated_at": "2025-10-29T08:15:00Z"
}
```

`diagnostic.regressions` と `metadata.changed` の和を `audit_diff.regressions` に使用する。`failures` は差分処理で例外や未対応パターンがあった場合に記録する。

## 3. DSL クエリ連携 (`audit-query`)

- DSL 文法: `expr := expr OR expr | expr AND expr | NOT expr | (expr) | predicate`
- `predicate := IDENT comparator value` (`comparator := == | != | in | contains | =~`)
- `IDENT` は `metadata.bridge.platform` のようなドット表記を許可。
- `value` は文字列・数値・配列リテラル。`in` の右辺は配列、`contains` は部分文字列。
- CLI インターフェース（予定）:

```
audit-query --from audits.jsonl \
            --query 'metadata.bridge.platform == "windows-msvc" and severity == "Error"' \
            --format table
```

プリセットを利用する場合は `audit-query --from audits.jsonl --query-file tooling/review/presets/ffi-regressions.dsl` を実行し、共通 DSL を共有する。

`audit-diff.py --query` は内部で `audit-query` の API (`filter_entries(entries, query_ast)`) を呼び出す。

> **実装メモ (2025-10-29)**: プロトタイプ段階では `and`/`or` のみをサポートし、括弧・否定は未実装。必要に応じて `audit_query` のパーサを拡張する。

## 4. CI 連携仕様

### 4.1 `collect-iterator-audit-metrics.py` Review セクション

| フィールド | 説明 |
|-----------|------|
| `audit_review.metric` | `audit_review.summary` 固定。 |
| `audit_review.audit_diff.regressions` | `diff.json` の `diagnostic.regressions + metadata.changed` 合計。 |
| `audit_review.audit_diff.pass_rate.delta` | `pass_rate.delta`。 |
| `audit_review.audit_diff.sources` | base/target のファイルパス。 |
| `audit_review.audit_query.coverage` | DSL プリセット集計 (`matched / total`)。欠損時は `null`。 |
| `audit_review.audit_dashboard.generated` | `reports/audit/dashboard/index.html` 等が生成された回数。 |
| `audit_review.failures[]` | 欠損ファイルや解析失敗の詳細。 |

CI は `audit_diff.regressions > 0` または `audit_query.coverage < 0.8` の場合に警告を出し、`pass_rate.delta < -0.05` で失敗扱いにする。閾値は `audit-diff.py --threshold` で調整可能。

### 4.2 成果物配置規約

- 差分レポート: `reports/audit/review/<commit>/diff.{json,md,html}`
- ダッシュボード: `reports/audit/dashboard/index.{html,md}`（CI ではアーティファクトのみ）
- DSL クエリ結果: `reports/audit/review/<commit>/query/<preset>.json`

`collect-iterator-audit-metrics.py` は上記既定パスを自動探索し、CLI オプションで明示した場合はそちらを優先する。

## 5. フォローアップ

- `tooling/json-schema/audit-diff.schema.json` を新設し、`diff.json` のバリデーションを CI に組み込む。
- `tooling/review/presets/` に DSL プリセット (`stage-regressions.dsl`, `ffi-regressions.dsl`, `typeclass-metadata.dsl`) を追加し、`audit-query` の単体テストに使用する。
- `reports/diagnostic-format-regression.md` のチェックリストに `audit-diff.py` / `audit-dashboard.py` / `audit-query` チェックを追加済み（2025-10-29 更新）。
- 実装着手時は `tooling/review/README.md` を作成し、利用手順・CI 連携・プリセット一覧をまとめる。

---

本設計は Phase 2-4 の開発チーム向けレビュー用ドラフトであり、実装進捗に合わせて更新する。仕様変更が生じた場合は本書と `docs/spec/3-6-core-diagnostics-audit.md` 付録を同期すること。
