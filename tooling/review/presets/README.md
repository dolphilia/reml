# Audit Query Presets

このディレクトリは `tooling/review/audit-query.py` が利用する DSL プリセットを集約する。

各 `.dsl` ファイルには 1 行のクエリ式を記述し、`audit-query --query-file <preset.dsl>` で読み込める。

| ファイル | 用途 |
|----------|------|
| `stage-regressions.dsl` | Capability Stage や iterator 監査のリグレッション検出 |
| `ffi-regressions.dsl` | FFI ブリッジ関連診断の抽出 |
| `typeclass-metadata.dsl` | 型クラスメタデータ欠損・不整合の抽出 |
