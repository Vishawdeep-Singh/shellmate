use serde::Deserialize;
use serde_json::{Value, json};

#[derive(Deserialize)]
pub struct ToolCall {
    pub id: String,
    pub function: FunctionCall,
}

#[derive(Deserialize)]
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
