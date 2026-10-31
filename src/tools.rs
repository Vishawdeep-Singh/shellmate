use anyhow::{Context, Result};
use schemars::{JsonSchema, schema_for};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::{
    agent::{AgentContext, agent_loop},
    api::Message,
    events::AgentEvent,
};

#[derive(Serialize, Deserialize, Clone)]
pub struct ToolCall {
    pub id: String,
    #[serde(rename = "type")]
    pub kind: String,
    pub function: FunctionCall,
}

#[derive(Deserialize, Serialize, Clone)]
pub struct FunctionCall {
    pub name: String,
    pub arguments: String,
}

/// Read and return the contents of a file
#[derive(Deserialize, JsonSchema)]
pub struct ReadInput {
    /// The path to the file to read
    pub file_path: String,
}

/// Write content to a file
#[derive(Deserialize, JsonSchema)]
pub struct WriteInput {
    /// The path of the file to write to
    pub file_path: String,
    /// The content to write to the file
    pub content: String,
}

/// Execute a shell command
#[derive(Deserialize, JsonSchema)]
pub struct BashInput {
    /// The command to execute
    pub command: String,
}

/// Execute a Skill
#[derive(Deserialize, JsonSchema)]
pub struct SkillInput {
    /// The name of the skill want to execute
    pub name: String,
    /// The optional arguments want to pass to the skill
    pub arguments: Option<String>,
}

#[derive(Deserialize)]
#[serde(tag = "name", content = "arguments")]
pub enum ToolInput {
    Read(ReadInput),
    Write(WriteInput),
    Bash(BashInput),
    Skill(SkillInput),
}

trait Run {
    async fn run(&self, ctx: &AgentContext) -> Result<String>;
}

impl Run for ReadInput {
    async fn run(&self, _ctx: &AgentContext) -> Result<String> {
        Ok(std::fs::read_to_string(&self.file_path)?)
    }
}
impl Run for WriteInput {
    async fn run(&self, _ctx: &AgentContext) -> Result<String> {
        if let Some(parent) = std::path::Path::new(&self.file_path).parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&self.file_path, &self.content)?;
        Ok(format!(
            "Wrote {} bytes to {}",
            self.content.len(),
            self.file_path
        ))
    }
}
impl Run for BashInput {
    async fn run(&self, _ctx: &AgentContext) -> Result<String> {
        let output = std::process::Command::new("sh")
            .arg("-c")
            .arg(&self.command)
            // No keyboard input: a command that waits for it would otherwise hang the UI.
            .stdin(std::process::Stdio::null())
            .output()?;
        let mut result = String::from_utf8_lossy(&output.stdout).into_owned();
        result.push_str(&String::from_utf8_lossy(&output.stderr));
        if !output.status.success() {
            result.push_str(&format!("\n(exit code: {:?})", output.status.code()));
        }
        Ok(result)
    }
}

impl Run for SkillInput {
    async fn run(&self, ctx: &AgentContext) -> Result<String> {
        let name = self.name.as_str();
        let skill = ctx
            .skills
            .iter()
            .find(|skill| skill.name() == Some(name))
            .with_context(|| format!("unknown skill: {name}"))?;
        let rendered = skill.render(self.arguments.as_deref().unwrap_or(""));

        if skill.context_fork() != Some("fork") {
            return Ok(rendered);
        }

        let sub_ctx = AgentContext {
            client: ctx.client.clone(),
            model: ctx.model.clone(),
            skills: Vec::new(),
            tool_definitions: subagent_tool_definitions(),
            depth: ctx.depth + 1,
            reporter: ctx.reporter.clone(),
        };
        ctx.reporter.report(AgentEvent::Forked { depth: ctx.depth });
        let mut messages = vec![Message::User { content: rendered }];
        let answer = Box::pin(agent_loop(&mut messages, &sub_ctx)).await?;

        let answer = answer.unwrap_or_default();

        Ok(format!(
            "Skill {name} ran in a separate context and returned: {}",
            answer.trim()
        ))
    }
}
impl ToolInput {
    pub fn from_call(call: &FunctionCall) -> Result<ToolInput> {
        let arguments: Value = serde_json::from_str(&call.arguments)?;
        Ok(serde_json::from_value(
            json!({ "name": call.name, "arguments": arguments }),
        )?)
    }
    pub async fn dispatch(&self, ctx: &AgentContext) -> Result<String> {
        match self {
            ToolInput::Read(args) => args.run(ctx).await,
            ToolInput::Write(args) => args.run(ctx).await,
            ToolInput::Bash(args) => args.run(ctx).await,
            ToolInput::Skill(args) => args.run(ctx).await,
        }
    }

    /// The main argument of the call, shown next to the tool name in the trace.
    fn detail(&self) -> String {
        match self {
            ToolInput::Read(args) => args.file_path.clone(),
            ToolInput::Write(args) => {
                format!("{} ({} bytes)", args.file_path, args.content.len())
            }
            ToolInput::Bash(args) => args.command.clone(),
            ToolInput::Skill(args) => match args.arguments.as_deref() {
                Some(arguments) if !arguments.trim().is_empty() => {
                    format!("{} {arguments}", args.name)
                }
                _ => args.name.clone(),
            },
        }
    }
}

impl ToolCall {
    /// Runs the call and returns the text sent back to the model.
    /// Reports when it starts and how it ended, so the user can follow along.
    pub async fn execute(&self, ctx: &AgentContext) -> String {
        let tool = self.function.name.clone();
        let depth = ctx.depth;

        let result = match ToolInput::from_call(&self.function) {
            Ok(input) => {
                ctx.reporter.report(AgentEvent::ToolStarted {
                    depth,
                    tool: tool.clone(),
                    detail: input.detail(),
                });
                input.dispatch(ctx).await
            }
            Err(e) => {
                ctx.reporter.report(AgentEvent::ToolStarted {
                    depth,
                    tool: tool.clone(),
                    detail: "(could not parse its arguments)".to_string(),
                });
                Err(e)
            }
        };

        match result {
            Ok(output) => {
                ctx.reporter.report(AgentEvent::ToolFinished {
                    depth,
                    tool,
                    output: output.clone(),
                });
                output
            }
            Err(e) => {
                let error = format!("error: {e:#}");
                ctx.reporter.report(AgentEvent::ToolFailed {
                    depth,
                    tool,
                    error: error.clone(),
                });
                error
            }
        }
    }
}

pub fn tool_definition<T: JsonSchema>(name: &str) -> Value {
    let schema = serde_json::to_value(schema_for!(T)).expect("schema serializes");
    json!({
        "type": "function",
        "function": {
            "name": name,
            "description": schema["description"],
            "parameters":schema
        }
    })
}

pub fn tool_definitions() -> Value {
    json!([
        tool_definition::<ReadInput>("Read"),
        tool_definition::<WriteInput>("Write"),
        tool_definition::<BashInput>("Bash"),
        tool_definition::<SkillInput>("Skill")
    ])
}

pub fn subagent_tool_definitions() -> Value {
    json!([
        tool_definition::<ReadInput>("Read"),
        tool_definition::<WriteInput>("Write"),
        tool_definition::<BashInput>("Bash"),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::Reporter;
    use crate::skills::Skill;
    use async_openai::{Client, config::OpenAIConfig};
    use std::collections::HashMap;
    use std::path::{Path, PathBuf};

    // Read, Write and Bash ignore the context; the client is never used to send anything.
    fn ctx_with(skills: Vec<Skill>) -> AgentContext {
        AgentContext {
            tool_definitions: tool_definitions(),
            skills,
            client: Client::with_config(OpenAIConfig::new()),
            model: "test-model".to_string(),
            depth: 0,
            reporter: Reporter::Stderr,
        }
    }

    fn ctx() -> AgentContext {
        ctx_with(Vec::new())
    }

    // Each test gets its own empty folder, so tests running in parallel never collide.
    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("tools-test-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn path_str(path: &Path) -> String {
        path.to_str().unwrap().to_string()
    }

    async fn bash(command: &str) -> String {
        ToolInput::Bash(BashInput {
            command: command.to_string(),
        })
        .dispatch(&ctx())
        .await
        .unwrap()
    }

    fn call(name: &str, arguments: &str) -> FunctionCall {
        FunctionCall {
            name: name.to_string(),
            arguments: arguments.to_string(),
        }
    }

    // ---------- Read ----------

    #[tokio::test]
    async fn read_returns_exact_contents() {
        let file = temp_dir("read").join("apple.py");
        std::fs::write(&file, "print('hi')\n\n  indented\n").unwrap();

        let output = ToolInput::Read(ReadInput {
            file_path: path_str(&file),
        })
        .dispatch(&ctx())
        .await
        .unwrap();

        assert_eq!(output, "print('hi')\n\n  indented\n");
    }

    #[tokio::test]
    async fn read_missing_file_is_an_error() {
        let file = temp_dir("read-missing").join("nope.txt");

        assert!(
            ToolInput::Read(ReadInput {
                file_path: path_str(&file)
            })
            .dispatch(&ctx())
            .await
            .is_err()
        );
    }

    // ---------- Write ----------

    #[tokio::test]
    async fn write_creates_new_file() {
        let file = temp_dir("write-new").join("hello.txt");

        let output = ToolInput::Write(WriteInput {
            file_path: path_str(&file),
            content: "hi there".to_string(),
        })
        .dispatch(&ctx())
        .await
        .unwrap();

        assert_eq!(std::fs::read_to_string(&file).unwrap(), "hi there");
        assert!(output.contains("8 bytes"));
    }

    #[tokio::test]
    async fn write_overwrites_existing_file() {
        let file = temp_dir("write-overwrite").join("hello.txt");
        std::fs::write(&file, "old contents that are longer").unwrap();

        ToolInput::Write(WriteInput {
            file_path: path_str(&file),
            content: "new".to_string(),
        })
        .dispatch(&ctx())
        .await
        .unwrap();

        assert_eq!(std::fs::read_to_string(&file).unwrap(), "new");
    }

    #[tokio::test]
    async fn write_creates_missing_parent_dirs() {
        let file = temp_dir("write-nested").join("a/b/c/deep.txt");

        ToolInput::Write(WriteInput {
            file_path: path_str(&file),
            content: "deep".to_string(),
        })
        .dispatch(&ctx())
        .await
        .unwrap();

        assert_eq!(std::fs::read_to_string(&file).unwrap(), "deep");
    }

    // ---------- Bash ----------

    #[tokio::test]
    async fn bash_runs_simple_command() {
        assert_eq!(bash("echo hello").await, "hello\n");
    }

    #[tokio::test]
    async fn bash_supports_pipes() {
        assert_eq!(bash("printf 'b\\na\\n' | sort").await, "a\nb\n");
    }

    #[tokio::test]
    async fn bash_includes_stderr() {
        let output = bash("echo out; echo oops >&2").await;

        assert!(output.contains("out"));
        assert!(output.contains("oops"));
    }

    #[tokio::test]
    async fn bash_reports_failure_exit_code() {
        assert!(bash("exit 3").await.contains("(exit code: Some(3))"));
    }

    #[tokio::test]
    async fn bash_success_has_no_exit_code() {
        assert!(!bash("true").await.contains("exit code"));
    }

    // ---------- parse ----------

    #[test]
    fn parse_read() {
        let tool = ToolInput::from_call(&call("Read", r#"{"file_path": "a.txt"}"#));

        assert!(matches!(tool, Ok(ToolInput::Read(args)) if args.file_path == "a.txt"));
    }

    #[test]
    fn parse_write() {
        let tool =
            ToolInput::from_call(&call("Write", r#"{"file_path": "a.txt", "content": "hi"}"#));

        assert!(matches!(
            tool,
            Ok(ToolInput::Write(args)) if args.file_path == "a.txt" && args.content == "hi"
        ));
    }

    #[test]
    fn parse_bash() {
        let tool = ToolInput::from_call(&call("Bash", r#"{"command": "ls"}"#));

        assert!(matches!(tool, Ok(ToolInput::Bash(args)) if args.command == "ls"));
    }

    #[test]
    fn parse_unknown_tool_is_an_error() {
        let error = ToolInput::from_call(&call("Delete", "{}"))
            .err()
            .expect("should fail");

        assert!(error.to_string().contains("Delete"));
    }

    #[test]
    fn parse_missing_field_is_an_error() {
        let error = ToolInput::from_call(&call("Read", r#"{"path": "a.txt"}"#))
            .err()
            .expect("should fail");

        assert!(error.to_string().contains("missing field"));
    }

    // ---------- execute ----------

    #[tokio::test]
    async fn execute_turns_errors_into_text() {
        let tool_call = ToolCall {
            id: "call_1".to_string(),
            kind: "function".to_string(),
            function: call("Delete", "{}"),
        };

        assert!(tool_call.execute(&ctx()).await.starts_with("error"));
    }

    // ---------- Skill ----------

    fn skill(name: &str, body: &str) -> Skill {
        Skill {
            frontmatter: HashMap::from([
                ("name".to_string(), name.to_string()),
                ("description".to_string(), format!("The {name} skill")),
            ]),
            body: body.to_string(),
            dir: PathBuf::from(format!(".claude/skills/{name}")),
        }
    }

    fn skill_args(name: &str, arguments: Option<&str>) -> SkillInput {
        SkillInput {
            name: name.to_string(),
            arguments: arguments.map(str::to_string),
        }
    }

    #[test]
    fn parse_skill_with_arguments() {
        let tool =
            ToolInput::from_call(&call("Skill", r#"{"name": "apple", "arguments": "4127"}"#));

        assert!(matches!(
            tool,
            Ok(ToolInput::Skill(args)) if args.name == "apple" && args.arguments.as_deref() == Some("4127")
        ));
    }

    #[test]
    fn parse_skill_without_arguments() {
        let tool = ToolInput::from_call(&call("Skill", r#"{"name": "apple"}"#));

        assert!(matches!(
            tool,
            Ok(ToolInput::Skill(args)) if args.name == "apple" && args.arguments.is_none()
        ));
    }

    #[tokio::test]
    async fn skill_renders_header_and_body_with_arguments() {
        let ctx = ctx_with(vec![skill("apple", "Deploy apple to $ARGUMENTS")]);

        let output = skill_args("apple", Some("4127")).run(&ctx).await.unwrap();

        assert_eq!(
            output,
            "Skill: apple (located at .claude/skills/apple)\n\nDeploy apple to 4127"
        );
    }

    #[tokio::test]
    async fn skill_without_arguments_leaves_placeholder_empty() {
        let ctx = ctx_with(vec![skill("apple", "Deploy apple to $ARGUMENTS")]);

        let output = skill_args("apple", None).run(&ctx).await.unwrap();

        assert!(output.ends_with("\n\nDeploy apple to "));
    }

    #[tokio::test]
    async fn skill_body_without_placeholder_gets_arguments_appended() {
        let ctx = ctx_with(vec![skill("plain", "Say hello")]);

        let output = skill_args("plain", Some("to bob")).run(&ctx).await.unwrap();

        assert!(output.ends_with("\n\nSay hello\n\nto bob"));
    }

    #[tokio::test]
    async fn unknown_skill_is_an_error_naming_it() {
        let error = skill_args("definitely-not-a-skill", None)
            .run(&ctx())
            .await
            .unwrap_err();

        assert!(error.to_string().contains("definitely-not-a-skill"));
    }

    #[tokio::test]
    async fn execute_turns_unknown_skill_into_error_text() {
        let tool_call = ToolCall {
            id: "call_1".to_string(),
            kind: "function".to_string(),
            function: call("Skill", r#"{"name": "definitely-not-a-skill"}"#),
        };

        assert!(tool_call.execute(&ctx()).await.starts_with("error"));
    }

    // ---------- detail ----------

    #[test]
    fn detail_is_the_main_argument() {
        let bash = ToolInput::Bash(BashInput {
            command: "ls -la".to_string(),
        });
        let skill = ToolInput::Skill(skill_args("loc", Some("src rs 3")));
        let bare_skill = ToolInput::Skill(skill_args("loc", Some("")));

        assert_eq!(bash.detail(), "ls -la");
        assert_eq!(skill.detail(), "loc src rs 3");
        assert_eq!(bare_skill.detail(), "loc");
    }

    #[test]
    fn tool_definitions_snapshot() {
        insta::assert_json_snapshot!(tool_definitions());
    }
}
