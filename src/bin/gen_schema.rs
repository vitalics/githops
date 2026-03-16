/// Utility binary: prints the JSON Schema for githops.yaml to stdout.
/// Run via:  cargo run --bin gen-schema 2>/dev/null > githops-core/githops.schema.json
fn main() {
    let mut schema = schemars::schema_for!(githops::config::Config);
    // Embed the githops version that generated this schema so editors and
    // `githops schema sync` can surface whether the schema is stale.
    schema.schema.extensions.insert(
        "x-githops-version".to_string(),
        serde_json::json!(env!("CARGO_PKG_VERSION")),
    );
    println!("{}", serde_json::to_string_pretty(&schema).unwrap());
}
