# 4.2 マルチターゲットリリースパイプライン計画

## 目的
- Phase 4 マイルストーン M2 に合わせ、Linux/Windows/macOS 向けの自動ビルド・テスト・署名・配布パイプラインを構築し、正式リリースを支える。
- `5-3-developer-toolchain.md` の手順を具体化し、リリースノートと成果物の配布を標準化する。

## スコープ
- **含む**: CI/CD 構成、成果物署名、圧縮形式統一、配布ポータル更新、ログ記録。
- **含まない**: 新規ターゲット追加、商用配布インフラ。必要な場合は別計画。
- **前提**: セルフホストビルドが安定し、主要ターゲットの成果物が生成できる。

## 作業ブレークダウン
1. **CI/CD 設計**: GitHub Actions もしくは他の CI を利用し、ビルド→テスト→署名→配布のステージを定義。
2. **署名手続き**: Linux (GPG)、Windows (コードサイニング証明書)、macOS (codesign + notarytool) の運用手順を整備し、自動ワークフロー化。
3. **成果物整備**: アーカイブ形式 (`.tar.gz`, `.zip`) とファイル構成を統一し、ターゲット別 Readme を同梱。
4. **配布チャネル更新**: ダウンロードページ・レジストリ (`5-2-registry-distribution.md`) を更新し、成果物とハッシュを掲載。
5. **リリースノート作成**: `5-5-roadmap-metrics.md` に基づくテンプレートでリリースノートを生成。
6. **監査ログ**: リリースごとの署名情報・チェックサムを `0-3-audit-and-metrics.md` に記録し、検証手順を提示。

## 成果物と検証
- 3 ターゲット全てのアーティファクトが自動生成され、署名検証が通る。
- 配布ページが更新され、ユーザーがダウンロード・検証できる。
- CI/CD ログが保存され、再現手順がドキュメント化される。

## リスクとフォローアップ
- 署名用証明書の更新・管理がリスクとなるため、キーローテーション手順をドキュメント化。
- Notarization など外部サービスの障害に備え、リトライ戦略と SLA を `0-4-risk-handling.md` に明記。
- 成果物サイズが増大する場合、圧縮や不要ファイル除外を検討。

## 参考資料
- [4-0-phase4-migration.md](4-0-phase4-migration.md)
- [5-3-developer-toolchain.md](../../5-3-developer-toolchain.md)
- [5-2-registry-distribution.md](../../5-2-registry-distribution.md)
- [5-5-roadmap-metrics.md](../../5-5-roadmap-metrics.md)
- [0-3-audit-and-metrics.md](0-3-audit-and-metrics.md)
- [0-4-risk-handling.md](0-4-risk-handling.md)

