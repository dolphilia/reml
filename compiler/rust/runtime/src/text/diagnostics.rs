use serde_json::{json, Map as JsonMap, Value};

use super::GraphemeStats;

/// `text.grapheme_stats` メタデータオブジェクトを生成する。
pub fn grapheme_stats_metadata(stats: &GraphemeStats) -> JsonMap<String, Value> {
    let mut info = JsonMap::new();
    info.insert("length".into(), json!(stats.grapheme_count));
    info.insert("bytes".into(), json!(stats.total_bytes));
    info.insert("total_display_width".into(), json!(stats.total_display_width));
    info.insert("avg_width".into(), json!(stats.avg_width));
    info.insert("emoji_ratio".into(), json!(stats.emoji_ratio));
    info.insert(
        "primary_script".into(),
        Value::String(stats.scripts.primary.label().into()),
    );
    info.insert("primary_ratio".into(), json!(stats.scripts.primary_ratio));
    info.insert("script_mix_ratio".into(), json!(stats.scripts.mix_ratio));
    info.insert("rtl_ratio".into(), json!(stats.direction.rtl_ratio));
    info.insert("cache_hits".into(), json!(stats.cache_hits));
    info.insert("cache_miss".into(), json!(stats.cache_miss));
    info.insert("cache_generation".into(), json!(stats.cache_generation));
    info.insert("cache_version".into(), json!(stats.cache_version));
    info.insert(
        "version_mismatch_evictions".into(),
        json!(stats.version_mismatch_evictions),
    );
    info.insert(
        "unicode_version".into(),
        Value::String(stats.unicode_version.clone()),
    );
    info.insert(
        "version".into(),
        Value::String(stats.unicode_version.clone()),
    );
    let denominator = (stats.cache_hits + stats.cache_miss) as f64;
    if denominator > 0.0 {
        info.insert(
            "cache_hit_ratio".into(),
            json!(stats.cache_hits as f64 / denominator),
        );
    }
    info
}

/// `AuditEnvelope.metadata["text.grapheme_stats"]` に統計情報を埋め込む。
pub fn insert_grapheme_stats_metadata(
    metadata: &mut JsonMap<String, Value>,
    stats: &GraphemeStats,
) {
    metadata.insert(
        "text.grapheme_stats".into(),
        Value::Object(grapheme_stats_metadata(stats)),
    );
}
