use serde::Deserialize;

#[derive(Deserialize, Clone, Debug)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ContractSource {
    Github { base_url: String, repo: String },
}
