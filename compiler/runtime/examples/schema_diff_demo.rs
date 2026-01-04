use reml_runtime::data::{
    Field, Schema, SchemaDataType, SchemaDiff, SchemaVersion, ValidationRule, ValidationRuleKind,
    ValidationRuleSeverity,
};
use serde_json::json;

fn make_schema_v1() -> Schema {
    Schema::builder("CoreConfig")
        .version(SchemaVersion::new(1, 0, 0))
        .field(
            Field::builder("endpoint", SchemaDataType::String)
                .description("Config service endpoint")
                .finish(),
        )
        .field(
            Field::builder("retries", SchemaDataType::Integer)
                .required(false)
                .default_value(json!(3))
                .finish(),
        )
        .finish()
}

fn make_schema_v2() -> Schema {
    Schema::builder("CoreConfig")
        .version(SchemaVersion::new(1, 1, 0))
        .field(
            Field::builder("endpoint", SchemaDataType::String)
                .description("Config service endpoint (https only)")
                .rule(
                    ValidationRule::builder(
                        "config.endpoint.scheme",
                        ValidationRuleKind::Regex {
                            pattern: "^https://".into(),
                        },
                    )
                    .severity(ValidationRuleSeverity::Error)
                    .message("https:// で始まる URL を指定してください")
                    .finish(),
                )
                .finish(),
        )
        .field(
            Field::builder("retries", SchemaDataType::Integer)
                .required(false)
                .default_value(json!(5))
                .finish(),
        )
        .field(
            Field::builder("timeout_ms", SchemaDataType::Integer)
                .required(false)
                .default_value(json!(1500))
                .finish(),
        )
        .finish()
}

fn main() {
    let legacy = make_schema_v1();
    let current = make_schema_v2();
    let diff = SchemaDiff::between(&legacy, &current);
    println!("# Schema Diff Demo");
    println!();
    println!("この出力は `compiler/runtime/examples/schema_diff_demo.rs` で生成しています。");
    println!("差分は JSON として記録されるため、監査レポートにそのまま貼り付けできます。");
    println!();
    println!("```json");
    println!(
        "{}",
        serde_json::to_string_pretty(&diff).expect("schema diff json")
    );
    println!("```");
}
