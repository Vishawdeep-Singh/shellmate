# shellmate

A small AI coding assistant for the terminal, written in Rust. It sends your
prompt to an LLM through an OpenAI-compatible API, lets the model call tools
(read files, write files, run shell commands, invoke skills), and loops until
the model gives a final answer.

## Features

- **Agent loop** with tool calling: Read, Write, Bash and Skill
- **Skills** loaded from `.claude/skills/<name>/SKILL.md`, with YAML frontmatter
  (`name`, `description`) and a body of instructions
- **Slash commands**: `/name args` expands one or more skills into the prompt,
  with `$ARGUMENTS`, `$ARGUMENTS[N]` and `$N` placeholders
- **Forked skills**: a skill with `context: fork` runs in its own subagent, and
  only its final answer is returned to the main conversation

## Setup

Set your API key (OpenRouter by default):

```sh
export OPENROUTER_API_KEY=...
# optional, defaults to https://openrouter.ai/api/v1
export OPENROUTER_BASE_URL=...
```

## Usage

```sh
cargo run --release -- -p "What files are in this directory?"
cargo run --release -- -p "/apple 4127"
```

## Tests

```sh
cargo test
```

Tool definitions are covered by an [insta](https://insta.rs) snapshot. After
changing a tool's arguments or doc comments, review it with
`cargo insta review`.
