use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::Stylize,
    symbols::border,
    text::{Line, Text},
    widgets::{Block, Paragraph, Widget},
    DefaultTerminal, Frame,
};
use crate::server::{deserialise, Message};
use crate::client::Client;

#[derive(Default)]
pub struct Application {
    messages: Vec<Message>,
    user_input: Vec<char>,
    pub exit: bool,
    send: bool
}

impl Application {
    pub fn new() -> Self {
        Self {
            messages: vec![],
            user_input: vec![],
            exit: false,
            send: false
        }
    }

    pub fn run(&mut self, terminal: &mut DefaultTerminal, client: &mut Client, hostname: &String) {
        while !self.exit {
            terminal.draw(|frame| self.draw(frame));
            let _ = self.handle_events();
            if self.send {
                self.send = false;
                let message: Message = match deserialise(format!("{hostname}|{}", self.user_input.iter().collect::<String>())).first() {
                    Some(message) => message.clone(),
                    None => return
                };
                let _ = client.send(&message);
                self.user_input.clear();
            }
        }
    }

    fn draw(&self, frame: &mut Frame) {
        frame.render_widget(self, frame.area());
    }

    fn handle_events(&mut self) {
        let e = match event::read() {
            Ok(e) => e,
            Err(_) => return
        };
        match e {
            Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                self.handle_key_event(key_event)
            }
            _ => { }
        };
    }

    fn handle_key_event(&mut self, key_event: KeyEvent) {
        match key_event.code {
            KeyCode::Esc => self.exit = true,
            KeyCode::Backspace => { self.user_input.pop(); },
            KeyCode::Char(c) => self.user_input.push(c),
            KeyCode::Enter => { self.send = true; },
            _ => {}
        };
    }
}

impl Widget for &Application {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let title = Line::from(" Rust Messaging App ").bold();
        let input = Line::from(self.user_input.iter().collect::<String>()).bold();
        let messages = Text::from(
            self.messages.iter().map(|x| Line::from(vec![
                x.author.to_string().magenta(),
                " -> ".gray(),
                x.content.to_string().white()
            ])).collect::<Vec<Line>>()
        );

        let block = Block::bordered().title(title).title_bottom(input).border_set(border::THICK);
        Paragraph::new(messages).centered().block(block).render(area, buf);
    }
}

pub fn execute_application(mut application: Application, terminal: &mut DefaultTerminal, client: &mut Client, hostname: &String) {
    loop {
        match application.run(terminal, client, hostname) {
            Ok(_) => { if application.exit { break; }}
            Err(_) => { break; }
        }
    }
    ratatui::restore();
}
