use ratatui::{
    crossterm::event::{self, Event, KeyCode, KeyEventKind},
    layout::{Constraint, Layout},
    style::{Color, Modifier, Style, Stylize},
    text::{Line, Text},
    widgets::{Block, Paragraph},
    Frame,
};
use std::io;

mod auth;
pub mod tui;

#[derive(Debug)]
enum InputMode {
    Normal,
    Editing,
}

impl Default for InputMode {
    fn default() -> Self {
        InputMode::Normal
    }
}

#[derive(Debug, Default)]
pub struct App {
    service: String,
    current_username: String,
    failed_counter: u8,
    exit: bool,
    input: String,
    display_text: String,
    character_index: usize,
    input_mode: InputMode,
    messages: Vec<String>,
    cursor_position: (u16, u16),
}

impl App {
    pub const fn new(service: String, current_username: String) -> Self {
        Self {
            service,
            current_username,
            failed_counter: 0,
            exit: false,
            input: String::new(),
            display_text: String::new(),
            input_mode: InputMode::Normal,
            messages: Vec::new(),
            character_index: 0,
            cursor_position: (0, 0),
        }
    }

    fn move_cursor_left(&mut self) {
        let cursor_moved_left = self.character_index.saturating_sub(1);
        self.character_index = self.clamp_cursor(cursor_moved_left);
    }

    fn move_cursor_right(&mut self) {
        let cursor_moved_right = self.character_index.saturating_add(1);
        self.character_index = self.clamp_cursor(cursor_moved_right);
    }

    fn enter_char(&mut self, new_char: char) {
        let index = self.byte_index();
        self.input.insert(index, new_char);
        self.move_cursor_right();
    }

    fn byte_index(&self) -> usize {
        self.input
            .char_indices()
            .map(|(i, _)| i)
            .nth(self.character_index)
            .unwrap_or(self.input.len())
    }

    fn delete_char(&mut self) {
        let is_not_cursor_leftmost = self.character_index != 0;

        if is_not_cursor_leftmost {
            let current_index = self.character_index;
            let from_left_to_current_index = current_index - 1;

            let before_char_to_delete = self.input.chars().take(from_left_to_current_index);
            let after_char_to_delete = self.input.chars().skip(current_index);

            self.input = before_char_to_delete.chain(after_char_to_delete).collect();
            self.move_cursor_left();
        }
    }

    fn clamp_cursor(&self, new_cursor_pos: usize) -> usize {
        new_cursor_pos.clamp(0, self.input.chars().count())
    }

    fn reset_cursor(&mut self) {
        self.character_index = 0;
    }

    fn submit_message(&mut self) {
        let authenticated = auth::authenticate(
            self.service.as_str(),
            self.current_username.as_str(),
            self.input.as_str(),
        );

        if !authenticated {
            self.increment_failed_counter();
            self.display_text = format!("Invalid Password ({})", self.failed_counter);
        } else {
            self.exit();
        }

        self.messages.push(self.input.clone());
        self.input.clear();
        self.reset_cursor();
    }

    pub fn run(&mut self, terminal: &mut tui::Tui) -> io::Result<()> {
        while !self.exit {
            terminal.draw(|frame| self.render_frame(frame))?;
            self.handle_events()?;
        }

        Ok(())
    }

    fn render_frame(&mut self, frame: &mut Frame) {
        let vertical = Layout::vertical([
            Constraint::Percentage(10),
            Constraint::Length(1),
            Constraint::Percentage(25),
            Constraint::Min(1),
            Constraint::Percentage(25),
            Constraint::Length(1),
            Constraint::Percentage(10),
        ]);
        let [_, title_area, _, main_area, _, help_area, _] = vertical.areas(frame.size());

        let title = Text::from(" TTYLOCK ".bold());
        let title = Paragraph::new(title).centered();

        frame.render_widget(title, title_area);

        let (msg, style) = match self.input_mode {
            InputMode::Normal => (
                vec![
                    "Press ".into(),
                    "Enter".bold(),
                    " and start typing your password".into(),
                ],
                Style::default().add_modifier(Modifier::RAPID_BLINK),
            ),
            InputMode::Editing => (
                vec![
                    "Press ".into(),
                    "Esc".bold(),
                    " to return to lockscreen, ".into(),
                    "Enter".bold(),
                    " to login".into(),
                ],
                Style::default(),
            ),
        };
        let help = Text::from(Line::from(msg)).patch_style(style);
        let help_message = Paragraph::new(help).centered();

        frame.render_widget(help_message, help_area);

        let main_layout = Layout::vertical([
            Constraint::Length(1),
            Constraint::Max(5),
            Constraint::Length(3),
        ]);

        let [username_area, _, password_area] = main_layout.areas(main_area);

        let username_text = Text::from(vec![Line::from(vec![
            "Username: ".into(),
            self.current_username.as_str().to_string().yellow(),
        ])]);

        let para = Paragraph::new(username_text).centered();
        frame.render_widget(para, username_area);

        let password_layout = Layout::horizontal([
            Constraint::Percentage(35),
            Constraint::Percentage(30),
            Constraint::Percentage(35),
        ]);
        let [_, password_area, _] = password_layout.areas(password_area);

        match self.input_mode {
            InputMode::Normal => {
                self.display_text.clear();
            }
            InputMode::Editing => {
                self.cursor_position = (
                    password_area.x + self.character_index as u16 + 1,
                    password_area.y + 1,
                );

                if self.display_text.is_empty() || self.display_text.starts_with("*") {
                    self.display_text = "*".repeat(self.input.len());
                }
            }
        }

        let password_input = Paragraph::new(self.display_text.as_str())
            .style(match self.input_mode {
                InputMode::Normal => Style::default(),
                InputMode::Editing => Style::default().fg(Color::Yellow),
            })
            .block(Block::bordered().title("Password"));
        frame.render_widget(password_input, password_area);
    }

    fn handle_events(&mut self) -> io::Result<()> {
        if let Event::Key(key) = event::read()? {
            match self.input_mode {
                InputMode::Normal => match key.code {
                    KeyCode::Enter => {
                        self.input_mode = InputMode::Editing;
                    }
                    _ => {}
                },
                InputMode::Editing if key.kind == KeyEventKind::Press => match key.code {
                    KeyCode::Enter => self.submit_message(),
                    KeyCode::Char(to_insert) => {
                        if self.display_text.starts_with("Invalid Password (") {
                            self.display_text.clear();
                        }

                        self.enter_char(to_insert);
                    }
                    KeyCode::Backspace => {
                        self.delete_char();
                    }
                    KeyCode::Left => {
                        self.move_cursor_left();
                    }
                    KeyCode::Right => {
                        self.move_cursor_right();
                    }
                    KeyCode::Esc => {
                        self.input.clear();
                        self.input_mode = InputMode::Normal;
                    }
                    _ => {}
                },
                InputMode::Editing => {}
            }
        }

        Ok(())
    }

    fn exit(&mut self) {
        self.exit = true;
    }

    fn increment_failed_counter(&mut self) {
        self.failed_counter += 1;
    }
}
