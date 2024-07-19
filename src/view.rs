use std::{ io::{self, Stdout}, time::Duration, };
use crate::ApplicationError;


use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{
        disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
    },
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Paragraph, Wrap},
};
use tui_input::backend::crossterm::EventHandler;
use tui_input::Input;


enum InputMode {
    Normal,
    Editing,
}

type Terminal = ratatui::Terminal<CrosstermBackend<Stdout>>;
pub struct ChatWindow {
    input: Input,
    input_mode: InputMode,
    output: Vec<String>,
    terminal: Terminal,
}

impl ChatWindow {
    pub fn build() -> Result<ChatWindow, ApplicationError> {
        let terminal = Self::build_terminal()?;
        
        Ok(ChatWindow {
            input: Input::default(),
            input_mode: InputMode::Normal,
            output: Vec::new(),
            terminal,
        })
    }

    fn build_terminal() -> Result<Terminal, ApplicationError> {
        match enable_raw_mode() {
            Ok(_) => (),
            Err(_) => return Err(ApplicationError::TerminalError),
        }

        let mut stdout = io::stdout();
        match execute!(stdout, EnterAlternateScreen, EnableMouseCapture) {
            Ok(_) => (),
            Err(_) => return Err(ApplicationError::TerminalError),
        }

        let backend = CrosstermBackend::new(stdout);
        match Terminal::new(backend) {
            Ok(terminal) => Ok(terminal),
            Err(_) => Err(ApplicationError::TerminalError),
        }
    }

    pub fn close_terminal(&mut self) -> Result<(), ApplicationError> {
        match disable_raw_mode() {
            Ok(_) => (),
            Err(_) => return Err(ApplicationError::TerminalError),
        }

        match execute!(
            self.terminal.backend_mut(),
            LeaveAlternateScreen,
            DisableMouseCapture
        ) {
            Ok(_) => (),
            Err(_) => return Err(ApplicationError::TerminalError),
        }
        match self.terminal.show_cursor() {
            Ok(_) => Ok(()),
            Err(_) => Err(ApplicationError::TerminalError)
        }
    }

    pub fn draw(&mut self, log: &Vec<String>) -> Result<(), ApplicationError> {
        match self.terminal.draw(|f| {
            let rects = Layout::default()
                .direction(Direction::Vertical)
                .margin(2)
                .constraints(
                    [
                        Constraint::Min(1),
                        Constraint::Length(3),
                        Constraint::Length(1),
                    ]
                    .as_ref(),
                )
                .split(f.size());

            let (msg, style) = match self.input_mode {
                InputMode::Normal => (
                    vec![
                        Span::raw("Press "),
                        Span::styled("Esc", Style::default().add_modifier(Modifier::BOLD)),
                        Span::raw(" to exit, "),
                        Span::styled("Enter", Style::default().add_modifier(Modifier::BOLD)),
                        Span::raw(" to type in the chat."),
                    ],
                    Style::default().add_modifier(Modifier::RAPID_BLINK),
                ),
                InputMode::Editing => (
                    vec![
                        Span::raw("Press "),
                        Span::styled("Esc", Style::default().add_modifier(Modifier::BOLD)),
                        Span::raw(" to stop editing, "),
                        Span::styled("Enter", Style::default().add_modifier(Modifier::BOLD)),
                        Span::raw(" to send the message."),
                    ],
                    Style::default(),
                ),
            };

            let mut text = Text::from(Line::from(msg));
            text = text.patch_style(style);
            let help_message = Paragraph::new(text);
            f.render_widget(help_message, rects[2]);

            let width = rects[0].width.max(3) - 3; // 2 width reserved for borders, 1 for cursor

            let scroll = self.input.visual_scroll(width as usize);
            let input = Paragraph::new(self.input.value())
                .style(match self.input_mode {
                    InputMode::Normal => Style::default(),
                    InputMode::Editing => Style::default().fg(Color::Yellow),
                })
                .scroll((0, scroll as u16))
                .block(Block::default().borders(Borders::ALL).title("Input"));
            f.render_widget(input, rects[1]);

            match self.input_mode {
                InputMode::Normal => {}
                InputMode::Editing => {
                    f.set_cursor(
                        // place cursor past end of input text
                        rects[1].x
                            + ((self.input.visual_cursor()).max(scroll) - scroll) as u16
                            + 1,
                            // move cursor from the border to the input line
                            rects[1].y + 1,
                    )
                }
            }

            let mut lines = vec![];

            for msg in log {
                lines.push(Line::raw(msg));
            }
            let chat = Paragraph::new(Text::from(lines))
                .wrap(Wrap { trim: true })
                .block(Block::default().borders(Borders::ALL).title("Chat Log"));

            f.render_widget(chat, rects[0]);
        }) {
            Ok(_) => Ok(()),
            Err(_) => Err(ApplicationError::TerminalError),
        }
    }

    pub fn get_output(&mut self) -> Option<String> {
        self.output.pop()
    }

    pub fn run(&mut self) -> Result<bool, ApplicationError> {
        if !event::poll(Duration::from_millis(100)).unwrap() {
            return Ok(true);
        }

        if let Ok(Event::Key(key)) = event::read() {
            match self.input_mode {
                InputMode::Normal => match key.code {
                    KeyCode::Enter => {
                        self.input_mode = InputMode::Editing;
                        return Ok(true);
                    }
                    KeyCode::Esc => {
                        return Ok(false);
                    }
                    _ => { return Ok(true); }
                }
                InputMode::Editing => match key.code {
                    KeyCode::Enter => {
                        let msg = self.input.value().into();
                        self.output.push(msg);
                        self.input.reset();
                        return Ok(true);
                    }
                    KeyCode::Esc => {
                        self.input_mode = InputMode::Normal;
                        return Ok(true);
                    }
                    _ => {
                        self.input.handle_event(&Event::Key(key));
                        return Ok(true);
                    }
                }
            }
        } else { Ok(true) }
    }
}