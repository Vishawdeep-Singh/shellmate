use std::fmt::format;

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

pub enum Tool {
    Read(ReadArgs),
}

impl Tool {
    pub fn parse(call: &FunctionCall) -> Result<Self, Box<dyn std::error::Error>> {
        match call.name.as_str() {
            "Read" => Ok(Tool::Read(serde_json::from_str(&call.arguments)?)),
            other => Err(format!("unknown tool: {other}").into()),
        }
    }
    pub fn run(&self) -> Result<String, Box<dyn std::error::Error>> {
        match self {
            Tool::Read(args) => Ok(std::fs::read_to_string(&args.file_path)?),
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
    }
    ])
}
