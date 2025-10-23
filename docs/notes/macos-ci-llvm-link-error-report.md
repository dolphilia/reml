# macOS CI LLVM リンクエラー調査レポート（2025-02-XX） {#macos-ci-llvm-link-error}

## 概要
- 対象: GitHub Actions `bootstrap-macos.yml` の `Build compiler` ステップ（ARM64 runner）
- 現象: `opam exec -- dune build` 実行時に `LLVMConstStringInContext2`・`LLVMPositionBuilderBeforeInstrAndDbgRecords` などのシンボル解決に失敗し、`clang: error: linker command failed with exit code 1` でビルドが停止
- 影響: CLI バイナリ (`src/main.exe`) と LLVM 依存テスト (`test_llvm_*`, `test_ffi_*`) が全滅し、`ffi_bridge.audit_pass_rate` が 0.0 でレポートされる

## 症状ログ（2025-02-XX Run ID T.B.A.）
```
Undefined symbols for architecture arm64:
  "_LLVMConstStringInContext2", referenced from:
      _llvm_const_string in libllvm.a[2](llvm_ocaml.o)
      _llvm_const_stringz in libllvm.a[2](llvm_ocaml.o)
  "_LLVMPositionBuilderBeforeInstrAndDbgRecords", referenced from:
      _llvm_position_builder_before_dbg_records in libllvm.a[2](llvm_ocaml.o)
  "_LLVMPrintDbgRecordToString", referenced from:
      _llvm_string_of_lldbgrecord in libllvm.a[2](llvm_ocaml.o)
ld: symbol(s) not found for architecture arm64
clang: error: linker command failed with exit code 1 (use -v to see invocation)
```

## 即時対処方針
1. `scripts/ci-verify-llvm-link.sh` を `audit-matrix` に組み込み、`LLVM_LINK_MISMATCH` / `LLVM_SYMBOL_MISSING` の検出でジョブ失敗
2. Homebrew `llvm@18` と opam LLVM バインディングのバージョン整合を取り、`$OPAM_SWITCH_PREFIX/lib/llvm` をリンク優先に設定
3. `otool -L` でリンク先ライブラリを確認し、`/opt/homebrew/Cellar` 経由の `libunwind` が再輸出されている場合は `install_name_tool -change` で修正

## 2025-02-XX ローカル検証メモ
- Homebrew `llvm@19` を導入 (`brew install llvm@19` → `brew unlink llvm@18`) し、`~/.zprofile` に以下を追記して環境変数を固定:
  ```
  export PATH="/opt/homebrew/opt/llvm@19/bin:$PATH"
  export LDFLAGS="-L/opt/homebrew/opt/llvm@19/lib $LDFLAGS"
  export CPPFLAGS="-I/opt/homebrew/opt/llvm@19/include $CPPFLAGS"
  export CMAKE_PREFIX_PATH="/opt/homebrew/opt/llvm@19:$CMAKE_PREFIX_PATH"
  ```
  `llvm-config --version` が `19.1.7` を指すことを確認。
- `opam update` → `opam upgrade` で `conf-llvm-static.19` / `llvm.19-static` を導入。念のため `opam pin add llvm 19-static` を実施し、将来の `opam upgrade` でダウングレードしないように固定。
- `python3 compiler/ocaml/scripts/gen_llvm_link_flags.py --macos-arm64` 実行後、`_build/default/src/llvm_gen/llvm-link-flags.sexp` が `-L ~/.opam/.../lib/llvm` と `/opt/homebrew/Cellar/llvm@19/19.1.7/lib` を先頭に含むことを確認。
- `opam exec -- nm -gU $(opam var lib)/llvm/libLLVM-19.a | grep LLVMConstStringInContext2`（統合ライブラリ）などでシンボルが存在することを確認。OCaml パッケージは分割アーカイブ (`libllvm_XCore.a` 等) を提供するため、`libLLVMCore.a` 単体は存在しない点に注意。
- `opam exec -- dune clean && opam exec -- dune build && opam exec -- dune runtest --display=short` を実行し、リンクエラー無しで全テストが通過することを確認。
- この作業で得た出力（S 式、`llvm-config` 結果、テストログ）は CI への反映検討時に参照する。

## 収集すべき検証ログ
| 項目 | 取得方法 | 目的 |
|------|----------|------|
| LLVM バージョンと libdir | `opam exec -- llvm-config --version --libdir --system-libs` | opam スイッチが正しく参照されているか確認 |
| LLVM Core シンボル | `nm -gU $(opam var lib)/llvm/libLLVMCore.dylib | grep LLVMConstStringInContext2` | バイナリ互換性の有無 |
| CLI/テスト依存ライブラリ | `otool -L src/main.exe`, `otool -L tests/test_llvm_array_access.exe` | `@rpath` と実パスの整合性 |
| RPATH 設定 | `install_name_tool -print_rpath <binary>` | 実行時探索パスの検証 |
| `ci-verify-llvm-link.sh` レポート | `opam exec -- bash scripts/ci-verify-llvm-link.sh --report ...` | CI での検証結果保存 |
| `collect-iterator-audit-metrics.py` 拡張ログ | `--append-from` オプションで JSON 出力 | 監査ダッシュボードとの連携 |

### ローカル再現手順（ARM64）
1. `opam switch create reml-macos-ci 5.2.1` → `opam install . --deps-only --with-test`
2. `brew install llvm@18`（未導入の場合）し、`brew info llvm@18` でパスを把握
3. `python3 compiler/ocaml/scripts/gen_llvm_link_flags.py --macos-arm64`
4. `opam exec -- dune build --verbose` で失敗ログを採取
5. 直後に `opam exec -- bash scripts/ci-verify-llvm-link.sh --report tmp/ci-verify-llvm-link.md` を実行し、`tmp/ci-verify-llvm-link.md` を本ノートへ添付

### CI 連携チェックリスト
- [ ] `audit-matrix` ジョブに `ci-verify-llvm-link.sh` を追加し、`reports/audit/macos/<run_id>/ci-verify-llvm-link.md` をアーティファクト化
- [ ] `collect-iterator-audit-metrics.py` のサマリに `llvm.link.status` / `llvm.link.missing_symbols[]` を追加
- [ ] `ffi_bridge.audit_pass_rate` の判定と同じゲートで `llvm.link.status != success` をブロック条件に設定
- [ ] `docs/notes/linux-ci-llvm-link-error-report.md` と内容の差分を定期的に突き合わせ、共通対策を抽出

## 技術的負債との関連
- `compiler/ocaml/docs/technical-debt.md` ID 23（macOS FFI サンプル自動検証）に「LLVM リンク互換性レポート（本ノート）を CI アーティファクトとして保存し、run ID を記録する」を完了条件として追記予定
- FFI 検証ジョブでは `ci-verify-llvm-link.md` のステータスを参照し、サンプル実行前にリンク不整合が無いことを確立する

## TODO
- [ ] Homebrew `llvm@18` の更新に追従して再検証（2025-03 予定）
- [ ] `ci-verify-llvm-link.sh` に JSON 出力モードを追加し、`collect-iterator-audit-metrics.py` が直接読み込めるようにする
- [ ] `audit-diff` レポートで `llvm.link.status` を可視化し、PR コメントにリンク検証結果を含める
- [ ] Linux ノートとの共通テンプレート化（`docs/notes/templates/llvm-link-report.md` を検討）
