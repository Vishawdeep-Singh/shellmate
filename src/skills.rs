use std::fs;
use std::path::Path;
use std::sync::LazyLock;
use std::{collections::HashMap, path::PathBuf};

use regex::{Captures, Regex};

static PLACEHOLDER: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\$(?:ARGUMENTS\[(?P<index>\d+)\]|(?P<all>ARGUMENTS)|(?P<num>\d+))").unwrap()
});

#[derive(Debug, PartialEq)]
pub struct Skill {
    pub frontmatter: HashMap<String, String>,
    pub body: String,
    pub dir: PathBuf,
}

impl Skill {
    pub fn name(&self) -> Option<&str> {
        self.frontmatter.get("name").map(String::as_str)
    }

    pub fn description(&self) -> Option<&str> {
        self.frontmatter.get("description").map(String::as_str)
    }

    pub fn context_fork(&self) -> Option<&str> {
        self.frontmatter.get("context").map(String::as_str)
    }

    pub fn render(&self, arguments: &str) -> String {
        let name = self.name().unwrap_or("unknown");
        let body = Skill::substitute_arguments(&self.body, arguments);
        format!(
            "Skill: {name} (located at {})\n\n{body}",
            self.dir.display()
        )
    }

    fn skill_parser(skill: &str, path: PathBuf) -> Skill {
        let parts: Vec<&str> = skill.splitn(3, "---").collect();
        let frontmatter: HashMap<String, String> = parts[1]
            .lines()
            .filter_map(|line| line.split_once(":"))
            .map(|(key, value)| (key.trim().to_string(), value.trim().to_string()))
            .collect();
        let body = parts[2].trim().to_string();
        Skill {
            frontmatter,
            body,
            dir: path,
        }
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
            skills.push(Skill::skill_parser(&content, path));
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

        let skill = Skill::skill_parser(text, PathBuf::from(".claude/skills/onyx"));

        assert_eq!(skill.frontmatter["name"], "onyx");
        assert_eq!(skill.frontmatter["description"], "Says one word");
        assert_eq!(skill.body.trim(), "Respond with exactly one word: damson");
    }

    // A colon inside a value and a `---` inside the body must not break parsing.
    #[test]
    fn skill_parser_handles_tricky_content() {
        let text = "---\nname: onyx\ndescription: Runs tests: unit and e2e\n---\nStep one\n---\nStep two\n";

        let skill = Skill::skill_parser(text, PathBuf::from(".claude/skills/onyx"));

        assert_eq!(skill.frontmatter["description"], "Runs tests: unit and e2e");
        assert_eq!(skill.body.trim(), "Step one\n---\nStep two");
    }

    // ---------- scan_skills ----------

    // Each test gets its own empty folder, so tests running in parallel never collide.
    fn temp_dir(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("skills-test-{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn scan_skills_missing_folder_returns_no_skills() {
        let missing = temp_dir("scan-missing").join("skills");

        assert!(Skill::scan_skills(&missing).is_empty());
    }

    #[test]
    fn scan_skills_reads_only_folders_with_skill_md() {
        let dir = temp_dir("scan-mixed");
        fs::create_dir_all(dir.join("apple")).unwrap();
        fs::write(
            dir.join("apple/SKILL.md"),
            "---\nname: apple\ndescription: Deploys apple\n---\nDeploy it\n",
        )
        .unwrap();
        fs::create_dir_all(dir.join("empty")).unwrap();
        fs::write(dir.join("notes.txt"), "not a skill").unwrap();

        let skills = Skill::scan_skills(&dir);

        assert_eq!(skills.len(), 1);
        assert_eq!(skills[0].name(), Some("apple"));
        assert_eq!(skills[0].description(), Some("Deploys apple"));
        assert_eq!(skills[0].body, "Deploy it");
    }

    // ---------- substitute_arguments ----------

    #[test]
    fn no_placeholders_and_no_arguments_returns_body() {
        assert_eq!(Skill::substitute_arguments("Say hello", ""), "Say hello");
    }

    #[test]
    fn no_placeholders_appends_arguments() {
        assert_eq!(
            Skill::substitute_arguments("Say hello", "to bob"),
            "Say hello\n\nto bob"
        );
    }

    #[test]
    fn arguments_placeholder_is_the_whole_text() {
        assert_eq!(
            Skill::substitute_arguments("Hi $ARGUMENTS", "alice bob"),
            "Hi alice bob"
        );
    }

    #[test]
    fn arguments_placeholder_keeps_spacing() {
        assert_eq!(
            Skill::substitute_arguments("[$ARGUMENTS]", "alice   bob"),
            "[alice   bob]"
        );
    }

    #[test]
    fn numbered_placeholders_pick_words() {
        assert_eq!(
            Skill::substitute_arguments("$0 and $1", "alice bob"),
            "alice and bob"
        );
    }

    #[test]
    fn indexed_placeholder_picks_word() {
        assert_eq!(
            Skill::substitute_arguments("$ARGUMENTS[1]", "alice bob"),
            "bob"
        );
    }

    #[test]
    fn placeholder_is_replaced_every_time() {
        assert_eq!(Skill::substitute_arguments("$0 $0", "alice"), "alice alice");
    }

    #[test]
    fn indexed_and_plain_arguments_mix() {
        assert_eq!(
            Skill::substitute_arguments("$ARGUMENTS / $ARGUMENTS[0]", "alice bob"),
            "alice bob / alice"
        );
    }

    #[test]
    fn two_digit_placeholder_is_read_as_one_number() {
        assert_eq!(
            Skill::substitute_arguments("$10", "a b c d e f g h i j k"),
            "k"
        );
    }

    #[test]
    fn missing_argument_becomes_empty() {
        assert_eq!(Skill::substitute_arguments("[$5]", "alice"), "[]");
    }

    #[test]
    fn placeholder_without_arguments_becomes_empty() {
        assert_eq!(Skill::substitute_arguments("Hi $0!", ""), "Hi !");
    }

    #[test]
    fn huge_index_becomes_empty_instead_of_panicking() {
        assert_eq!(
            Skill::substitute_arguments("[$99999999999999999999]", "alice"),
            "[]"
        );
    }

    // An argument that looks like a placeholder is inserted as is, never replaced again.
    #[test]
    fn inserted_arguments_are_not_substituted_again() {
        assert_eq!(Skill::substitute_arguments("$0 $1", "$1 x"), "$1 x");
    }
}
