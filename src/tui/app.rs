//! The state of the full-screen UI, and how keys and agent events change it.
//! Nothing here draws. `view.rs` reads this state and draws it.

use std::time::Instant;

use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

use super::dropdown::{self, Dropdown, ModelList, Pick, SkillInfo};
use crate::events::AgentEvent;
use crate::models::ModelInfo;

/// One item in the chat panel.
pub enum Entry {
    User(String),
    Event(AgentEvent),
    Answer(String),
    Error(String),
    /// Something the UI did itself, like switching the model.
    Notice(String),
}

pub enum Status {
    Ready,
    Thinking { since: Instant },
}

/// What the event loop should do after a key press.
#[derive(Debug, PartialEq)]
pub enum Action {
    Nothing,
    Submit(String),
    Quit,
}

pub struct App {
    pub entries: Vec<Entry>,
    pub input: String,
    pub status: Status,
    /// How many lines the chat is scrolled up from the bottom. 0 follows new messages.
    pub scroll_up: u16,
    pub skills: Vec<SkillInfo>,
    /// The model the next prompt goes to.
    pub model: String,
    pub models: ModelList,
    /// The highlighted row in the `/` dropdown.
    pub selected: usize,
    /// Advances on every tick, to animate the spinner.
    pub frame: usize,
}

impl App {
    pub fn new(skills: Vec<SkillInfo>, model: String) -> Self {
        App {
            entries: Vec::new(),
            input: String::new(),
            status: Status::Ready,
            scroll_up: 0,
            skills,
            model,
            models: ModelList::Loading,
            selected: 0,
            frame: 0,
        }
    }

    pub fn is_busy(&self) -> bool {
        matches!(self.status, Status::Thinking { .. })
    }

    /// The `/` dropdown for what is typed so far, or `None` when it is closed.
    pub fn dropdown(&self) -> Option<Dropdown> {
        dropdown::build(&self.input, &self.skills, &self.models, &self.model)
    }

    pub fn on_key(&mut self, key: KeyEvent) -> Action {
        if key.kind != KeyEventKind::Press {
            return Action::Nothing;
        }
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);

        match key.code {
            KeyCode::Char('c' | 'd') if ctrl => return Action::Quit,
            KeyCode::Enter => return self.submit(),
            KeyCode::Char(c) => self.type_text(|input| input.push(c)),
            KeyCode::Backspace => self.type_text(|input| {
                input.pop();
            }),
            KeyCode::Esc => self.type_text(String::clear),
            KeyCode::Tab => {
                self.accept_suggestion();
            }
            KeyCode::Up => self.up(),
            KeyCode::Down => self.down(),
            KeyCode::PageUp => self.scroll_up = self.scroll_up.saturating_add(10),
            KeyCode::PageDown => self.scroll_up = self.scroll_up.saturating_sub(10),
            _ => {}
        }
        Action::Nothing
    }

    pub fn on_agent_event(&mut self, event: AgentEvent) {
        self.entries.push(Entry::Event(event));
    }

    pub fn on_turn_done(&mut self, result: anyhow::Result<Option<String>>) {
        match result {
            Ok(Some(answer)) => self.entries.push(Entry::Answer(answer)),
            Ok(None) => {}
            Err(err) => self.entries.push(Entry::Error(format!("{err:#}"))),
        }
        self.status = Status::Ready;
    }

    pub fn on_models_loaded(&mut self, result: anyhow::Result<Vec<ModelInfo>>) {
        self.models = match result {
            Ok(models) => ModelList::Ready(models),
            Err(err) => ModelList::Failed(format!("{err:#}")),
        };
    }

    pub fn tick(&mut self) {
        self.frame = self.frame.wrapping_add(1);
    }

    /// Changes the input, and starts the dropdown selection from the top again.
    fn type_text(&mut self, edit: impl FnOnce(&mut String)) {
        edit(&mut self.input);
        self.selected = 0;
    }

    /// Enter first completes a skill from the dropdown; otherwise it sends the prompt.
    fn submit(&mut self) -> Action {
        if self.accept_suggestion() {
            return Action::Nothing;
        }
        let prompt = self.input.trim().to_string();
        if prompt.is_empty() || self.is_busy() {
            return Action::Nothing;
        }
        self.input.clear();
        if prompt == "exit" {
            return Action::Quit;
        }

        self.entries.push(Entry::User(prompt.clone()));
        self.status = Status::Thinking {
            since: Instant::now(),
        };
        self.scroll_up = 0;
        Action::Submit(prompt)
    }

    /// Acts on the highlighted dropdown row. Returns false when the dropdown is closed,
    /// so Enter can send the prompt instead.
    fn accept_suggestion(&mut self) -> bool {
        let Some(dropdown) = self.dropdown() else {
            return false;
        };
        let pick = dropdown
            .rows
            .into_iter()
            .nth(self.selected)
            .map_or(Pick::Nothing, |row| row.pick);

        match pick {
            Pick::Complete(text) => self.input = text,
            Pick::Model(id) => self.switch_model(id),
            Pick::Nothing => {}
        }
        self.selected = 0;
        true
    }

    fn switch_model(&mut self, id: String) {
        self.entries
            .push(Entry::Notice(format!("switched to {id}")));
        self.model = id;
        self.input.clear();
    }

    fn dropdown_len(&self) -> usize {
        self.dropdown().map_or(0, |dropdown| dropdown.rows.len())
    }

    /// Up moves through the dropdown when it is open, and scrolls the chat otherwise.
    fn up(&mut self) {
        let count = self.dropdown_len();
        if count > 0 {
            self.selected = (self.selected + count - 1) % count;
        } else {
            self.scroll_up = self.scroll_up.saturating_add(1);
        }
    }

    fn down(&mut self) {
        let count = self.dropdown_len();
        if count > 0 {
            self.selected = (self.selected + 1) % count;
        } else {
            self.scroll_up = self.scroll_up.saturating_sub(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn app() -> App {
        let skill = |name: &str| SkillInfo {
            name: name.to_string(),
            description: format!("The {name} skill"),
        };
        let mut app = App::new(
            vec![skill("apple"), skill("loc"), skill("lint")],
            "anthropic/claude-haiku-4.5".to_string(),
        );
        let model = |id: &str| -> ModelInfo {
            serde_json::from_value(serde_json::json!({ "id": id, "name": id })).unwrap()
        };
        app.on_models_loaded(Ok(vec![
            model("anthropic/claude-haiku-4.5"),
            model("anthropic/claude-sonnet-4.5"),
        ]));
        app
    }

    fn press(app: &mut App, code: KeyCode) -> Action {
        app.on_key(KeyEvent::new(code, KeyModifiers::NONE))
    }

    fn type_str(app: &mut App, text: &str) {
        for c in text.chars() {
            press(app, KeyCode::Char(c));
        }
    }

    #[test]
    fn enter_completes_the_highlighted_skill_instead_of_sending() {
        let mut app = app();
        type_str(&mut app, "/l");
        press(&mut app, KeyCode::Down);

        assert_eq!(press(&mut app, KeyCode::Enter), Action::Nothing);
        assert_eq!(app.input, "/lint ");
    }

    #[test]
    fn up_wraps_around_to_the_last_row() {
        let mut app = app();
        type_str(&mut app, "/");
        press(&mut app, KeyCode::Up);
        press(&mut app, KeyCode::Tab);

        assert_eq!(app.input, "/lint ");
    }

    #[test]
    fn picking_a_model_switches_to_it() {
        let mut app = app();
        type_str(&mut app, "/mo");
        press(&mut app, KeyCode::Enter);
        assert_eq!(app.input, "/model ");

        type_str(&mut app, "sonnet");
        assert_eq!(press(&mut app, KeyCode::Enter), Action::Nothing);

        assert_eq!(app.model, "anthropic/claude-sonnet-4.5");
        assert!(app.input.is_empty());
        assert!(matches!(
            app.entries.last(),
            Some(Entry::Notice(text)) if text == "switched to anthropic/claude-sonnet-4.5"
        ));
    }

    #[test]
    fn an_unknown_model_is_never_sent_to_the_agent() {
        let mut app = app();
        type_str(&mut app, "/model zzz");

        assert_eq!(press(&mut app, KeyCode::Enter), Action::Nothing);
        assert_eq!(app.model, "anthropic/claude-haiku-4.5");
    }

    #[test]
    fn enter_sends_the_prompt_and_starts_thinking() {
        let mut app = app();
        type_str(&mut app, "/loc src rs 3");

        assert_eq!(
            press(&mut app, KeyCode::Enter),
            Action::Submit("/loc src rs 3".to_string())
        );
        assert!(app.input.is_empty());
        assert!(app.is_busy());
    }

    #[test]
    fn nothing_is_sent_while_the_agent_is_busy() {
        let mut app = app();
        type_str(&mut app, "first");
        press(&mut app, KeyCode::Enter);
        type_str(&mut app, "second");

        assert_eq!(press(&mut app, KeyCode::Enter), Action::Nothing);
        assert_eq!(app.input, "second");
    }

    #[test]
    fn exit_and_ctrl_c_quit() {
        let mut app = app();
        type_str(&mut app, "exit");
        assert_eq!(press(&mut app, KeyCode::Enter), Action::Quit);

        let ctrl_c = KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL);
        assert_eq!(app.on_key(ctrl_c), Action::Quit);
    }

    #[test]
    fn a_finished_turn_adds_the_answer_and_is_ready_again() {
        let mut app = app();
        type_str(&mut app, "hi");
        press(&mut app, KeyCode::Enter);

        app.on_turn_done(Ok(Some("hello!".to_string())));

        assert!(!app.is_busy());
        assert!(matches!(app.entries.last(), Some(Entry::Answer(text)) if text == "hello!"));
    }
}
