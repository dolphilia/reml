# Reml 実装対応リポジトリ再編計画

## 1. 背景と目的
- 現状のリポジトリは仕様書（0-*, 1-*, 2-*...）がルート直下に羅列され、実装・ツール・補助資料がサブディレクトリに散在している。
- ブートストラップ計画（`docs/plans/bootstrap-roadmap/`）や実装比較サンプル（`examples/`）など、実装着手に向けた資料が増え、仕様書中心の構成では開発ワークフローを整理しづらい。
- Reml 実装を進めるために「実装リポジトリとしての入口」と「公式仕様書アーカイブ」を両立させる再編方針を定義し、段階的に移行するための計画を策定する。

## 2. 現状整理
### 2.1 主要ディレクトリ（再編後）
```
./
├─ README.md                 # プロジェクト概要・導線
├─ docs/
│   ├─ README.md             # 仕様・ガイド・ノートの索引
│   ├─ spec/                 # 章番号付き仕様書
│   ├─ guides/               # 運用ガイド
│   ├─ notes/                # 調査メモ
│   └─ plans/                # ブートストラップ計画
├─ compiler/                 # ブートストラップ実装領域
├─ runtime/                  # ランタイム・Capability 実装領域
├─ tooling/                  # CLI/CI/リリース/LSP 等のツール資産
├─ examples/                 # サンプル実装
├─ docs-migrations.log       # ドキュメント移行履歴
├─ AGENTS.md / CLAUDE.md     # AI エージェント指針
└─ .claude/                  # エージェント補助設定
```

### 2.2 文書間の依存関係
- 仕様書からの参照はすべて相対パス（例: `[1-1-syntax.md]`）で、`docs/guides/`・`docs/notes/`・`docs/plans/` からも同一形式でリンク。
- README / AGENTS / CLAUDE が「ルートに仕様書がある」ことを前提に説明しており、移設時に必ず改訂が必要。
- `docs/notes/process/guides-to-spec-integration-plan.md` 等、過去の移行計画書が残っており、再編時に更新またはアーカイブ整備が求められる。

### 2.3 実装フェーズとのギャップ
- `docs/plans/bootstrap-roadmap/` では OCaml 製ブートストラップ→セルフホストのロードマップが定義済みだが、ソース配置先やビルド体制の受け皿が未整備。
- 実装者が最初に開く README が仕様目次のみで、実装タスクへの導線が欠落。
- AI エージェント向け指針は「ドキュメント専用」を前提としており、コード編集時の注意事項が存在しない。

## 3. 再編の基本方針
1. **二層構造の確立**: ルートには実装・開発エントリポイントを配置し、仕様書は `docs/spec/` 以下へ体系化。
2. **既存番号体系の維持**: 章番号（0〜5）は維持し、ファイル名も可能な限り変更せずディレクトリ階層で整理する。
3. **導線の再設計**: ルート `README.md` をプロジェクト概要＋開発導線に更新し、仕様目次は `docs/README.md` へ移設。
4. **メタ文書の刷新**: `AGENTS.md` / `CLAUDE.md` を「ドキュメント + コード」両対応に改訂し、再編後のパス方針を明示。
5. **移行の段階的実施**: 大量リンク更新を伴うため、自動置換＋検証フェーズを挟んで段階的に移行する。

## 4. 成功指標
- 仕様書・ガイド・ノート・計画書すべてが `docs/` 配下で整然と参照でき、リンク切れが 0 である。
- ルート README から実装ブートストラップ計画・仕様アーカイブ・サンプル・将来メモへ遷移できる導線が 3 クリック以内で整備される。
- AI エージェント指針が更新され、コード編集・ビルド手順の基本方針が明記されている。
- 新ディレクトリ構成下で `docs/plans/bootstrap-roadmap/` に記載された計画を開始できる初期スケルトン（例: `compiler/ocaml/`）が確保されている。

## 5. 新ディレクトリ構成案
### 5.1 トップレベル
```
/
├─ README.md                # プロジェクト概要・導線（新規書き直し）
├─ docs/
│   ├─ README.md            # 仕様書・ガイドの索引
│   ├─ spec/                # 章別ディレクトリ（0〜5）
│   ├─ guides/              # 運用・ベストプラクティス
│   ├─ notes/               # 調査・メモ
│   └─ plans/               # 実装計画・ロードマップ
├─ compiler/
│   ├─ README.md            # ブートストラップ実装の受け皿
│   ├─ ocaml/               # Phase1 OCaml コンパイラ実装（`docs/plans/bootstrap-roadmap/1-x` に対応）
│   └─ self-host/           # Phase3 以降の Reml コンパイラ実装用プレースホルダ
├─ runtime/
│   ├─ README.md            # ランタイム整備計画との紐付け
│   └─ native/              # C/LLVM ベースの最小ランタイム（Phase1 `1-5-runtime-integration.md`）
├─ tooling/
│   ├─ README.md            # 開発者体験・CI/リリース整備のハブ
│   ├─ cli/                 # `1-6-developer-experience.md` で定義された CLI 資産
│   ├─ ci/                  # `1-7-linux-validation-infra.md` 等で言及される CI スクリプト・ローカル再現ツール
│   ├─ release/             # `6-2-multitarget-release-pipeline.md` の署名・配布スクリプト
│   └─ lsp/                 # Phase2 以降で整備する LSP / IDE 連携資産
├─ examples/                # 旧 `examples/` を改称し、実装サンプルを整理
├─ AGENTS.md                # AI エージェント共通指針
├─ CLAUDE.md                # Claude Code 専用ガイド
└─ docs-migrations.log      # 大規模移行の記録（任意、Git の履歴補助）
```

### 5.2 仕様ディレクトリ詳細
```
docs/spec/
├─ 0-0-overview.md
├─ 0-1-project-purpose.md
├─ 0-2-glossary.md
├─ 0-3-code-style-guide.md
├─ 1-0-language-core-overview.md
├─ 1-1-syntax.md
└─ （以下 2-*, 3-*, 4-*, 5-* の各仕様書）
```
- `docs/guides/`・`docs/notes/`・`docs/plans/` は `docs/` に移設し、必要に応じてプレフィックスを整理（例: `docs/notes/algebraic-effects-*` → `docs/notes/algebraic-effects/`）。
- 既存の `docs/plans/bootstrap-roadmap/` は `docs/plans/bootstrap-roadmap/` へ移動し、章内リンクも更新。

### 5.3 開発コンポーネント整備指針
- `compiler/ocaml/` は Phase 1 計画（`docs/plans/bootstrap-roadmap/1-0-phase1-bootstrap.md` ほか）で求められる OCaml ブートストラップ実装のソース・ビルドスクリプト・テストを保持する。Phase 2 の拡張に合わせ `ocaml/README.md` で対応範囲を追記する。
- `compiler/self-host/` は Phase 3 自己ホスト化（`docs/plans/bootstrap-roadmap/3-0-phase3-self-host.md`）の準備領域として空ディレクトリを用意し、セルフホスト実装開始時にサブモジュールを配置する。
- `runtime/native/` は最小 RC ランタイム（`docs/plans/bootstrap-roadmap/1-5-runtime-integration.md`）と将来の Capability 対応を配置し、ターゲット別サブディレクトリ（`linux/`, `windows/` など）を必要に応じて追加する。
- `tooling/cli/` は `docs/plans/bootstrap-roadmap/1-6-developer-experience.md` で定義された CLI 資産とヘルプドキュメント、`tooling/ci/` は `docs/plans/bootstrap-roadmap/1-7-linux-validation-infra.md` などで参照されるローカル再現スクリプトや CI 補助スクリプトを格納する。
- `tooling/release/` は `docs/plans/bootstrap-roadmap/6-2-multitarget-release-pipeline.md` に基づく署名・パッケージング・配布スクリプトの置き場とし、秘密情報管理のガイドラインを README に明記する。
- `tooling/lsp/` は Phase 2 以降に計画されている LSP/IDE 資産（`docs/plans/bootstrap-roadmap/3-6-core-diagnostics-audit-plan.md` など）を集約し、CLI と共通の診断ポリシーを共有する。
- `.github/workflows/` は現行どおり維持し、`tooling/ci/` のスクリプトから参照することで CI 設定とローカル補助の責務を分離する。

### 5.4 旧→新パス対応（代表）
| 現在のパス | 新パス案 | 備考 |
|-------------|-----------|------|
| `README.md` | `docs/README.md` (旧目次として) | ルート README は新規に作成
| `AGENTS.md` | `AGENTS.md` | ルートに据え置き。内容のみ改訂
| `CLAUDE.md` | `CLAUDE.md` | ルートに据え置き。内容のみ改訂
| `docs/guides/*` | `docs/guides/*` | パス更新（サブディレクトリ化を検討）
| `docs/notes/*` | `docs/notes/*` | テーマ別サブフォルダを追加
| `docs/plans/bootstrap-roadmap/*` | `docs/plans/bootstrap-roadmap/*` | リンク更新が大量発生
| `examples/*` | `examples/*` | README 内リンク修正必須

## 6. マイグレーションフェーズ
### Phase 0: 準備 (1〜2日)
- `docs/`・`compiler/`・`runtime/`・`tooling/`・`examples/` のディレクトリと README 下書きを作成し、`tooling/cli/` `tooling/ci/` `tooling/release/` など主要サブディレクトリを初期化。`AGENTS.md` / `CLAUDE.md` の改訂ドラフトを並行で準備。
- 自動リンク更新のためのスクリプト方針決定（`python` or `node`、`grep` / `sed` の併用）。
- 影響を把握するため、`grep -R "1-1-syntax.md"` などで参照数を洗い出し、移行順序を決定。

### Phase 1: 仕様書の移設 (3〜4日)
- 既存の章番号付き `*.md` を `docs/spec/` へ移動し、番号体系はファイル名のまま維持。
- 移動後に `find docs/spec -name "*.md"` で存在確認。
- 章内および外部からのリンクを新パスへ置換（自動置換 → 手動確認）。
- 一時的に `git mv` を活用し、履歴を保持。

### Phase 2: ガイド・ノート・計画書の整理 (2〜3日)
- `docs/guides/`・`docs/notes/`・`docs/plans/` を `docs/` 配下に移設。
- テーマごとにサブディレクトリ分割（例: `docs/notes/algebraic-effects/` へ統合）。
- `docs/notes/process/guides-to-spec-integration-plan.md` など旧計画書は「アーカイブ」として付箋を追記し、新構成の参照先を明示。

### Phase 3: サンプル・メタ文書対応 (1〜2日)
- `examples/` を `examples/` に改称し、内部 README のパス更新。
- `AGENTS.md` / `CLAUDE.md` の内容を再編（実装作業時のビルド・テスト方針やコードレビュー観点を追加）。
- 追加のメタ文書が必要な場合は `docs/notes/meta-guidelines.md` など `docs/` 配下に配置し、ルート README からの導線を整理。

### Phase 4: ルート README と導線整備 (2日)
- 新しい `README.md` を作成し、以下を明記:
  - プロジェクト概要と開発ロードマップの要約
  - 実装着手方法（`compiler/ocaml/` への導線、依存関係、ビルド計画）
  - 仕様アーカイブへのリンク (`docs/README.md`)
  - コミュニティ・運用ポリシーの概要
- `docs/README.md` には旧 README の章別目次を移植し、章ごとの説明を更新。

### Phase 5: 実装用ディレクトリの初期化 (同時並行可)
- `compiler/ocaml/` に最低限の README とディレクトリスケルトン（`src/`, `tests/`, `docs/`）を作成。
- `runtime/native/` と `tooling/cli|ci|release|lsp` にも README と TODO を配置し、参照計画書 (`1-5`, `1-6`, `1-7`, `4-2`, `3-6` など) へのリンクを明記。
- ブートストラップ計画書 (`docs/plans/bootstrap-roadmap/`) から該当タスクを引用し、各 README に TODO を列挙。

### Phase 6: リンク検証とレビュー (2〜3日)
- `grep -R "../[0-5]-" docs` などで旧パスが残っていないか確認。
- ローカルで `markdown-link-check` または `lychee --offline` を実行（導入手順を `docs/notes/tooling.md` などに記録）。
- レビュアからのフィードバックに対応し、移行ログ（`docs-migrations.log`）へ概要を追記。

## 7. 重要文書の改訂要件
| 文書 | 改訂ポイント |
|------|---------------|
| `README.md` | 実装リポジトリとしてのトップページ化。仕様目次は `docs/README.md` へ移行。 |
| `docs/README.md` | 旧 README 内容を移植し、新パスでのリンクを再生成。章ごとに簡易説明を追加。 |
| `AGENTS.md` | 「ドキュメント専用」前提の撤廃。コード編集・テストポリシー・依存環境の取得方針を追加。 |
| `CLAUDE.md` | Claude Code 専用の作業ガイドを再編し、新ディレクトリ構成での参照パスを明示。 |
| `compiler/README.md` | Phase 1〜3 の実装範囲（OCaml ブートストラップ／セルフホスト）とディレクトリ構造を整理。 |
| `runtime/README.md` | 最小ランタイムとターゲット別拡張計画を紐付け、`docs/plans/bootstrap-roadmap/1-5-runtime-integration.md` などの参照を明記。 |
| `tooling/README.md` | CLI・CI・リリース・LSP 等の資産配置方針を記載し、各サブディレクトリの役割と参照計画書を列挙。 |
| `docs/plans/bootstrap-roadmap/*.md` | 仕様パス変更に伴うリンク更新、および実装ディレクトリへの参照追加。 |
| `docs/notes/*` | 参照リンク修正のほか、新構成に合わせたタグ・メタデータ欄を整備。 |

## 8. チェックリスト
- [x] `git ls-files "0-*.md" "1-*.md" ...` が空になる（移設済み）
- [ ] `grep -R "../0-" -n` などで旧相対パスが残っていない
- [x] `README.md` から `docs/spec/` への導線が確認できる
- [x] `AGENTS.md` / `CLAUDE.md` が新構成を説明している
- [x] `compiler/README.md`・`runtime/README.md`・`tooling/README.md` で各フェーズ計画との紐付けが明記されている
- [x] `examples/` が README と連動し、サンプルのビルド手順（必要に応じて）が明記されている
- [ ] 実装計画（Phase 1〜4）が `compiler/` / `runtime/` README から追跡できる

## 9. リスクと対策
| リスク | 対応策 |
|--------|--------|
| 大量リンク更新による漏れ | 自動置換後に `grep` / `markdown-link-check` を実行。リンク切れ検出用スクリプトを CI に追加する。 |
| 履歴の追跡が困難になる | `git mv` を活用し、移動履歴を保持。主要移行は `docs-migrations.log` に概要を記録。 |
| エージェント指針が古いまま残る | Phase 3 完了前に `AGENTS.md` / `CLAUDE.md` を改訂し、旧構成への参照は削除またはリダイレクト記載。 |
| `tooling/` 配下の責務が混在する | 各サブディレクトリに README を配置し、対応する計画書（1-6, 1-7, 4-2 等）へのリンクを明記して用途を固定。 |
| 実装ディレクトリが空のままになる | 最低限の README と TODO を用意し、Phase 5 を他フェーズと並行して進める。 |
| 移行期間中の PR が競合する | 移行開始前に告知し、`docs-migrations.log` で段階別の完了タイミングを共有。 |

## 10. フォローアップ課題
- `docs/` 配下の章ごとに `index.md` を追加し、小項目の自動生成（目次）をサポートする。
- 文書内の脚注形式（`【F:path†Lx-Ly】` 等）について、新パスに合わせた変換規約を整備する。
- 実装コード導入後のテスト戦略・CI 設計を `tooling/` 配下で定義する（例: `tooling/ci/README.md`）。
- 旧構成を前提とした第三者資料（ブログ等）に告知するためのリリースノートを `docs/notes/` に作成。
- 追加のメタドキュメント（例: `CONTRIBUTING.md`, `CODE_OF_CONDUCT.md`）を配置する場所を検討し、コミュニティ拡張に備える。

---
本計画は Phase 0 の準備が完了した段階でレビューし、必要に応じてタイムラインや担当者をアサインしてください。
