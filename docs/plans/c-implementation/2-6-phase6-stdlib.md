# フェーズ 6: 標準ライブラリとセルフホスティング

このフェーズでは、`Core` ライブラリを充実させ、実世界のコードでコンパイラの正当性を検証することに焦点を当てます。

## 6.1 ランタイムライブラリ戦略
- **場所**: `compiler/c/lib/runtime.c` (または類似)。
- **内容**:
  - メモリ割り当てラッパー (必要なら GC フック)。
  - IO プリミティブ (`print`, `read_file`)。
  - システムプリミティブ (`time`, `env`)。
- **バインディング**: これらの C 関数を `extern "C"` として Reml に公開する。

## 6.2 Core ライブラリの移植
- **ソース**: `compiler/c/lib/core/` (Reml ファイル)。
- **タスク**:
  - `Core.Prelude`, `Core.Collections` を `compiler/ocaml` または `compiler/rust` から再実装/コピーし、C ランタイムに適応させる。
  - パフォーマンスが重要なセクション (`@intrinsic`) に対してネイティブ C 実装を作成する。

## 6.3 セルフホスティングテスト (Spec Core)
- **目標**: `examples/spec_core/` のテストスイートをパスする。
- **ハーネス**:
  - `spec_core` 内の各 `.reml` ファイルをコンパイルして実行するテストランナーを作成。
  - `Assertion Failed` または成功終了を検証する。

## 6.4 実用的な例
- **目標**: `examples/practical/http_server.reml` (または類似) を実行する。
- **タスク**:
  - 必要な `Core.Net` プリミティブ (socket, bind, listen) を C で実装。
  - 非同期 IO の挙動を検証。

## 6.5 パッケージングと配布
- **タスク**:
  - インストーラ (.deb, .msi, .dmg) を生成するための `CPack` 設定を作成。
  - `reml help` とエラーメッセージを洗練させる。

## チェックリスト
- [ ] `Core` ライブラリのプリミティブが C で実装された。
- [ ] `stdlib` がコンパイルされリンクされる。
- [ ] `examples/spec_core` のパス率が > 90% になる。
- [ ] 自明でないアプリケーションをコンパイルして実行できる。
