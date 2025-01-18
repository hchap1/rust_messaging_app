use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::Stylize,
    symbols::border,
    text::{Line, Text},
    widgets::{Block, Paragraph, Widget},
    DefaultTerminal
};
use crate::server::{deserialise, Message};
use crate::client::Client;
use crate::utility::{sync, sync_vec, AM, AMV};
use std::thread::{sleep, spawn};
use std::sync::Arc;
use std::time::Duration;

#[derive(Default)]
pub struct Application {
    messages: Vec<Message>,
    user_input: Vec<char>,
    pub exit: bool,
    send: bool
}

fn render(area: Rect, buf: &mut Buffer, user_input: &Vec<char>, messages: &Vec<Message>) {
    let title = Line::from(" Rust Messaging App ").bold();
    let input = Line::from(user_input.iter().collect::<String>()).bold();
    let messages = Text::from(
        messages.iter().map(|x| Line::from(vec![
            match x.author == "SERVER" {
                true => x.author.to_string().green(),
                false => x.author.to_string().magenta()
            },
            " -> ".gray(),
            x.content.to_string().white()
        ])).collect::<Vec<Line>>()
    );

    let block = Block::bordered().title(title).title_bottom(input).border_set(border::THICK);
    Paragraph::new(messages).centered().block(block).render(area, buf);
}

fn polling_message_renderer(
        exit: AM<bool>,
        send: AM<bool>,
        terminal: AM<DefaultTerminal>,
        mut client: AM<Client>,
        hostname: String,
        messages: AMV<Message>,
        user_input: AMV<char>) {
    {
        let user_input = user_input.lock().unwrap();
        {
            let mut terminal = terminal.lock().unwrap();
            let messages = messages.lock().unwrap();
            match terminal.draw(|frame| render(frame.area(), frame.buffer_mut(), &user_input, &messages)) {
                Ok(_) => {},
                Err(_) => return
            };
        }
    }
    loop {
        {
            let exit = exit.lock().unwrap();
            if *exit { break; }
        }
        let mut cont: bool = false;
        {
            let mut client = client.lock().unwrap();
            match client.consume_inbox() {
                Some(mut inbox) => {
                    let mut messages = messages.lock().unwrap();
                    messages.append(&mut inbox);
                }
                None => {
                    sleep(Duration::from_millis(500));
                    cont = true;
                }
            }
        }
        let mut user_input = user_input.lock().unwrap();
        let mut send = send.lock().unwrap();
        if *send {
            *send = false;
            let message: Message = match deserialise(format!("{hostname}|{}", user_input.iter().collect::<String>())).first() {
                Some(message) => message.clone(),
                None => return
            };
            {
                let mut client = client.lock().unwrap();
                let _ = client.send(&message);
            }
            cont = false;
            user_input.clear();
        }
        if cont {
            continue;
        }
        {
            let mut terminal = terminal.lock().unwrap();
            let messages = messages.lock().unwrap();
            match terminal.draw(|frame| render(frame.area(), frame.buffer_mut(), &user_input, &messages)) {
                Ok(_) => {},
                Err(_) => return
            };
        }
    }
}

fn blocking_event_loop(exit: AM<bool>, user_input: AMV<char>, send: AM<bool>, terminal: AM<DefaultTerminal>, messages: AMV<Message>) {
    loop {
        let e = match event::read() {
            Ok(e) => e,
            Err(_) => return
        };
        match e {
            Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                let mut user_input = user_input.lock().unwrap();
                let mut exit = exit.lock().unwrap();
                let mut send = send.lock().unwrap();
                handle_key_event(key_event, &mut exit, &mut user_input, &mut send);
                if *exit { return; }
            }
            _ => { }
        };
        {
            let mut terminal = terminal.lock().unwrap();
            let messages = messages.lock().unwrap();
            let user_input = user_input.lock().unwrap();
            match terminal.draw(|frame| render(frame.area(), frame.buffer_mut(), &user_input, &messages)) {
                Ok(_) => {},
                Err(_) => return
            };
        }
    }
}

fn handle_key_event(key_event: KeyEvent, exit: &mut bool, user_input: &mut Vec<char>, send: &mut bool) {
    match key_event.code {
        KeyCode::Esc => {
            *exit = true;
        }
        KeyCode::Backspace => {
            user_input.pop();
        },
        KeyCode::Char(c) => {
            user_input.push(c);
        }
        KeyCode::Enter => {
            *send = true;
        },
        _ => {}
    };
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
}

pub fn execute_application(application: Application, terminal: DefaultTerminal, client: AM<Client>, hostname: String) {
    let user_input: AMV<char> = sync_vec(application.user_input);
    let terminal: AM<DefaultTerminal> = sync(terminal);
    let exit: AM<bool> = sync(application.exit);
    let send: AM<bool> = sync(application.send);
    let messages: AMV<Message> = sync_vec(application.messages);
    let d_user_input = Arc::clone(&user_input);
    let d_terminal = Arc::clone(&terminal);
    let d_exit = Arc::clone(&exit);
    let d_send = Arc::clone(&send);
    let d_messages = Arc::clone(&messages);
    let network_thread = spawn(move || {
        polling_message_renderer(exit, send, terminal, client, hostname, messages, user_input);
    });
    let event_thread = spawn(move || {
        blocking_event_loop(d_exit, d_user_input, d_send, d_terminal, d_messages);
    });
    let _ = network_thread.join();
    let _ = event_thread.join();
    ratatui::restore();
}
