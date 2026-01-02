# Reml CLI ワークフローガイド

**対象**: Phase 1-6 `remlc-ocaml` CLI  
**最終更新**: 2025-10-10

本ガイドは OCaml 実装版コンパイラ `remlc-ocaml` を用いた日常開発フローを整理し、診断・トレース機能を活用したデバッグ手順をまとめる。仕様の背景は [do../../plans/bootstrap-roadmap/1-6-developer-experience.md](../../plans/bootstrap-roadmap/1-6-developer-experience.md) を参照。

---

## 1. 基本的な使い方

### 1.1 実行コマンド

`remlc`（Phase 1-6 OCaml ブートストラップ版、計画書では `remlc-ocaml` と記載）は `dune exec` 経由で利用する。

```bash
opam exec -- dune exec -- remlc <入力ファイル.reml> [オプション]
```

### 1.1.1 プラグインバンドルのインストール（Rust CLI）

Core.Plugin のバンドルは Rust CLI からインストールする想定であり、以下の形式を基準とする。

```bash
reml plugin install --bundle plugins/bundle.json --policy strict
reml plugin install --bundle plugins/bundle.json --policy permissive
reml plugin install --bundle plugins/bundle.json --policy strict --output json
reml plugin verify --bundle plugins/bundle.json --policy strict
reml plugin verify --bundle plugins/bundle.json --policy permissive --output json
```

- `--bundle` は [5-7-core-parse-plugin.md](../../spec/5-7-core-parse-plugin.md) の Bundle JSON 形式に従う。
- `--policy strict` は署名必須、`permissive` は警告のみで継続する。

`--output json` の出力例:

```json
{
  "bundle_id": "bundle.demo",
  "bundle_version": "0.1.0",
  "plugins": [
    {
      "plugin_id": "plugin.demo",
      "capabilities": ["plugin.demo.audit"]
    }
  ],
  "signature_status": "verified"
}
```

JSON 出力は `do../../schemas/plugin-bundle-registration.schema.json` に準拠する。

`reml plugin verify --output json` の出力例:

```json
{
  "bundle_id": "bundle.demo",
  "bundle_version": "0.1.0",
  "signature_status": "verified",
  "bundle_hash": "sha256:demo",
  "manifest_paths": ["plugins/demo/reml.toml"]
}
```

### 1.1.2 CLI サブコマンド一覧（Rust CLI）

| コマンド | 説明 |
| --- | --- |
| `reml plugin install --bundle <path> --policy strict|permissive` | プラグインバンドルをインストールする |
| `reml plugin verify --bundle <path> --policy strict|permissive` | バンドル署名を検証する（登録は行わない） |
| `reml_capability list --format json` | Capability Registry を JSON で一覧する |

### 1.2 最小のサンプル

リポジトリには `examples/cli/` 配下に以下のベースサンプルがある。

- `add.reml`: 最小構成の算術コード。リンクと IR 生成の疎通確認に利用する。
- `emit_suite.reml`: 条件分岐と関数呼び出しを含むベースライン。`--emit-*` オプションの複合検証に使用する。
- `trace_sample.reml`: トレースと統計を確認するやや長いコード。
- `type_error.reml`: 診断出力を検証するための意図的な型エラー。

代表的な実行例:

```bash
opam exec -- dune exec -- remlc examples/cli/add.reml --emit-ir

opam exec -- dune exec -- remlc examples/cli/emit_suite.reml --emit-ast --emit-tast --emit-ir --out-dir build/emit
```

生成された `out.ll` / `out.bc` は `--out-dir` で出力先を指定できる。自分のコードを試す場合は `tmp/` 配下などにファイルを複製して編集する。

### 1.3 ランタイムとリンク

実行可能ファイルを得る場合は以下を実行する。

```bash
opam exec -- dune exec -- \
  remlc examples/cli/add.reml \
  --link-runtime \
  --out-dir build
```

`--runtime-path` でランタイム静的ライブラリを明示できる（既定値: `runtime/native/build/libreml_runtime.a`）。

---

## 2. 出力オプションと中間成果物

| オプション | 説明 | 既定値 |
| --- | --- | --- |
| `--emit-ast` | 構文解析結果（AST）を標準出力に表示 | OFF |
| `--emit-tast` | 型推論後の AST を表示 | OFF |
| `--emit-ir` | LLVM IR (`.ll`) を生成 | OFF |
| `--emit-bc` | LLVM Bitcode (`.bc`) を生成 | OFF |
| `--out-dir <dir>` | 中間成果物の出力ディレクトリ | `.` |

`--emit-*` オプションは複数同時指定が可能。出力ファイルは `--out-dir/<basename>.<ext>` に配置される。

---

## 3. 診断・トレース機能

### 3.1 診断フォーマット

- `--format=text`（既定）: ソーススニペット付きの日本語メッセージ。
- `--format=json`: CI・エディタ統合向けの JSON。  
  詳細は [../tooling/diagnostic-format.md](../tooling/diagnostic-format.md) を参照。

カラー出力は `--color=auto|always|never` で制御する。`NO_COLOR` や `FORCE_COLOR` 環境変数も考慮される。

```bash
opam exec -- dune exec -- remlc examples/cli/type_error.reml --format=json
```

上記コマンドで JSON 診断を確認できる（`type_error.reml` は `if` 条件に整数を渡す誤りを含む）。

### 3.2 フェーズトレース

`--trace` を有効化するとパーサーから LLVM 生成までの各フェーズが標準エラーに記録される。

```bash
opam exec -- dune exec -- remlc examples/cli/trace_sample.reml --trace
```

出力例:

```
[TRACE] Parsing started
[TRACE] Parsing completed (0.012s, 640 bytes allocated)
[TRACE] TypeChecking started
…
[TRACE] Total: 0.061s (2112 bytes allocated)
```

### 3.3 コンパイル統計

`--stats` でトークン数・AST ノード数・unify 呼び出しなどを収集できる。詳細は [../tooling/trace-output.md](../tooling/trace-output.md) を参照。

### 3.4 メトリクスファイル出力（Phase 1-6 Week 16 追加）

`--metrics <path>` オプションを使用すると、統計情報をファイルに出力できます。
出力形式は `--metrics-format` で `json`（デフォルト）または `csv` を選択できます。

```bash
# JSON形式で出力（CI連携用）
opam exec -- dune exec -- remlc examples/cli/trace_sample.reml --metrics metrics.json

# CSV形式で出力（表計算ソフトで分析）
opam exec -- dune exec -- remlc examples/cli/trace_sample.reml --metrics metrics.csv --metrics-format csv
```

JSON出力はスキーマ定義 [`do../../schemas/remlc-metrics.schema.json`](../../schemas/remlc-metrics.schema.json) に準拠します。
これにより、CI パイプラインでの性能回帰検出やメトリクスダッシュボードへの統合が容易になります。

---

## 4. 推奨ワークフロー

1. **パース確認**: `--emit-ast` で糖衣構文の展開結果を確認。
2. **型検査**: `--emit-tast` + `--format=json` で診断ログを CI に取り込む。
3. **IR 検証**: `--emit-ir --verify-ir` を組み合わせて LLVM 検証を自動化。
4. **リンク**: `--link-runtime` で実行可能バイナリを生成 (`llvm-objdump` で検証)。
5. **トレース/統計**: パフォーマンス回帰が疑われる場合は `--trace --stats` を併用し、`do../../plans/bootstrap-roadmap/0-3-audit-and-metrics.md` に測定値を記録する。

---

## 5. CI/CD への組み込み

- **GitHub Actions** 例

```yaml
    - name: Compile sample with metrics
      run: |
        opam exec -- dune exec -- remlc examples/cli/trace_sample.reml \
          --trace --stats --metrics metrics.json 2>trace.log

    - name: Archive metrics and trace
      uses: actions/upload-artifact@v4
      with:
        name: remlc-metrics
        path: |
          metrics.json
          trace.log
```

- **JSON 診断収集**: `--format=json` を指定して標準エラーを収集し、LSP 互換のパイプラインに連携する。
- **スモークテスト**: `--emit-ir --verify-ir` を主要サンプルに対して実行し、リンク工程は必要に応じて nightly ジョブに移す。
- **性能回帰検出**: `--metrics` で出力したJSONをCI上で過去の実行結果と比較し、parse_throughputやmemory_peak_ratioの異常値を検出する。

### 5.4 監査ログの永続化と圧縮履歴

- `--emit-audit --audit-store=<profile>` を指定すると CLI が `reports/audit/<target>/<YYYY>/<MM>/<DD>/<build-id>.jsonl` を生成し、`reports/audit/index.json`・`summary.md` を更新する。`profile=local` は `tooling/audit-store/local/<timestamp>/audit.jsonl` に書き出す。
- CI プロファイルでは最新 20 件の監査ログを `reports/audit/history/<target>.jsonl.gz` として圧縮し、失敗時は `reports/audit/failed/<build-id>/` に `audit.jsonl` と `entry.json` を退避する。`history/` 生成や復元には `camlzip` が必要なため、`opam install . --deps-only --with-test` を実行して依存を揃えてから CLI をビルドする。
- 監査成果物は `do../../plans/bootstrap-roadmap/0-3-audit-and-metrics.md` の手順に従ってレビュー時に確認する。`reports/audit/index.json` が更新された PR では `history/*.jsonl.gz` の更新と `failed/` ディレクトリの有無も併せて確認する。

---

## 6. 効果構文 PoC の有効化

Phase 2-7 では代数的効果構文（`perform` / `handle`）が PoC ステージにあり、明示的なフラグを指定したビルドのみ利用できる。

- **CLI**: `--experimental-effects` または互換名 `-Zalgebraic-effects` を付与すると `RunConfig.experimental_effects=true` が設定され、Parser/Typer が効果構文を受理する。テストやゴールデンを再生成する際は必ずフラグを指定する。  
  例:  
  ```bash
  opam exec -- dune exec -- \
    remlc examples/effects/perform_basic.reml \
    --experimental-effects \
    --format=json >tmp/effect.json
  ```
- **LSP/CI**: `tooling/lsp/config/*.json` や CI の補助スクリプトでは `experimentalEffects`（camelCase）キーに `true` を設定する。`tooling/lsp/run_config_loader` が CLI と同じ RunConfig 設定を生成するため、LSP セッションからも PoC を再現できる。
- **監査ログ**: 効果フラグを有効化した状態で CLI を実行すると、`extensions.effects.experimental` や `audit.metadata.effect.syntax.constructs.*` が出力され、`collect-iterator-audit-metrics.py --section effects --require-success` が PoC KPI を検証する。

脚注 `[^effects-syntax-poc-phase25]` が撤去されるまでは、本番ビルドでフラグを既定有効にせず、PoC を必要とするタスクのみ opt-in 運用とする。フラグ名の変更が決定された場合は CLI/LSP/CI/ドキュメントの全経路を同時更新し、`do../../notes/effects/effect-system-tracking.md` の H-O3 チェックリストを参照して整合性を確認する。

---

## 7. トラブルシューティング

| 症状 | 確認ポイント |
| --- | --- |
| `Error: no input file` | コマンド末尾にソースファイルを指定しているか確認。`--help` で使用例を参照。 |
| LLVM 検証失敗 | `--verify-ir` の出力を確認し、`compiler/ocaml/tests/llvm-ir/golden/` との差分を確認。 |
| ランタイムリンク失敗 | `--runtime-path` が正しいか、`runtime/native/build/libreml_runtime.a` が最新か確認。 |
| カラーが表示されない | `--color=always` を指定するか、端末が TTY として認識されているか確認。 |

---

## 8. 参考資料

- [do../../plans/bootstrap-roadmap/1-6-developer-experience.md](../../plans/bootstrap-roadmap/1-6-developer-experience.md)
- [../tooling/trace-output.md](../tooling/trace-output.md)
- [../tooling/diagnostic-format.md](../tooling/diagnostic-format.md)
- [../compiler/llvm-integration-notes.md](../compiler/llvm-integration-notes.md)
