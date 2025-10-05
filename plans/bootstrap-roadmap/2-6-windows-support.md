# 2.6 Windows x64 (MSVC ABI) 対応計画

## 目的
- Phase 2 マイルストーン M4 に向けて、`-target x86_64-pc-windows-msvc` のビルドパイプラインを確立し、Windows 環境でのスモークテストを完了させる。
- System V ABI との差分を整理し、Phase 3 のクロスコンパイル機能拡張に備える。

## スコープ
- **含む**: LLVM TargetMachine 設定、MSVC 呼出規約対応、名前マングリング、PE 生成、GitHub Actions (windows-latest) テスト、ランタイムビルド。
- **含まない**: ARM64 Windows、MinGW、UWP 対応。必要に応じて別計画とする。
- **前提**: Phase 1 の x86_64 Linux ターゲットが安定、Phase 2 の型クラス/効果/FFI 実装が Windows でビルドできるよう調整済み。

## 作業ブレークダウン
1. **Toolchain 準備**: LLVM/MSVC の組み合わせを決定し、`0-3-audit-and-metrics.md` にバージョンを記録。CI 用セットアップスクリプトを作成。
2. **ABI 適合**: Calling convention, struct layout, alignment の差異を確認し、LLVM IR 生成側でターゲット切替ロジックを実装。
3. **ランタイム移植**: ランタイム C コードを Windows ビルドに対応させ、`cl.exe`/`link.exe` でライブラリを生成。
4. **テスト導入**: GitHub Actions に Windows ジョブを追加し、Parser〜ランタイムのスモークテストとサンプル実行を行う。
5. **ドキュメント整備**: `guides/llvm-integration-notes.md` に Windows セットアップ手順、`notes/llvm-spec-status-survey.md` に差分報告を追記。
6. **成果物署名検討**: コードサイニング証明書の取得要否を調査し、Phase 4 リリース準備に備えて方針を `0-4-risk-handling.md` に記録。

## 成果物と検証
- Windows ジョブが安定稼働し、`llc -mtriple=x86_64-pc-windows-msvc` で生成したバイナリが実行可能であること。
- ABI 差分がドキュメント化され、レビュー記録が残る。
- CLI で `--target x86_64-pc-windows-msvc` を指定可能になり、テストケースが通過。

## リスクとフォローアップ
- CI の時間が延びる場合は nightly ジョブと PR ジョブを分離する。
- Windows 固有のファイルパス・改行問題に対応するため、テストで共通抽象化を導入。
- 署名や配布のプロセスは Phase 4 で本格化するため、必要な下調べを `notes/` に記録。

## 参考資料
- [2-0-phase2-stabilization.md](2-0-phase2-stabilization.md)
- [guides/llvm-integration-notes.md](../../guides/llvm-integration-notes.md)
- [notes/llvm-spec-status-survey.md](../../notes/llvm-spec-status-survey.md)
- [0-3-audit-and-metrics.md](0-3-audit-and-metrics.md)
- [0-4-risk-handling.md](0-4-risk-handling.md)

