use std::{fs, path::Path};

#[test]
fn public_repository_contains_no_secret_or_runtime_boundaries() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let source = fs::read_to_string(root.join("src/lib.rs")).expect("source");
    for forbidden in [
        "DATABASE_URL",
        "SUPABASE_SERVICE_ROLE",
        "reqwest",
        "sea_orm",
        "sqlx",
        "tokio::net",
        "std::env::var",
        "Authorization: Bearer",
    ] {
        assert!(
            !source.contains(forbidden),
            "forbidden public boundary: {forbidden}"
        );
    }
}
