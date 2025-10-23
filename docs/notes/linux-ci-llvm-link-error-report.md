# Linux CI LLVM リンクエラー調査レポート（2025-02-XX）

## 概要
- 対象: GitHub Actions `bootstrap-linux.yml` の `Build` ジョブ（`opam exec -- dune build`）
- 現象: OCaml → LLVM 依存コードをリンクする際に `LLVMConstStringInContext2` などのシンボル解決に失敗し、`collect2: error: ld returned 1 exit status` でビルドが中断
- 影響: CLI 本体（`src/main.exe`）および LLVM 依存テストバイナリ（`test_llvm_*`, `test_cli_callconv_snapshot`, `test_ffi_lowering` など）が一括で失敗

## 症状の詳細
- `libLLVMSupport.a(Process.cpp.o)` から `setupterm`, `set_curterm`, `tigetnum`, `del_curterm` など `ncurses` 系シンボルが解決できない
- `libllvm.a(llvm_ocaml.o)` から `LLVMConstStringInContext2`, `LLVMPositionBuilderBeforeInstrAndDbgRecords`, `LLVMPrintDbgRecordToString` 等の新 API が解決できない
- いずれも **ランタイムではなくリンク時** の欠落であり、ライブラリ探索順序またはバージョンミスマッチが原因であると推定

## 原因分析
### 1. LLVM 本体のバージョン差異
- `opam` 経由でインストールされた OCaml LLVM バインディングは LLVM 18.1 系の新 API を要求
- 一方で `link_flags` が参照している `libLLVM*.so` / `libLLVM*.a` が Ubuntu system LLVM 18.1.0 由来になっており（`/usr/lib/llvm-18/...`）、OCaml バインディングが期待する `LLVMConstStringInContext2` 等を含んでいない
- `llvm-config` が system 側を指すと、`--libs` や `--system-libs` で古いライブラリが列挙され、リンク時に混在が発生する

### 2. terminfo (`ncurses`) 依存解決不足
- `LLVMSupport` は Linux で `terminfo` を利用するため `-ltinfo` / `-lncursesw` のリンクが必要
- `llvm-config --system-libs` から取得できない環境（または混在環境）では、リンクフラグにこれらが含まれず `set_curterm` 等が未解決となる

### 3. ライブラリ探索パスの競合
- 現在の `compiler/ocaml/scripts/gen_llvm_link_flags.py` は `llvm-config --libdir` のみを `-L` に追加するが、`clang` や `lld` のインストール状況によって `/usr/lib/llvm-18` が先に解決されるケースがある
- その結果、**opam 側のライブラリ**（`$OPAM_SWITCH_PREFIX/lib/llvm`）が参照されず、旧版の `libLLVM*.so` を拾ってしまう

## これまでの対応
1. `gen_llvm_link_flags.py` で `llvm-config` 未検出時のフォールバックを定義（`-lLLVM-18` 追加）  
   → 症状は改善せず
2. `OPAM_SWITCH_PREFIX/bin/llvm-config` を最優先で利用するよう修正  
   → 一部の環境で `llvm-config` 自体が存在せず、完全には解決せず
3. `-L` 追加のみに留まり、`-Wl,-rpath` を付与しておらず、実行時に system 側が優先される余地が残っている
4. `-ltinfo` を明示的に追加したが、`-lncursesw` が不足しているため `setupterm` 解決に失敗

## 推奨解決策
1. **ライブラリ探索の優先順位付け**
   - `$OPAM_SWITCH_PREFIX/lib/llvm` を最優先に `-L` 追加し、かつ `-Wl,-rpath,$OPAM_SWITCH_PREFIX/lib/llvm` を明示（`LD_LIBRARY_PATH` に依存しない）
   - system LLVM (`/usr/lib/llvm-*`) は最後に fallback として列挙
2. **必須コンポーネントの強制リンク**
   - `libLLVMCore`, `libLLVMBitWriter`, `libLLVMSupport`, `libLLVM-C` 等を `-l` 指定で明示。`llvm-config --libs` の出力に依存しない（バージョン差異を吸収）
3. **terminfo 依存の補完**
   - Linux では `-ltinfo` のみならず `-lncursesw` / `-lcurses` を順に試すロジックを導入（存在チェックで動的に追加）
4. **dune 前処理のリセット**
   - `git clean -fd compiler/ocaml/src/llvm_gen` → `opam exec -- dune clean` → `opam exec -- dune build` を行い、古い `.sexp` / `.o` を一掃して再生成
5. **CI 確認手順**
   - 修正後に `bootstrap-linux.yml` の該当部分で `cat compiler/ocaml/src/llvm_gen/llvm-link-flags.sexp` を出力し、リンクフラグが想定通りになっているか検証
   - 必要に応じて `ldd src/main.exe` を CI の `post-build` ステップで実行し、参照している `libLLVM` が opam 側かを確認

## 参考ログ
```
/usr/bin/ld: /usr/lib/llvm-18/lib/libLLVMSupport.a(Process.cpp.o): undefined reference to `set_curterm'
/usr/bin/ld: /home/runner/work/reml/reml/_opam/lib/llvm/libllvm.a(llvm_ocaml.o): undefined reference to `LLVMConstStringInContext2'
```

## 今後のアクション
1. `gen_llvm_link_flags.py` の改善版（`-Wl,-rpath` と opam libdir 優先）を作成してコミット
2. CI 上で `dune clean` → `opam exec -- dune build` を実行して再検証
3. 依然として未解決の場合は、`llvm-config --version` の出力と `ld -lLLVM` の探索結果をログに記録して追加調査
4. スイッチ内に `llvm-config` がない場合は `opam install conf-llvm-18`（または `llvm.18`）の導入を検討し、CI 前処理でインストールする

### 2025-02-XX 対応ログ
- `bootstrap-linux.yml` に以下の変更を適用:
  - `opam exec -- dune clean` を明示的に実行し、古い `llvm-link-flags.sexp` を破棄
  - `opam env` を用いて `$OPAM_PREFIX` を取得し、`LLVM_CONFIG`/`LD_LIBRARY_PATH`/`PKG_CONFIG_PATH` を `GITHUB_ENV` にエクスポート
  - `opam exec -- llvm-config --version/--libdir` をログに出力
  - ビルド後に `compiler/ocaml/src/llvm_gen/llvm-link-flags.sexp` を表示
- 目的: opam 側の LLVM バイナリが優先されることを保証し、CI ログでリンクフラグの実体を即座に確認できるようにする

### 2025-02-XX 追加調査ログ
- 上記対応直後の CI で `LLVM_CONFIG=${OPAM_PREFIX}/bin/llvm-config` を強制した結果、スイッチ内に実行ファイルが存在せず `Command not found 'llvm-config'` で失敗
- 原因: `opam switch` に `llvm-config` バイナリを含むパッケージがインストールされていない（OCaml 用 `llvm` ライブラリは C API のみ提供）
- 改善策:
  - `bootstrap-linux.yml` の環境設定ステップで `if [ -x ... ]` により存在確認を行う
  - 見つからない場合は system にインストール済みの `llvm-config-18` → `llvm-config` の順でフォールバック
  - いずれも無い場合は早期に `exit 127` して原因を明示
  - ログに `[info]` / `[error]` メッセージを出力し、使用された `llvm-config` のパスを記録

### 2025-02-XX 再発ログ
- フォールバックで `/usr/bin/llvm-config-18` を検出したものの、後続で `opam exec -- llvm-config` を呼び出していたため `Command not found 'llvm-config'` が再発
- 原因: `opam exec` のシェルはスイッチ環境の `PATH` に限定されるため、system `llvm-config-18` が見つからない
- 対応:
  - `opam exec` を経由せず、検出した絶対パス `${LLVM_CFG}` を直接実行して `--version`/`--libdir` を取得
  - `LLVM_CONFIG` の値を `GITHUB_ENV` に書き出しつつ、同ステップで `LLVM_CFG` 変数を利用するよう修正
- 課題（未解決）:
  - `llvm-config` がどこにも存在しない場合は 127 で停止するため、必要に応じて `sudo apt-get install llvm-18-dev` を再確認する
  - さらに堅牢化するには `opam install conf-llvm-18` 等でスイッチ内に `llvm-config` を配置する選択肢も検討

## 知見まとめ
- OCaml LLVM バインディングはソース互換性よりもビルド時のバイナリ互換性がシビアであり、CI 環境では `llvm-config` のバージョン差異が顕著な失敗要因になる
- Linux では LLVM ライブラリが `terminfo` に依存するため、`-ltinfo` だけでなく `-lncursesw` や `-lcurses` を追加する保険が必要
- `-Wl,-rpath` を付与しないと実行時に system ライブラリを掴む可能性があり、ビルド時に opam ライブラリを指しても安全ではない
- `llvm-link-flags.sexp` は `dune clean` で再生成しない限り古い内容を保持するため、スクリプト変更時は必ずクリーンビルドを実施する必要がある
