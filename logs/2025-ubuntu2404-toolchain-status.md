# 2025-10-23 セッション（ツールチェーン状態確認と不要ライブラリ削除）

## セッションメタデータ
- **実行目的**: `update-alternatives` の設定結果を確認し、不要になった `libllvm19` を削除してディスク使用量とライブラリ競合リスクを軽減する。
- **関連タスク**: `docs/notes/linux-ci-local-setup-2025.md` セクション 2（必要パッケージの導入）フォローアップ。

## コマンドと結果概要
1. `update-alternatives --display clang`
   - 自動モードで `/usr/bin/clang-18` が最適リンクとなり、現在も同バイナリを指していることを確認。
2. `update-alternatives --display clang++`
   - 自動モードで `/usr/bin/clang++-18` が選択されていることを確認。
3. `sudo apt autoremove`
   - 不要パッケージ `libllvm19` を削除し、約 129 MB のディスクを解放。

## update-alternatives --display clang
```text
clang - 自動モード
  最適なリンクのバージョンは '/usr/bin/clang-18' です
  リンクは現在 /usr/bin/clang-18 を指しています
  リンク clang は /usr/bin/clang です
/usr/bin/clang-18 - 優先度 50
```

## update-alternatives --display clang++
```text
clang++ - 自動モード
  最適なリンクのバージョンは '/usr/bin/clang++-18' です
  リンクは現在 /usr/bin/clang++-18 を指しています
  リンク clang++ は /usr/bin/clang++ です
/usr/bin/clang++-18 - 優先度 50
```

## sudo apt autoremove
```text
パッケージリストを読み込んでいます... 完了
依存関係ツリーを作成しています... 完了        
状態情報を読み取っています... 完了        
以下のパッケージは「削除」されます:
  libllvm19
アップグレード: 0 個、新規インストール: 0 個、削除: 1 個、保留: 0 個。
この操作後に 129 MB のディスク容量が解放されます。
続行しますか? [Y/n] 
(データベースを読み込んでいます ... 現在 175170 個のファイルとディレクトリがインストールされています。)
libllvm19:amd64 (1:19.1.1-1ubuntu1~24.04.2) を削除しています ...
libc-bin (2.39-0ubuntu8.6) のトリガを処理しています ...
```

## 備考
- `libllvm19` の削除により、現行利用する LLVM 18 系と将来導入予定の 18.1.8（opam パッケージ）との競合を最小化できる。
- `update-alternatives` のリンクが 18 系を指していることを記録したため、後続のビルドログでバージョンギャップが発生した際のトラブルシュートに利用可能。

## /usr/bin/llvm-config* 出力
```text
/usr/bin/llvm-config-18  /usr/bin/llvm-config-19
```

## which llvm-config-* 出力
```text
/usr/bin/llvm-config-18
/usr/bin/llvm-config-19
```

## llvm-config-*-version
```text
llvm-config-18 --version
18.1.3

llvm-config-19 --version
19.1.1
```
