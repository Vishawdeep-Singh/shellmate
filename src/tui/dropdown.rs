//! What the `/` dropdown shows for the text typed so far.
//!
//! - `/` or `/lo`       built-in commands and skills whose name starts with what was typed
//! - `/model` + space   models whose id or name contains every word typed after it

use crate::models::ModelInfo;

/// Commands built into the UI, listed before skills.
const COMMANDS: [(&str, &str); 1] = [("model", "Switch to another model")];

const MODEL_COMMAND: &str = "/model ";

/// A skill offered in the dropdown.
pub struct SkillInfo {
    pub name: String,
    pub description: String,
}

/// The model list is fetched in the background when the UI starts.
pub enum ModelList {
    Loading,
    Ready(Vec<ModelInfo>),
    Failed(String),
}

pub struct Dropdown {
    pub title: String,
    pub rows: Vec<Row>,
}

/// One line in the dropdown, and what choosing it does.
pub struct Row {
    pub label: String,
    pub detail: String,
    pub pick: Pick,
}

#[derive(Debug, PartialEq)]
pub enum Pick {
    /// Put this text in the input box, ready for arguments.
    Complete(String),
    /// Switch to this model.
    Model(String),
    /// An information line, like "loading models…". Choosing it does nothing.
    Nothing,
}

impl Row {
    fn info(text: impl Into<String>) -> Self {
        Row {
            label: text.into(),
            detail: String::new(),
            pick: Pick::Nothing,
        }
    }
}

/// The dropdown for `input`, or `None` when it should be closed.
pub fn build(
    input: &str,
    skills: &[SkillInfo],
    models: &ModelList,
    current_model: &str,
) -> Option<Dropdown> {
    if let Some(filter) = input.strip_prefix(MODEL_COMMAND) {
        return Some(model_dropdown(filter, models, current_model));
    }

    let typed = input.strip_prefix('/')?;
    if typed.contains(char::is_whitespace) {
        return None;
    }

    let commands = COMMANDS
        .iter()
        .filter(|(name, _)| name.starts_with(typed))
        .map(|(name, about)| Row {
            label: format!("/{name}"),
            detail: about.to_string(),
            pick: Pick::Complete(format!("/{name} ")),
        });
    let skills = skills
        .iter()
        .filter(|skill| skill.name.starts_with(typed))
        .map(|skill| Row {
            label: format!("/{}", skill.name),
            detail: skill.description.clone(),
            pick: Pick::Complete(format!("/{} ", skill.name)),
        });

    let rows: Vec<Row> = commands.chain(skills).collect();
    if rows.is_empty() {
        return None;
    }
    Some(Dropdown {
        title: " commands ".to_string(),
        rows,
    })
}

fn model_dropdown(filter: &str, models: &ModelList, current_model: &str) -> Dropdown {
    let rows = match models {
        ModelList::Loading => vec![Row::info("loading models…")],
        ModelList::Failed(error) => vec![Row::info(format!("could not load models: {error}"))],
        ModelList::Ready(models) => {
            let words: Vec<String> = filter.split_whitespace().map(str::to_lowercase).collect();
            let rows: Vec<Row> = models
                .iter()
                .filter(|model| matches_every_word(model, &words))
                .map(|model| model_row(model, current_model))
                .collect();
            if rows.is_empty() {
                vec![Row::info("no model matches")]
            } else {
                rows
            }
        }
    };
    Dropdown {
        title: " models · type to filter ".to_string(),
        rows,
    }
}

fn matches_every_word(model: &ModelInfo, words: &[String]) -> bool {
    let haystack = format!("{} {}", model.id, model.name).to_lowercase();
    words.iter().all(|word| haystack.contains(word.as_str()))
}

fn model_row(model: &ModelInfo, current_model: &str) -> Row {
    let mut detail = model.name.clone();
    if let Some(tokens) = model.context_length {
        detail.push_str(&format!(" · {}k context", tokens / 1000));
    }
    if model.id == current_model {
        detail.push_str(" · current");
    }
    Row {
        label: model.id.clone(),
        detail,
        pick: Pick::Model(model.id.clone()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn skills() -> Vec<SkillInfo> {
        ["apple", "loc", "lint"]
            .into_iter()
            .map(|name| SkillInfo {
                name: name.to_string(),
                description: format!("The {name} skill"),
            })
            .collect()
    }

    fn models() -> ModelList {
        let model = |id: &str, name: &str| -> ModelInfo {
            serde_json::from_value(serde_json::json!({
                "id": id,
                "name": name,
                "context_length": 200_000
            }))
            .unwrap()
        };
        ModelList::Ready(vec![
            model("anthropic/claude-haiku-4.5", "Anthropic: Claude Haiku 4.5"),
            model(
                "anthropic/claude-sonnet-4.5",
                "Anthropic: Claude Sonnet 4.5",
            ),
            model("openai/gpt-5", "OpenAI: GPT-5"),
        ])
    }

    fn labels(input: &str, models: &ModelList) -> Vec<String> {
        build(input, &skills(), models, "anthropic/claude-haiku-4.5")
            .map(|dropdown| dropdown.rows.into_iter().map(|row| row.label).collect())
            .unwrap_or_default()
    }

    #[test]
    fn slash_lists_commands_then_skills() {
        assert_eq!(
            labels("/", &models()),
            ["/model", "/apple", "/loc", "/lint"]
        );
    }

    #[test]
    fn typing_narrows_the_list() {
        assert_eq!(labels("/l", &models()), ["/loc", "/lint"]);
    }

    #[test]
    fn closed_without_a_slash_or_after_arguments() {
        assert!(labels("loc", &models()).is_empty());
        assert!(labels("/loc src", &models()).is_empty());
    }

    #[test]
    fn model_command_lists_every_model() {
        assert_eq!(
            labels("/model ", &models()),
            [
                "anthropic/claude-haiku-4.5",
                "anthropic/claude-sonnet-4.5",
                "openai/gpt-5"
            ]
        );
    }

    #[test]
    fn model_filter_needs_every_word_in_id_or_name() {
        assert_eq!(
            labels("/model claude SON", &models()),
            ["anthropic/claude-sonnet-4.5"]
        );
    }

    #[test]
    fn current_model_is_marked() {
        let dropdown = build(
            "/model haiku",
            &skills(),
            &models(),
            "anthropic/claude-haiku-4.5",
        )
        .unwrap();

        assert_eq!(
            dropdown.rows[0].detail,
            "Anthropic: Claude Haiku 4.5 · 200k context · current"
        );
    }

    #[test]
    fn information_rows_while_loading_or_without_matches() {
        assert_eq!(labels("/model ", &ModelList::Loading), ["loading models…"]);
        assert_eq!(labels("/model zzz", &models()), ["no model matches"]);
    }
}
