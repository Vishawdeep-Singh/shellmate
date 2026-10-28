use std::fmt::{Write, format};

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

pub enum Tool {
    Read(ReadArgs),
    Write(WriteArgs),
}

impl Tool {
    pub fn parse(call: &FunctionCall) -> Result<Self, Box<dyn std::error::Error>> {
        match call.name.as_str() {
            "Read" => Ok(Tool::Read(serde_json::from_str(&call.arguments)?)),
            "Write" => Ok(Tool::Write(serde_json::from_str(&call.arguments)?)),
            other => Err(format!("unknown tool: {other}").into()),
        }
    }
    pub fn run(&self) -> Result<String, Box<dyn std::error::Error>> {
        match self {
            Tool::Read(args) => Ok(std::fs::read_to_string(&args.file_path)?),
            Tool::Write(args) => {
                std::fs::write(&args.file_path, &args.content)?;
                Ok(format!(
                    "Wrote {} bytes to {}",
                    args.content.len(),
                    args.file_path
                ))
            }
        }
    }
}
impl ToolCall {
    pub fn execute(&self) -> String {
        Tool::parse(&self.function)
            .and_then(|tool| tool.run())
            .unwrap_or_else(|e| format!("error :{e}"))
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
    }
        ])
}
