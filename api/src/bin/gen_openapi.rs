//! Refresh `api/openapi.yaml` from the live source.
//!
//! Invoked by `make openapi` whenever a route or schema changes. The
//! snapshot is committed; the `openapi_yaml_snapshot_matches` test fails CI
//! if it falls out of sync.

fn main() {
    let yaml = ame_api::http::openapi::openapi_yaml();
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/openapi.yaml");
    std::fs::write(path, &yaml).expect("write openapi.yaml");
    eprintln!("wrote {} bytes to {path}", yaml.len());
}
