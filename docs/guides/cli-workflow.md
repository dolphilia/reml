# Reml CLI ワークフローガイド

**対象**: Phase 1-6 `remlc-ocaml` CLI  
**最終更新**: 2025-10-10

本ガイドは OCaml 実装版コンパイラ `remlc-ocaml` を用いた日常開発フローを整理し、診断・トレース機能を活用したデバッグ手順をまとめる。仕様の背景は [docs/plans/bootstrap-roadmap/1-6-developer-experience.md](../plans/bootstrap-roadmap/1-6-developer-experience.md) を参照。

---

## 1. 基本的な使い方

### 1.1 実行コマンド

`remlc` (Phase 1-6 OCaml ブートストラップ版、計画書では `remlc-ocaml` と記載) は `dune exec` 経由で利用する。

```bash
opam exec -- dune exec -- remlc <入力ファイル.reml> [オプション]
```

### 1.2 最小のサンプル

以下のコードを `tmp/add.reml` など任意のパスに保存する。

```reml
fn add(a: i64, b: i64) -> i64 = a + b
fn main() -> i64 = add(2, 40)
```

```bash
opam exec -- dune exec -- remlc tmp/add.reml --emit-ir
```

生成された `out.ll` / `out.bc` は `--out-dir` で出力先を指定できる。

### 1.3 ランタイムとリンク

実行可能ファイルを得る場合は以下を実行する。

```bash
opam exec -- dune exec -- \
  remlc tmp/add.reml \
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

### 3.2 フェーズトレース

`--trace` を有効化するとパーサーから LLVM 生成までの各フェーズが標準エラーに記録される。

```bash
opam exec -- dune exec -- remlc tmp/add.reml --trace
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

`--stats` でトークン数・AST ノード数・unify 呼び出しなどを収集できる。`Cli.Stats.to_json` を利用するとテストから JSON を取得できる。詳細は [docs/guides/trace-output.md](trace-output.md) を参照。

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
    - name: Compile sample
      run: opam exec -- dune exec -- remlc tmp/add.reml --trace --stats 2>trace.log
    - name: Archive trace
      uses: actions/upload-artifact@v4
      with:
        name: remlc-trace
        path: trace.log
```

- **JSON 診断収集**: `--format=json` を指定して標準エラーを収集し、LSP 互換のパイプラインに連携する。
- **スモークテスト**: `--emit-ir --verify-ir` を主要サンプルに対して実行し、リンク工程は必要に応じて nightly ジョブに移す。

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
