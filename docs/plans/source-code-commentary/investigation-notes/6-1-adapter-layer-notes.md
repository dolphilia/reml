# 調査メモ: 第19章 Adapter レイヤ

## 対象モジュール

- `compiler/adapter/src/lib.rs`
- `compiler/adapter/src/capability.rs`
- `compiler/adapter/src/env.rs`
- `compiler/adapter/src/fs.rs`
- `compiler/adapter/src/network.rs`
- `compiler/adapter/src/process.rs`
- `compiler/adapter/src/random.rs`
- `compiler/adapter/src/target.rs`
- `compiler/adapter/src/time.rs`
- `compiler/adapter/README.md`
- `docs/plans/rust-migration/2-2-adapter-layer-guidelines.md`

## 入口と全体像

- `lib.rs` が Env/FS/Network/Time/Random/Process/Target の各サブシステムを公開する最小の入口。
  - `compiler/adapter/src/lib.rs:1-11`
- `capability.rs` はサブシステム共通の Capability と監査メタデータ生成を提供する基盤。
  - `compiler/adapter/src/capability.rs:3-65`
- README によると、目的はプラットフォーム差異の吸収と Capability/監査の共通抽象化。
  - `compiler/adapter/README.md:1-21`

## データ構造

- **AdapterCapability**: 各サブシステムの `id` / `stage` / `effect_scope` / `audit_key_prefix` を束ね、監査メタデータ生成に使う。
  - `compiler/adapter/src/capability.rs:3-65`
- **EnvOperation / EnvScope**: Env 操作種別と監査スコープを列挙。
  - `compiler/adapter/src/env.rs:5-38`
- **PlatformSnapshot / EnvContext**: 実行環境情報と Env 操作の文脈を保持。
  - `compiler/adapter/src/env.rs:41-97`
- **EnvError / EnvErrorKind / AdapterError**: Env 専用エラーと監査エラーを束ねる上位型。
  - `compiler/adapter/src/env.rs:99-128`
- **EnvMutationArgs / EnvMutationResult / EnvAuditHandle**: Env 変更の監査入力/結果/ハンドル。
  - `compiler/adapter/src/env.rs:130-192`
- **TargetProfile / RunConfigTarget / TargetInference**: 実行ターゲットの推論と RunConfig へ渡す表現。
  - `compiler/adapter/src/target.rs:10-193`

## コアロジック

- **監査メタデータ生成**: `AdapterCapability::audit_metadata` が `capability.*` と `adapter.*` の基本キーを整形する。
  - `compiler/adapter/src/capability.rs:28-64`
- **Env の読み書きと監査**: `get_env` は UTF-8 以外を `InvalidEncoding` とし、`set_env`/`remove_env` は監査ハンドルを通じて結果を通知する。`panic::catch_unwind` で `std::env` の panic を `IoFailure` に変換する。
  - `compiler/adapter/src/env.rs:194-303`
- **Env 入力検証**: `ensure_valid_key`/`ensure_valid_value` が NULL バイトと空キーを拒否する。
  - `compiler/adapter/src/env.rs:305-349`
- **ターゲット推論**: `infer_target_from_env` が `REML_TARGET_*` 系の環境変数を読み取り、`TargetProfile` を上書きする。`parse_bool` は文字列 bool を厳密に検証する。
  - `compiler/adapter/src/target.rs:274-371`
- **FS/Network/Process/Random/Time**: それぞれ Rust 標準ライブラリや `getrandom` に薄く委譲し、監査メタデータを生成するヘルパを持つ。
  - `compiler/adapter/src/fs.rs:11-39`
  - `compiler/adapter/src/network.rs:10-33`
  - `compiler/adapter/src/process.rs:10-33`
  - `compiler/adapter/src/random.rs:8-28`
  - `compiler/adapter/src/time.rs:7-35`

## エラー処理

- Env の失敗は `EnvErrorKind` で分類し、`AdapterError::Env` に集約する。
  - `compiler/adapter/src/env.rs:99-128`
- 監査トレイトの失敗は `AdapterError::Audit` でラップする。
  - `compiler/adapter/src/env.rs:289-301`
- `getrandom` のエラーは `io::ErrorKind::Other` に変換される。
  - `compiler/adapter/src/random.rs:18-23`
- `parse_bool` の無効値は `InvalidEncoding` として返し、`EnvContext::detect` を付与する。
  - `compiler/adapter/src/target.rs:360-369`

## 仕様との対応メモ

- Env のキー/値チェックは `docs/spec/3-10-core-env.md` の運用に相当しそうだが、Capability や audit の接続はアダプタ単体では未配線。
- `TargetProfile`/`RunConfigTarget` は `docs/spec/3-7-core-config-data.md` のターゲット拡張に関連しそうだが、詳細対応は要確認。
- `docs/plans/rust-migration/2-2-adapter-layer-guidelines.md` には FS/Network/Time/Random/Process を Capability 連携でまとめる設計がある。実装は最小ラッパに留まり、設計とのギャップ整理が必要。

## TODO / 不明点

- `AdapterCapability` で生成した `adapter.*` 監査メタデータがどのタイミングで `AuditEnvelope` に取り込まれるか、runtime 側の接続点を追跡する必要がある。
- `TargetInference` が CLI/Runtime のどの段で呼ばれるか（frontend/ runtime/ tooling か）を確認したい。
- FS/Network/Process などが Capability 判定や Stage ゲートを実際に通る設計は別層にあるため、章末で「未配線/最小実装」の扱いを整理する。
