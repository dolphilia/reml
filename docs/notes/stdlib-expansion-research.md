# Reml 標準ライブラリ拡張の調査

## 1. はじめに

本文書は、Reml を実用的で「一人前（full-fledged）」のプログラミング言語にするための、標準ライブラリ拡張に関する調査と提案をまとめたものです。現在の Reml 標準ライブラリ（`Core.*`）は、言語実装ドメイン（パーサー、診断、テキスト、ソースコード処理）に重点を置いていますが、汎用言語としてウェブサーバー、システムツール、データ処理アプリケーションなどのタスクに対応するためには、より広範なユーティリティが必要です。

## 2. 標準ライブラリの調査

人気のある「バッテリー同梱（batteries-included）」型言語（Python, Go）やシステム言語（Rust）との標準ライブラリ機能の比較。

| 機能ドメイン | Reml (現状) | Python | Go | Rust (std + 公式/事実上の標準) |
| :--- | :--- | :--- | :--- | :--- |
| **コレクション** | `Core.Collections` (Map, Set, List) | `list`, `dict`, `set`, `collections` | `slice`, `map` | `std::collections` |
| **テキスト/Unicode** | `Core.Text` (Grapheme, Unicode) | `str` (Unicode), `re` | `strings`, `unicode/utf8` | `String`, `regex` (crate) |
| **ファイル/IO** | `Core.Io`, `Core.Path` | `os`, `pathlib`, `io` | `os`, `io`, `path/filepath` | `std::fs`, `std::io`, `std::path` |
| **ネットワーク** | **欠落** | `socket`, `http`, `urllib`, `asyncio` | `net`, `net/http` | `std::net`, `reqwest/hyper` (crates) |
| **暗号化** | **欠落** | `hashlib`, `secrets`, `ssl` | `crypto/sha256`, `crypto/rand` | `ring`, `rust-crypto` (crates) |
| **圧縮** | **欠落** | `gzip`, `zipfile`, `tarfile` | `compress/gzip`, `archive/zip` | `flate2`, `tar` (crates) |
| **プロセス/OS** | Plugin (`4-2`) / `Core.Env` | `subprocess`, `os`, `sys` | `os/exec`, `os` | `std::process`, `std::env` |
| **エンコーディング** | `Core.Config` (JSON/TOML), Base64 in Text | `json`, `csv`, `base64` | `encoding/json`, `encoding/csv` | `serde` (crate), `base64` (crate) |
| **日付/時間** | `Core.Numeric`/`Core.Time` | `datetime`, `time`, `zoneinfo` | `time` | `std::time`, `chrono` (crate) |
| **数学** | `Core.Numeric` (基本のみ) | `math`, `random`, `decimal` | `math`, `math/rand` | `std` (basic), `rand` (crate) |

## 3. ギャップ分析

### 3.1 ネットワーク (`Core.Net`)
**ステータス**: 完全に欠落。
**要件**: Webサービス、APIと対話するツール、またはネットワークアプリケーションを構築するために、Remlには以下が必要です：
- **HTTP クライアント/サーバー**: リクエストとコンテンツ提供のための高レベルAPI。
- **低レベルソケットアクセス**: 低レベルプロトコルのためのTCP/UDP。
- **非同期統合**: `Core.Async` と統合されている必要があります。

### 3.2 暗号化 (`Core.Crypto`)
**ステータス**: 完全に欠落。
**要件**: セキュリティ、認証、データの整合性に不可欠です。
- **ハッシュ**: SHA256, SHA512 など。
- **乱数**: 暗号論的擬似乱数生成器 (CSPRNG)。
- **暗号化**: 基本的な対称鍵（AES）および潜在的に非対称鍵のサポート。

### 3.3 圧縮・アーカイブ (`Core.Archive`)
**ステータス**: 欠落。
**要件**: パッケージングツール、データ処理、ネットワーク転送の最適化に必要です。
- **アルゴリズム**: Gzip, Deflate, Zstd (現代の標準)。
- **フォーマット**: Zip, Tar。

### 3.4 OSインタラクション (`Core.System` / `Core.Process`)
**ステータス**: `Core.Env` は存在。`Core.Process` はオプションのプラグイン（`4-2`）として定義されている。
**要件**: 汎用言語として、サブプロセスの生成は単なるプラグインではなくコア機能であるべきです。シグナル、パイプ、プロセスライフサイクルの制御は極めて重要です。

### 3.5 エンコーディング (`Core.Encoding`)
**ステータス**: 散在している。Base64 は `Core.Text` に、JSON/TOML は `Core.Config` にある。
**要件**: データフォーマット専用の名前空間。
- **CSV**: データサイエンスや交換に不可欠。
- **URL**: クエリパラメータのエンコード/デコード（多くの場合 Net の一部）。
- **Hex**: バイナリから16進数文字列への変換。
- **統一インターフェース**: ストリーミングエンコード/デコードに `Reader`/`Writer` トレイトを使用。

### 3.6 データベース (`Core.Sql`)
**ステータス**: 欠落。
**要件**: 完全なドライバは通常外部にありますが、バックエンドを交換可能にするための標準インターフェース（Goの `database/sql` や Javaの JDBC のようなもの）が必要です。

## 4. 標準ライブラリ拡張の提案

Reml 標準ライブラリトラックに以下のモジュールを追加することを提案します。

### 4.1 `Core.Net`
**効果**: `effect {net}` (`io`, `io.async` を包含)
**サブモジュール**:
- `Core.Net.Http`: `Client`, `Server`, `Request`, `Response`。
- `Core.Net.Tcp`: `TcpListener`, `TcpStream`。
- `Core.Net.Udp`: `UdpSocket`。
- `Core.Net.Url`: `Url` パーサー/ビルダー。

### 4.2 `Core.Crypto`
**効果**: `effect {crypto}` (計算), `effect {random}`
**サブモジュール**:
- `Core.Crypto.Hash`: `Sha256`, `Sha512`, `Blake3`。
- `Core.Crypto.Random`: `secure_random_bytes`, `random_u64`。
- `Core.Crypto.Cipher` (オプション/将来): AES-GCM, ChaCha20。

### 4.3 `Core.Archive`
**効果**: `effect {io}` (ストリーミング)
**サブモジュール**:
- `Core.Archive.Gzip`: `GzipEncoder`, `GzipDecoder`。
- `Core.Archive.Zip`: `ZipReader`, `ZipWriter`。
- `Core.Archive.Tar`: `TarReader`, `TarWriter`。

### 4.4 `Core.Encoding`
**効果**: `@pure` (主に) または `effect {io}` (ストリーミング用)
**サブモジュール**:
- `Core.Encoding.Csv`: `Reader`, `Writer`。
- `Core.Encoding.Base64`: Text から移動/エイリアス。
- `Core.Encoding.Hex`: Hex ダンプ/ロード。
- `Core.Encoding.Json`: `Core.Config` へのエイリアス/参照、または特化したデータバインディング。

### 4.5 `Core.System` (`Core.Env` と `Core.Process` の統合)
**効果**: `effect {process}, effect {system}`
**変更点**:
- `4-2-process-plugin` を標準ライブラリの `Core.System.Process` 配下に昇格させる。
- `Core.Env` を `Core.System.Env` に統合する（または `Core.Env` エイリアスとして維持する）。

### 4.6 `Core.Math` (`Core.Numeric` の拡張)
**効果**: `@pure`
**機能**:
- 三角関数 (`sin`, `cos`, `tan`)。
- 対数 (`log`, `ln`)。
- 定数 (`PI`, `E`)。
- PRNG (非暗号ランダム) を `Core.Math.Random` に。

## 5. 実装戦略

1.  **Rust クレートの統合**: Reml の公式ランタイムは Rust ベースです。これらのモジュールは、パフォーマンスと信頼性を確保するために、可能な限り成熟した Rust クレートに直接マッピングすべきです。
    - `Core.Net` -> `reqwest`, `tokio`
    - `Core.Crypto` -> `ring`
    - `Core.Archive` -> `flate2`, `tar`, `zip`
    - `Core.Encoding` -> `csv`, `serde_json`, `base64`

2.  **監査と安全性**:
    - すべての IO/Net/Process 操作は `Core.Diagnostics` および `AuditSink` と統合する必要があります。
    - `effect` システムタグは厳密に強制されなければなりません。`effect {net}` は一般的な `effect {io}` と区別し、きめ細かな Capability 制御（例：「ファイル IO は許可するがネットワークは拒否」）を可能にする必要があります。

3.  **クロスプラットフォーム**:
    - API は `Core.Path` を受け入れ、OS の差異（Windows パス vs POSIX）を処理する必要があります。
    - `Core.System` は、シグナルやプロセス属性のための抽象インターフェースを提供する必要があります。

## 6. 次のステップ

1.  基盤的な可用性のブロッカーであるため、`Core.Net` と `Core.Crypto` を優先する。
2.  `Core.Net` の詳細な仕様をドラフトする（`Core.Io` スタイルに従う）。
3.  `Core.Process` について「プラグイン vs 標準ライブラリ」の境界を再検討する - 通常、プロセス制御は実用性を謳う言語にとっては「標準」と見なされます。

## 7. 追加の提案と検討事項

提案の初期ドラフトを見直した結果、以下の機能領域も「一人前の言語」として欠かせない要素であると考えられます。これらについても追加で検討すべきです。

### 7.1 `Core.Sync` (並行処理プリミティブ)
**ステータス**: `Core.Async` (言語機能) は計画されているが、具体的な同期プリミティブの仕様が見当たらない。
**要件**: マルチスレッドや非同期タスク間の安全なデータ共有と調整。
- **Primitives**: `Mutex`, `RwLock`, `Condvar`。
- **Channels**: `mpsc` (Multi-Producer, Single-Consumer), `broadcast`。
- **Atomics**: `AtomicInt`, `AtomicBool` 等のロックフリー操作。
- **方針**: Rust の `std::sync` および `tokio::sync` をモデルにし、同期・非同期の両方で使えるインターフェース（あるいは明確な分離）を提供する。

### 7.2 `Core.Log` (アプリケーションロギング)
**ステータス**: `Core.Diagnostics` はあるが、これは主にコンパイラ診断や監査ログ向けであり、構造が重厚である。
**要件**: アプリケーション開発者が手軽に実行時情報を出力するための軽量ファサード。
- **Levels**: `Debug`, `Info`, `Warn`, `Error`。
- **Interface**: `log.info("Server started port={}", port)` のような直感的な API。
- **統合**: バックエンドとして `Core.Diagnostics` や `AuditSink` に接続可能にしつつ、デフォルトでは標準出力/標準エラーへ構造化テキストを出力する。

### 7.3 `Core.Encoding` と `Core.Data` の責務整理
**課題**: 本提案の `Core.Encoding` (JSON/CSV) と、既存計画 ([3-7](3-7-core-config-data.md)) の `Core.Data` には機能的な重複の可能性がある。
**提案**:
- **`Core.Encoding`**: 低レベルな「バイト列 ⇔ データ構造」の変換（Codec）に集中する（例: `JsonEncoder`, `Base64Decoder`）。ストリーミング処理を主眼に置く。
- **`Core.Data`**: 高レベルな「データモデリング・検証・操作」に集中する（例: スキーマ定義、バリデーション、DataFrame的な操作）。
- **連携**: `Core.Data` のモデルを `Core.Encoding` でシリアライズする、という階層構造を明確にする。

### 7.4 `Core.System.Daemon` (サービス化サポート)
**ステータス**: 欠落。
**要件**: 長時間実行されるサーバーアプリケーション（デーモン）の実装サポート。
- **機能**: PIDファイル管理、デーモン化（ダブルフォーク等）、シグナルハンドリングの簡易ラッパー、正常なシャットダウンフック。
- **位置づけ**: `Core.System` のサブモジュールとして検討。
