# Reml CLI ワークフローガイド

**対象**: Phase 1-6 `remlc-ocaml` CLI  
**最終更新**: 2025-10-10

本ガイドは OCaml 実装版コンパイラ `remlc-ocaml` を用いた日常開発フローを整理し、診断・トレース機能を活用したデバッグ手順をまとめる。仕様の背景は [docs/plans/bootstrap-roadmap/1-6-developer-experience.md](../plans/bootstrap-roadmap/1-6-developer-experience.md) を参照。

---

## 1. 基本的な使い方

### 1.1 実行コマンド

`remlc`（Phase 1-6 OCaml ブートストラップ版、計画書では `remlc-ocaml` と記載）は `dune exec` 経由で利用する。

```bash
opam exec -- dune exec -- remlc <入力ファイル.reml> [オプション]
```

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
  詳細は [docs/guides/diagnostic-format.md](diagnostic-format.md) を参照。

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

`--stats` でトークン数・AST ノード数・unify 呼び出しなどを収集できる。詳細は [docs/guides/trace-output.md](trace-output.md) を参照。

### 3.4 メトリクスファイル出力（Phase 1-6 Week 16 追加）

`--metrics <path>` オプションを使用すると、統計情報をファイルに出力できます。
出力形式は `--metrics-format` で `json`（デフォルト）または `csv` を選択できます。

```bash
# JSON形式で出力（CI連携用）
opam exec -- dune exec -- remlc examples/cli/trace_sample.reml --metrics metrics.json

# CSV形式で出力（表計算ソフトで分析）
opam exec -- dune exec -- remlc examples/cli/trace_sample.reml --metrics metrics.csv --metrics-format csv
```

JSON出力はスキーマ定義 [`docs/schemas/remlc-metrics.schema.json`](../schemas/remlc-metrics.schema.json) に準拠します。
これにより、CI パイプラインでの性能回帰検出やメトリクスダッシュボードへの統合が容易になります。

---

## 4. 推奨ワークフロー

1. **パース確認**: `--emit-ast` で糖衣構文の展開結果を確認。
2. **型検査**: `--emit-tast` + `--format=json` で診断ログを CI に取り込む。
3. **IR 検証**: `--emit-ir --verify-ir` を組み合わせて LLVM 検証を自動化。
4. **リンク**: `--link-runtime` で実行可能バイナリを生成 (`llvm-objdump` で検証)。
5. **トレース/統計**: パフォーマンス回帰が疑われる場合は `--trace --stats` を併用し、`docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` に測定値を記録する。

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

---

## 6. トラブルシューティング

| 症状 | 確認ポイント |
| --- | --- |
| `Error: no input file` | コマンド末尾にソースファイルを指定しているか確認。`--help` で使用例を参照。 |
| LLVM 検証失敗 | `--verify-ir` の出力を確認し、`compiler/ocaml/tests/llvm-ir/golden/` との差分を確認。 |
| ランタイムリンク失敗 | `--runtime-path` が正しいか、`runtime/native/build/libreml_runtime.a` が最新か確認。 |
| カラーが表示されない | `--color=always` を指定するか、端末が TTY として認識されているか確認。 |

---

## 7. 参考資料

- [docs/plans/bootstrap-roadmap/1-6-developer-experience.md](../plans/bootstrap-roadmap/1-6-developer-experience.md)
- [docs/guides/trace-output.md](trace-output.md)
- [docs/guides/diagnostic-format.md](diagnostic-format.md)
- [docs/guides/llvm-integration-notes.md](llvm-integration-notes.md)
