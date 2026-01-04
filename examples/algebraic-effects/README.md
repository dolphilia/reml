# アルゴリズミック効果サンプル

このフォルダは Async/FFI ハンドラと Stage 運用の参考資料です。

## ファイル一覧
- `cli-commands.md`: stage 切り替え時の CLI コマンド例
- `capability-console.yaml`: Capability Registry 設定サンプル
- `audit-log.json`: 監査ログ出力例（effects 拡張付き）

## 利用方法
1. `cli-commands.md` に従って Experimental → Stable までの昇格シナリオを再現。
2. Capability 設定をプロジェクトの `runtime.cap.toml` へ取り込む。
3. `audit-log.json` をダッシュボードに取り込み、Experimental 呼び出しが残っていないか確認。
