/// Utility binary: prints the JSON Schema for githops.yaml to stdout.
/// Run via:  cargo run --bin gen-schema > githops-core/githops.schema.json
fn main() {
    let schema = schemars::schema_for!(githops::config::Config);
    println!("{}", serde_json::to_string_pretty(&schema).unwrap());
}
