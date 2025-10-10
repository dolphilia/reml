# Reml CLI ヘルプ同期テンプレート

**目的**: `remlc-ocaml` CLI の `--help` 出力、man ページ、ガイド文書を同一ソースで管理し、Phase 1-6 で導入したオプション体系の一貫性を保つ。  
**関連計画書**: [docs/plans/bootstrap-roadmap/1-6-developer-experience.md](../plans/bootstrap-roadmap/1-6-developer-experience.md) §6.

---

## 1. 運用ポリシー
- 一次ソースは `compiler/ocaml/src/cli/options.ml` の `print_full_help`。この関数を更新したら本書とテンプレートを同じコミットで更新する。
- man ページは Markdown テンプレート（`docs/guides/man/remlc-ocaml.1.md`）から生成し、リリース時に `pandoc` もしくは `ronn` で変換する。直接 `.1` を編集しない。
- ガイド文書（`docs/guides/cli-workflow.md`）およびサンプル README（`examples/cli/README.md`）はヘルプ内容から派生する情報として同期する。

## 2. 更新手順チェックリスト
1. `options.ml` のオプション説明・例を修正する。
2. 本テンプレート内の表と脚注を更新する。
3. `docs/guides/man/remlc-ocaml.1.md` を更新し、`tooling/cli/scripts/update-man-pages.sh` を実行して `tooling/cli/man/remlc-ocaml.1` を再生成する（`--check` オプションで差分確認）。
4. 関連ガイド（`cli-workflow.md` など）の該当箇所を反映する。
5. `docs/plans/bootstrap-roadmap/1-6-developer-experience.md` の進捗欄と TODO を更新する。
6. 変更内容を `docs/plans/bootstrap-roadmap/1-6-developer-experience.md` §「Week 16 ヘルプ・ドキュメント整備」に記録する。

> **補足**: `pandoc` を利用する場合は `brew install pandoc`（macOS）や `apt install pandoc`（Linux）で導入できる。CI への追加は Phase 1-7 で検討する。

## 3. オプション一覧（同期用リファレンス）

| 区分 | オプション | 説明 | 既定値 / 補足 |
| --- | --- | --- | --- |
| 入力 | `<file>` | コンパイル対象の Reml ソースファイル | 必須 |
| 出力 | `--emit-ast` | 構文解析結果（AST）を標準出力に表示 | OFF |
|  | `--emit-tast` | 型付け済み AST（Typed AST）を標準出力に表示 | OFF |
|  | `--emit-ir` | LLVM IR (`.ll`) を出力ディレクトリへ生成 | OFF |
|  | `--emit-bc` | LLVM Bitcode (`.bc`) を出力ディレクトリへ生成 | OFF |
|  | `--out-dir <dir>` | 中間成果物の出力先ディレクトリ | `.` |
| 診断 | `--format <text|json>` | 診断メッセージの出力形式 | `text` |
|  | `--color <auto|always|never>` | カラー表示のモード切替 | `auto`（`NO_COLOR` で強制 OFF） |
| トレース | `--trace` | コンパイルフェーズのトレースを標準エラーに出力 | OFF |
|  | `--stats` | コンパイル統計情報（トークン数等）を標準エラーに出力 | OFF |
|  | `--verbose <0-3>` | ログ詳細度（`REMLC_LOG` 環境変数も利用可） | `1` |
| コンパイル | `--target <triple>` | ターゲットトリプル | `x86_64-linux` |
|  | `--link-runtime` | ランタイムライブラリとリンクして実行可能ファイルを生成 | OFF |
|  | `--runtime-path <path>` | ランタイム静的ライブラリのパス | `runtime/native/build/libreml_runtime.a` |
|  | `--verify-ir` | 生成した LLVM IR を `llvm-verifier` 相当で検証 | OFF |
| ヘルプ | `--help`, `-help` | セクション化された詳細ヘルプを表示 | - |

## 4. man ページテンプレート
- テンプレート本文: [`docs/guides/man/remlc-ocaml.1.md`](man/remlc-ocaml.1.md)
- 生成スクリプト: `tooling/cli/scripts/update-man-pages.sh`（`--check` でテンプレートとの差分を確認可能）
- `pandoc` を直接利用する場合のコマンド例:

```bash
pandoc \
  --from markdown \
  --to man \
  --output tooling/cli/man/remlc-ocaml.1 \
  docs/guides/man/remlc-ocaml.1.md
```

生成物 `tooling/cli/man/remlc-ocaml.1` をインストール手順で `man1/` へ配置し、`CMAKE_INSTALL_MANDIR` 相当のディレクトリへインストールする（CI への組み込みは Phase 1-7 で実施予定）。

## 5. 同期確認ポイント
- `print_full_help` と man テンプレートの文言・区切り順が一致しているか。
- サンプルコマンドが `examples/cli/README.md` のチェックリストと同じか。
- 環境変数の説明が `docs/guides/config-cli.md` と矛盾していないか。
- 参考リンクで `docs/guides/cli-workflow.md` と `docs/guides/trace-output.md` を案内しているか。

## 6. TODO / 追加検討
- [x] `tooling/cli/scripts/update-man-pages.sh` を追加してテンプレートとの同期手順を確立する（Phase 1-6 完了）。
- [ ] Phase 1-7 で CI に man ページ生成スクリプトを組み込み、整合性チェックを自動化する。
- [ ] `pandoc` に依存しない最小ジェネレーター（OCaml スクリプト）の検討。
- [ ] `docs/guides/diagnostic-format.md` の JSON スキーマ確定後に man ページへリンクする。

---

## 参考資料
- `compiler/ocaml/src/cli/options.ml`
- `docs/plans/bootstrap-roadmap/1-6-developer-experience.md`
- `docs/guides/cli-workflow.md`
- `docs/guides/trace-output.md`
