//! The list of models the user can switch to with `/model`.
//!
//! OpenRouter's `/models` endpoint returns every model it offers, with more fields than
//! OpenAI's format, so it is read into our own types (async-openai's "bring your own type").

use anyhow::Result;
use async_openai::{Client, config::OpenAIConfig};
use serde::Deserialize;

pub const DEFAULT_MODEL: &str = "anthropic/claude-haiku-4.5";

#[derive(Debug, Clone, Deserialize)]
pub struct ModelInfo {
    /// What the API expects, like `anthropic/claude-sonnet-4.5`.
    pub id: String,
    /// A human name, like `Anthropic: Claude Sonnet 4.5`.
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub context_length: Option<u64>,
    #[serde(default)]
    supported_parameters: Vec<String>,
}

impl ModelInfo {
    /// shellmate needs tool calling. A provider that does not list its parameters
    /// gets the benefit of the doubt.
    fn supports_tools(&self) -> bool {
        self.supported_parameters.is_empty()
            || self.supported_parameters.iter().any(|p| p == "tools")
    }
}

#[derive(Deserialize)]
struct ModelList {
    data: Vec<ModelInfo>,
}

/// Every model that can call tools, sorted by id.
pub async fn fetch(client: &Client<OpenAIConfig>) -> Result<Vec<ModelInfo>> {
    let list: ModelList = client.models().list_byot().await?;
    Ok(usable(list.data))
}

fn usable(models: Vec<ModelInfo>) -> Vec<ModelInfo> {
    let mut models: Vec<ModelInfo> = models
        .into_iter()
        .filter(ModelInfo::supports_tools)
        .collect();
    models.sort_by(|a, b| a.id.cmp(&b.id));
    models
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_tool_models_sorted_and_drops_the_rest() {
        let json = r#"{"data": [
            {"id": "z/tools", "name": "Z", "context_length": 8000, "supported_parameters": ["tools", "temperature"]},
            {"id": "b/no-tools", "name": "B", "supported_parameters": ["temperature"]},
            {"id": "a/unknown", "name": "A"}
        ]}"#;
        let list: ModelList = serde_json::from_str(json).unwrap();

        let ids: Vec<String> = usable(list.data)
            .into_iter()
            .map(|model| model.id)
            .collect();

        assert_eq!(ids, ["a/unknown", "z/tools"]);
    }
}
