use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

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

#[derive(Deserialize)]
pub struct ReadArgs {
    pub file_path: String,
}
#[derive(Deserialize)]
pub struct WriteArgs {
    pub file_path: String,
    pub content: String,
}

#[derive(Deserialize)]
pub struct BashArgs {
    pub command: String,
}

#[derive(Deserialize)]
#[serde(tag = "name", content = "arguments")]
pub enum Tool {
    Read(ReadArgs),
    Write(WriteArgs),
    Bash(BashArgs),
}

trait Run {
    fn run(&self) -> Result<String>;
}

impl Run for ReadArgs {
    fn run(&self) -> Result<String> {
        Ok(std::fs::read_to_string(&self.file_path)?)
    }
}
impl Run for WriteArgs {
    fn run(&self) -> Result<String> {
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
impl Run for BashArgs {
    fn run(&self) -> Result<String> {
        let output = std::process::Command::new("sh")
            .arg("-c")
            .arg(&self.command)
            .output()?;
        let mut result = String::from_utf8_lossy(&output.stdout).into_owned();
        result.push_str(&String::from_utf8_lossy(&output.stderr));
        if !output.status.success() {
            result.push_str(&format!("\n(exit code: {:?})", output.status.code()));
        }
        Ok(result)
    }
}

impl Tool {
    pub fn parse(call: &FunctionCall) -> Result<Tool> {
        let arguments: Value = serde_json::from_str(&call.arguments)?;
        Ok(serde_json::from_value(
            json!({ "name": call.name, "arguments": arguments }),
        )?)
    }
    pub fn run(&self) -> Result<String> {
        match self {
            Tool::Read(args) => args.run(),
            Tool::Write(args) => args.run(),
            Tool::Bash(args) => args.run(),
        }
    }
}
impl ToolCall {
    pub fn execute(&self) -> String {
        Tool::parse(&self.function)
            .and_then(|tool| tool.run())
            .unwrap_or_else(|e| format!("error: {e:#}"))
    }
}

pub fn tools() -> Value {
    json!([
            {
              "type": "function",
              "function": {
                "name": "Read",
                "description": "Read and return the contents of a file",
                "parameters": {
                  "type": "object",
                  "properties": {
                    "file_path": {
                      "type": "string",
                      "description": "The path to the file to read"
                    }
                  },
                  "required": ["file_path"]
                }
              }
            },
            {
          "type": "function",
          "function": {
            "name": "Write",
            "description": "Write content to a file",
            "parameters": {
              "type": "object",
              "required": ["file_path", "content"],
              "properties": {
                "file_path": {
                  "type": "string",
                  "description": "The path of the file to write to"
                },
                "content": {
                  "type": "string",
                  "description": "The content to write to the file"
                }
              }
            }
          }
        },{
      "type": "function",
      "function": {
        "name": "Bash",
        "description": "Execute a shell command",
        "parameters": {
          "type": "object",
          "required": ["command"],
          "properties": {
            "command": {
              "type": "string",
              "description": "The command to execute"
            }
          }
        }
      }
    }
            ])
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::{Path, PathBuf};

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

    fn bash(command: &str) -> String {
        Tool::Bash(BashArgs {
            command: command.to_string(),
        })
        .run()
        .unwrap()
    }

    fn call(name: &str, arguments: &str) -> FunctionCall {
        FunctionCall {
            name: name.to_string(),
            arguments: arguments.to_string(),
        }
    }

    // ---------- Read ----------

    #[test]
    fn read_returns_exact_contents() {
        let file = temp_dir("read").join("apple.py");
        std::fs::write(&file, "print('hi')\n\n  indented\n").unwrap();

        let output = Tool::Read(ReadArgs {
            file_path: path_str(&file),
        })
        .run()
        .unwrap();

        assert_eq!(output, "print('hi')\n\n  indented\n");
    }

    #[test]
    fn read_missing_file_is_an_error() {
        let file = temp_dir("read-missing").join("nope.txt");

        assert!(
            Tool::Read(ReadArgs {
                file_path: path_str(&file)
            })
            .run()
            .is_err()
        );
    }

    // ---------- Write ----------

    #[test]
    fn write_creates_new_file() {
        let file = temp_dir("write-new").join("hello.txt");

        let output = Tool::Write(WriteArgs {
            file_path: path_str(&file),
            content: "hi there".to_string(),
        })
        .run()
        .unwrap();

        assert_eq!(std::fs::read_to_string(&file).unwrap(), "hi there");
        assert!(output.contains("8 bytes"));
    }

    #[test]
    fn write_overwrites_existing_file() {
        let file = temp_dir("write-overwrite").join("hello.txt");
        std::fs::write(&file, "old contents that are longer").unwrap();

        Tool::Write(WriteArgs {
            file_path: path_str(&file),
            content: "new".to_string(),
        })
        .run()
        .unwrap();

        assert_eq!(std::fs::read_to_string(&file).unwrap(), "new");
    }

    #[test]
    fn write_creates_missing_parent_dirs() {
        let file = temp_dir("write-nested").join("a/b/c/deep.txt");

        Tool::Write(WriteArgs {
            file_path: path_str(&file),
            content: "deep".to_string(),
        })
        .run()
        .unwrap();

        assert_eq!(std::fs::read_to_string(&file).unwrap(), "deep");
    }

    // ---------- Bash ----------

    #[test]
    fn bash_runs_simple_command() {
        assert_eq!(bash("echo hello"), "hello\n");
    }

    #[test]
    fn bash_supports_pipes() {
        assert_eq!(bash("printf 'b\\na\\n' | sort"), "a\nb\n");
    }

    #[test]
    fn bash_includes_stderr() {
        let output = bash("echo out; echo oops >&2");

        assert!(output.contains("out"));
        assert!(output.contains("oops"));
    }

    #[test]
    fn bash_reports_failure_exit_code() {
        assert!(bash("exit 3").contains("(exit code: Some(3))"));
    }

    #[test]
    fn bash_success_has_no_exit_code() {
        assert!(!bash("true").contains("exit code"));
    }

    // ---------- parse ----------

    #[test]
    fn parse_read() {
        let tool = Tool::parse(&call("Read", r#"{"file_path": "a.txt"}"#));

        assert!(matches!(tool, Ok(Tool::Read(args)) if args.file_path == "a.txt"));
    }

    #[test]
    fn parse_write() {
        let tool = Tool::parse(&call("Write", r#"{"file_path": "a.txt", "content": "hi"}"#));

        assert!(matches!(
            tool,
            Ok(Tool::Write(args)) if args.file_path == "a.txt" && args.content == "hi"
        ));
    }

    #[test]
    fn parse_bash() {
        let tool = Tool::parse(&call("Bash", r#"{"command": "ls"}"#));

        assert!(matches!(tool, Ok(Tool::Bash(args)) if args.command == "ls"));
    }

    #[test]
    fn parse_unknown_tool_is_an_error() {
        let error = Tool::parse(&call("Delete", "{}"))
            .err()
            .expect("should fail");

        assert!(error.to_string().contains("Delete"));
    }

    #[test]
    fn parse_missing_field_is_an_error() {
        let error = Tool::parse(&call("Read", r#"{"path": "a.txt"}"#))
            .err()
            .expect("should fail");

        assert!(error.to_string().contains("missing field"));
    }

    // ---------- execute ----------

    #[test]
    fn execute_turns_errors_into_text() {
        let tool_call = ToolCall {
            id: "call_1".to_string(),
            kind: "function".to_string(),
            function: call("Delete", "{}"),
        };

        assert!(tool_call.execute().starts_with("error"));
    }
}
