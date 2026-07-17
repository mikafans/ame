use std::{env, fs, path::PathBuf};

fn main() {
    let manifest = ame_api::http::agents::build_skill_manifest();
    let docs_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("api directory must have a repository parent")
        .join("docs/public");
    fs::create_dir_all(&docs_dir).expect("create docs/public");
    let output = serde_json::to_string_pretty(&manifest).expect("serialize skill manifest");
    fs::write(docs_dir.join("skill.json"), format!("{output}\n"))
        .expect("write docs/public/skill.json");
    fs::copy(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("openapi.yaml"),
        docs_dir.join("openapi.yaml"),
    )
    .expect("copy api/openapi.yaml to docs/public/openapi.yaml");
}
