# 0.2 Windows ツールチェーン監査計画

本章は Rust 移植に先立って Windows 向け開発・CI 環境を監査し、MSVC / GNU 両ツールチェーンで再現性の高いセットアップを確立するための手順を定義する。Phase 2-6 で判明した課題を踏まえ、P0 では環境差異の検出とログ収集フォーマットを標準化する。

## 0.2.1 目的
- Rust 実装が参照する Windows ツールチェーン（`rustup`, LLVM, SDK, PowerShell スクリプト）の依存関係を整理し、欠落時の検出手順をまとめる。
- OCaml 実装で運用している PowerShell 補助スクリプトを Rust 版でも再利用できるよう、必須引数と出力ログ形式を定義する。
- CI（GitHub Actions windows-latest）の前提条件と手動検証手順を提示し、`dual-write` 構成で安定した結果を得られるようにする。

## 0.2.2 監査対象とチェックリスト

| 項目 | 推奨構成 | 確認コマンド / スクリプト | ログに残す情報 | 備考 |
| --- | --- | --- | --- | --- |
| Rust ツールチェーン | `rustup` (stable), `rustup target add x86_64-pc-windows-msvc`, `x86_64-pc-windows-gnu` | `rustup show`, `rustc -Vv` | コンポーネント一覧、ホスト triple、プロファイル | P1 で nightly を利用する場合は追加記録。 |
| LLVM | LLVM 19 系（MSVC 配布） + `llvm-config`, `opt`, `llc` | `llvm-config --version`, `where opt`, `where llc` | バージョン、パス、`DataLayout` サマリ | `LLVM_DIR` 環境変数を記録し、Rust 版 `llvm-sys` 設定に利用。 |
| MSVC | Visual Studio Build Tools, `cl.exe`, `link.exe` | `vswhere.exe -latest -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64` | `installationPath`, バージョン, `VCToolsVersion`, `WindowsSdkDir` | `setup-windows-toolchain.ps1` で自動収集可。 |
| MinGW (任意) | `x86_64-w64-mingw32-gcc`, `ld`, `ar` | `where gcc`, `gcc -v` | バージョン、`sysroot` | GNU toolchain でのクロス検証向け。 |
| PowerShell スクリプト | `tooling/toolchains/setup-windows-toolchain.ps1`, `check-windows-bootstrap-env.ps1`, `run-final-check.ps1` | `.\setup-windows-toolchain.ps1 -CheckOutputJson <出力パス>`, `.\check-windows-bootstrap-env.ps1 -OutputJson <出力パス>` | 実行結果ステータス、エラー詳細、生成 JSON | Rust 版セットアップ時も同スクリプトを再利用し、ログは `reports/toolchain/...` へ保存。 |
| GitHub Actions | `windows-latest` イメージ、MSVC/MinGW マトリクス | CI 設定ファイル（`.github/workflows/`） | 成功/失敗ログ、`collect-iterator-audit-metrics.py` 出力 | Rust 版 CI 追加時に同マトリクスへジョブ追加。 |

## 0.2.3 ログ出力と保管方針
- PowerShell スクリプトは `-CheckOutputJson` / `-OutputJson` 引数で JSON を出力し、`reports/toolchain/windows/YYYYMMDD/` に保存する。Rust 版は同フォーマットを読み取り、差分比較に利用する。
- `setup-windows-toolchain.ps1` の実行時には環境変数 `RUSTUP_TOOLCHAIN`, `LLVM_VERSION`, `VCVARSALL_ARCH` を明示し、ログへ出力する。
- CI では以下のアーティファクトを保存する:
  - `collect-iterator-audit-metrics.py --platform windows-msvc` の結果（JSON）
  - `check-windows-bootstrap-env.ps1 -OutputJson <出力パス>` の結果
  - `cargo version`, `rustc -Vv`, `cl.exe /?` の標準出力

## 0.2.4 検証シナリオ
1. **MSVC パス検証**  
   - `vswhere.exe` で取得したパスに基づき `vcvars64.bat` を呼び出し、`cl.exe` がパスに含まれているか確認。  
   - `check-windows-bootstrap-env.ps1` 実行後に `diagnostic` レベルの警告が 0 件であることを確認する。
2. **GNU パス検証**  
   - `rustup target add x86_64-pc-windows-gnu` を実行し、`cargo build --target x86_64-pc-windows-gnu` でサンプルをビルド。  
   - `x86_64-w64-windows-gnu` 用のリンカが正しく解決されるか `-v` オプションで確認。
3. **LLVM コマンド整合**  
   - `opt --version` と `llc --version` が同一バージョンであることを確認。  
   - `llvm-config --host-target` の出力を `unified-porting-principles.md` §2 の `TargetMachine` 設定と照合する。
4. **Rust CLI PoC 事前検証**  
   - Phase P1 の Rust CLI が準備でき次第、`collect-iterator-audit-metrics.py --require-success --platform windows-msvc` を Rust 出力に対して実行し、OCaml 版と比較する。

## 0.2.5 CI 統合方針
- `.github/workflows/` に Windows 用 Rust ジョブを追加し、以下の順序で実行する:
  1. PowerShell で `setup-windows-toolchain.ps1 -CheckOutputJson <出力パス>`
  2. `rustup`/`cargo` セットアップ（MSVC/MinGW マトリクス）
  3. Rust CLI/ライブラリのビルド + スモークテスト
  4. `collect-iterator-audit-metrics.py --platform windows-msvc --require-success`
  5. アーティファクト保存 (`reports/toolchain`, `metrics/`)
- 既存の OCaml ジョブと同じ Gate (`stage_mismatch_count == 0`) を Rust ジョブにも適用し、差分発生時はジョブ全体を失敗させる。

## 0.2.6 フォローアップ
- 監査結果で特定したギャップは `docs/plans/bootstrap-roadmap/2-6-windows-support.md` と `docs/plans/rust-migration/4-0-risk-register.md`（P4 予定）へ転記する。
- 新しいツールチェーンバージョンを採用する際は `tooling/toolchains/versions.toml` を更新し、`docs-migrations.log` に記録する。
- 脚注・追加資料は `docs/notes/windows-rust-toolchain-study.md`（新規作成予定） 等へまとめ、仕様やガイドに影響する場合は `docs/spec/` を同時更新する。

## 0.2.7 関連資料
- `docs/plans/bootstrap-roadmap/2-6-windows-support-migration-options.md`
- `docs/plans/bootstrap-roadmap/2-6-windows-support.md`
- `docs/plans/bootstrap-roadmap/windows-llvm-build-investigation.md`
- `tooling/toolchains/README.md`
- `tooling/toolchains/setup-windows-toolchain.ps1`
- `tooling/toolchains/check-windows-bootstrap-env.ps1`
- `tooling/toolchains/run-final-check.ps1`

---

> **監査メモ**: Windows 固有の課題が収集できなかった場合でも、必ず空欄ではなく「未検証」ステータスを `check-windows-bootstrap-env.ps1` の JSON に記録する。Rust 版の初回実行で差異を検出しやすくするためである。
