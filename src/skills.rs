use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::sync::LazyLock;

use regex::{Captures, Regex};

static PLACEHOLDER: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\$(?:ARGUMENTS\[(?P<index>\d+)\]|(?P<all>ARGUMENTS)|(?P<num>\d+))").unwrap()
});

#[derive(Debug, PartialEq)]
pub struct Skill {
    pub frontmatter: HashMap<String, String>,
    pub body: String,
}

impl Skill {
    pub fn name(&self) -> Option<&str> {
        self.frontmatter.get("name").map(String::as_str)
    }

    pub fn description(&self) -> Option<&str> {
        self.frontmatter.get("description").map(String::as_str)
    }

    fn skill_parser(skill: &str) -> Skill {
        let parts: Vec<&str> = skill.splitn(3, "---").collect();
        let frontmatter: HashMap<String, String> = parts[1]
            .lines()
            .filter_map(|line| line.split_once(":"))
            .map(|(key, value)| (key.trim().to_string(), value.trim().to_string()))
            .collect();
        let body = parts[2].trim().to_string();
        Skill { frontmatter, body }
    }

    pub fn scan_skills(path: &Path) -> Vec<Skill> {
        let mut skills: Vec<Skill> = Vec::new();
        if !path.is_dir() {
            return Vec::new();
        }

        let entries = fs::read_dir(path).unwrap();

        for entry in entries {
            let entry = entry.unwrap();
            let path = entry.path();

            if !path.is_dir() {
                continue;
            }

            let skill_file = path.join("SKILL.md");

            if !skill_file.exists() {
                continue;
            }

            let content = fs::read_to_string(&skill_file).unwrap();
            skills.push(Skill::skill_parser(&content));
        }
        skills
    }
    pub fn substitute_arguments(body: &str, arguments: &str) -> String {
        let words: Vec<&str> = arguments.split_whitespace().collect();

        if !PLACEHOLDER.is_match(body) {
            return if arguments.is_empty() {
                body.to_string()
            } else {
                format!("{body}\n\n{arguments}")
            };
        }

        PLACEHOLDER
            .replace_all(body, |caps: &Captures| {
                match caps.name("index").or(caps.name("num")) {
                    Some(number) => number
                        .as_str()
                        .parse::<usize>()
                        .ok()
                        .and_then(|n| words.get(n))
                        .copied()
                        .unwrap_or("")
                        .to_string(),
                    None => arguments.to_string(),
                }
            })
            .into_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn skill_parser_reads_frontmatter_and_body() {
        let text = "---\nname: onyx\ndescription: Says one word\n---\nRespond with exactly one word: damson\n";

        let skill = Skill::skill_parser(text);

        assert_eq!(skill.frontmatter["name"], "onyx");
        assert_eq!(skill.frontmatter["description"], "Says one word");
        assert_eq!(skill.body.trim(), "Respond with exactly one word: damson");
    }

    // A colon inside a value and a `---` inside the body must not break parsing.
    #[test]
    fn skill_parser_handles_tricky_content() {
        let text = "---\nname: onyx\ndescription: Runs tests: unit and e2e\n---\nStep one\n---\nStep two\n";

        let skill = Skill::skill_parser(text);

        assert_eq!(skill.frontmatter["description"], "Runs tests: unit and e2e");
        assert_eq!(skill.body.trim(), "Step one\n---\nStep two");
    }
}
