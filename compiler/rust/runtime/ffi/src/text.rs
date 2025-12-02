//! FFI ビルド時の `Core.Text` 監査スタブ。
//!
//! ランタイム本体の `text` モジュールを直接共有するには、
//! Unicode 依存や IO 実装を丸ごと取り込む必要があるため、
//! FFI 層では最小限の `take_text_audit_metadata` だけを提供する。
//! 現時点では Text API からの監査メタデータは生成されないため、
//! `None` を返すプレースホルダとして実装している。

use serde_json::{Map as JsonMap, Value};

/// Text API が記録した監査メタデータを取り出す。
/// FFI ビルドでは Text 経路を提供していないため常に `None`。
pub fn take_text_audit_metadata() -> Option<JsonMap<std::string::String, Value>> {
    None
}
