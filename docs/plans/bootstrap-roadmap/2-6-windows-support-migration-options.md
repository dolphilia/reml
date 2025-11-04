# 2-6 Windows サポート: 言語移植オプション検討

## 背景
- Phase 2-6 では `-target x86_64-pc-windows-msvc` のパイプライン確立が未完であり、LLVM ツールチェーンが MSVC 配布版と MSYS2 版で混在している（[docs/plans/bootstrap-roadmap/2-6-windows-support.md](./2-6-windows-support.md)）。
- Windows 向け OCaml LLVM バインディングが `llvm` / `llvm.bitwriter` の欠落でビルド失敗し、`opam install llvm` を CI に組み込めないことが判明している（同上 41–57 行相当）。
- Phase 2-5 までのレビューでは Linux/macOS の安定度が高く、Windows のみが長期課題として残っている（[docs/plans/bootstrap-roadmap/2-5-review-log.md](./2-5-review-log.md)）。
- Phase 3 以降は標準ライブラリの整備とセルフホスト準備が主題となるため、Windows サポートを早期に安定化させる代替経路が求められている（[docs/plans/bootstrap-roadmap/3-0-phase3-self-host.md](../3-0-phase3-self-host.md)）。

## 現状課題の整理
1. LLVM 19.1.1 (MSVC 配布) と MSYS2 LLVM 16.0.4 の併用により、`llc`/`opt` の解決優先度がセッションで揺れている。
2. `reml-msvc-env` を手動で呼び出さない限り `cl.exe`/`link.exe` が検出されず、自動化が不安定。
3. OCaml 版では LLVM バインディングを静的リンクさせるための `.lib` 配置と `conf-llvm-static` の検証が複雑で、CI で再現しづらい。
4. Phase 3 のロードマップではランタイム Capability と診断の整合に集中する必要があり、Windows 固有のビルド問題が継続すると着手が遅れる。

## 移植時の評価軸
- **Windows ツールチェーン整合性**: MSVC / MinGW / GitHub Actions (windows-latest) で同一言語の公式ツールを利用できるか。
- **LLVM 連携**: LLVM 19 以降の API へ継続的に追随でき、`TargetMachine` 設定や DataLayout 差分を安全に扱えるか。
- **Reml 仕様との親和性**: 代数的データ型、パターンマッチ、遅延評価、エフェクト表現など OCaml 実装が持つ抽象を移せるか。
- **学習 / メンテコスト**: 現行チームが継続開発できる習熟度、CI 設備構築の難易度、コミュニティサポートの有無。
- **段階的移行可否**: パーサー・型推論・LLVM IR 生成・ランタイムのモジュールごとに段階移植できるか、または全面書き換えが必要か。

## 候補言語評価

### Rust
- **長所**
  - `rustup` で MSVC / GNU の両ツールチェーンを公式に配布しており、GitHub Actions での再現性が高い。
  - `llvm-sys` / `inkwell` などのバインディングにより LLVM 19 系機能へ追随でき、`TargetMachine` や `DataLayout` を直接操作可能。
  - 代数的データ型、パターンマッチ、`Result`/`Option` といった Reml 仕様に近い抽象が標準で提供され、型推論ロジックの移植がしやすい。
  - 所有権モデルによりランタイムの安全性を高められ、セルフホスト準備時の信頼性向上が期待できる。
- **短所**
  - ライフタイムと借用の習得コストが高く、既存 OCaml チームのキャッチアップ期間を要する。
  - マクロやジェネリクスでパーサーコンビネーターを表現する際の抽象化が複雑になり、DSL 的記述の移植に追加設計が必要。
- **段階移行案**
  - まずフロントエンド（パーサー + 型推論）と LLVM IR 生成を Rust で実装し、ランタイムは C 実装と連携。
  - Rust 版 CLI を `Phase 2-6` の Windows CI に追加し、ビルドパイプラインの安定度とパフォーマンスを検証。

### F# / .NET
- **長所**
  - OCaml に近い構文・型システムを持ち、既存コードの移植が比較的容易。
  - Visual Studio / JetBrains Rider など Windows ネイティブ IDE でのデバッグが充実し、MSBuild で統合パイプラインを整備しやすい。
  - .NET ランタイムが Windows に標準搭載されており、インストール手順を簡略化できる。
- **短所**
  - LLVM 連携には `LLVMSharp` 等のラッパーが必要で、ネイティブ ABI 呼び出しや DataLayout の細かな調整に追加層が入る。
  - クロスプラットフォーム対応では Mono / .NET Runtime の整備が必須となり、macOS/Linux 向け CI の再構築コストが発生。
- **段階移行案**
  - Windows 専用フロントエンドとして F# を導入し、LLVM IR 生成を C++ や既存 OCaml と連携させるハイブリッド構成を試験。
  - 成果を Phase 2-6 のスモークテストに組み込み、.NET ベースのデバッグ体験向上を図る。

### C++20
- **長所**
  - LLVM フロントエンド実装の実績が豊富で、公式 API を直接利用できる。
  - MSVC / MinGW どちらでも成熟したビルドシステムがあり、既存の CI ワークフローに組み込みやすい。
  - ABI や DataLayout を正確に制御でき、Phase 2-6 が求める win64 calling convention への対応が容易。
- **短所**
  - パターンマッチや代数的データ型が標準で提供されず、Reml 仕様を表現するには `std::variant` やテンプレートの大量利用が必要で、実装が複雑化。
  - エラー処理やパーサーコンビネーターを関数型スタイルで書き直すコストが重く、バグ混入リスクが高い。
- **段階移行案**
  - LLVM IR 生成とランタイム周辺のみを C++ に置き換え、パーサーは OCaml または別言語で維持する多言語構成を検証。
  - 将来的な全面移植は Phase 3 の開発速度を下げる恐れがあり、選択する場合は専任チームの確保が前提となる。

### Zig
- **長所**
  - C ABI 互換とクロスコンパイル機能が非常に強力で、単一ビルドスクリプトで MSVC / MinGW 両方を制御できる。
  - 構文が小さく、ランタイムや FFI 層を低コストで記述できる。
- **短所**
  - LLVM バインディングの成熟度が Rust/C++ に劣り、最新版 LLVM との同期を自前で追う必要が出やすい。
  - パターンマッチや型推論、効果システムを支える抽象が標準では提供されず、Reml の仕様を移すには補助ライブラリを多く実装する必要がある。
- **段階移行案**
  - ランタイム / FFI 層のみ Zig で再実装し、コンパイラフロントエンドは既存言語で維持するハイブリッド案を先に検証。
  - 成熟度を見極めた上で全面移行可否を判断する。

## 推奨アクション
1. Rust でフロントエンド + LLVM IR 生成の PoC を作成し、`x86_64-pc-windows-msvc` / `x86_64-w64-windows-gnu` それぞれのビルドとスモークテストを検証する。
2. F# で Windows 専用フロントエンド PoC を実施し、IDE 連携と LLVMSharp 統合の工数を評価する。
3. PoC 結果を比較し、Phase 2-6 ドキュメントへ Windows 対策ロードマップとして追記するレビューセッションを設定する。
4. 選定後は Phase 3 マイルストーンに影響するタスク（標準ライブラリ実装、Capability 整合）の再計画と、`docs/plans/bootstrap-roadmap/windows-llvm-build-investigation.md` の更新を行う。

## 参照資料
- [docs/plans/bootstrap-roadmap/2-6-windows-support.md](./2-6-windows-support.md)
- [docs/plans/bootstrap-roadmap/2-5-review-log.md](./2-5-review-log.md)
- [docs/plans/bootstrap-roadmap/3-0-phase3-self-host.md](../3-0-phase3-self-host.md)
- [docs/plans/bootstrap-roadmap/windows-llvm-build-investigation.md](./windows-llvm-build-investigation.md)
- [docs/spec/0-0-overview.md](../../spec/0-0-overview.md)
