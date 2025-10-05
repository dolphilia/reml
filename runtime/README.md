# runtime ディレクトリ構成（準備中）

Phase 1 の最小ランタイムおよび将来の Capability 拡張を収容する領域です。`docs/plans/bootstrap-roadmap/1-5-runtime-integration.md` を基準にタスクを展開します。

## サブディレクトリ
- `native/`: C/LLVM ベースの最小ランタイムとターゲット別実装（Linux/Windows 等）を配置予定

必要に応じて `native/` 配下にターゲットごとのサブディレクトリ（`linux/`, `windows/` など）を追加します。

## TODO
- [ ] Phase 1 M3 に合わせて `native/` 内へ最小ランタイムのソースとテストを配置
- [ ] Phase 2 で Windows/MSVC 対応を追加し、共通ヘッダを整備
