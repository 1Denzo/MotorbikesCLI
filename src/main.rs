use crossterm::{
    event::{self, Event as CEvent, KeyCode},
    terminal::{disable_raw_mode, enable_raw_mode},
};
use rand::{distributions::Alphanumeric, prelude::*};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};
use serde::ser::SerializeStruct;
use thiserror::Error;
use tui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Span, Spans},
    widgets::{
        Block, BorderType, Borders, Cell, List, ListItem, ListState, Paragraph, Row, Table, Tabs,
    },
    Terminal,
};

const DB_PATH: &str = "C:/Users/denzo/RustroverProjects/MotorbikesCLI/data/moto1.json";

#[derive(Error, Debug)]
pub enum Error {
    #[error("error reading the DB file: {0}")]
    ReadDBError(#[from] io::Error),
    #[error("error parsing the DB file: {0}")]
    ParseDBError(#[from] serde_json::Error),
}

enum Event<I> {
    Input(I),
    Tick,
}

#[derive(Serialize, Deserialize, Clone)]
struct Motorbike {
    id: u32,
    vendor: String,
    model: String,
    year_of_production: u32,
    engine: String,
}

#[derive(Copy, Clone, Debug)]
enum MenuItem {
    Home,
    Motorbikes,
}

impl From<MenuItem> for usize {
    fn from(input: MenuItem) -> usize {
        match input {
            MenuItem::Home => 0,
            MenuItem::Motorbikes => 1,
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    enable_raw_mode().expect("can run in raw mode");

    let (tx, rx) = mpsc::channel();
    let tick_rate = Duration::from_millis(200);
    thread::spawn(move || {
        let mut last_tick = Instant::now();
        loop {
            let timeout = tick_rate
                .checked_sub(last_tick.elapsed())
                .unwrap_or_else(|| Duration::from_secs(0));

            if event::poll(timeout).expect("poll works") {
                if let CEvent::Key(key) = event::read().expect("can read events") {
                    tx.send(Event::Input(key)).expect("can send events");
                }
            }

            if last_tick.elapsed() >= tick_rate {
                if let Ok(_) = tx.send(Event::Tick) {
                    last_tick = Instant::now();
                }
            }
        }
    });

    let stdout = io::stdout();
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;

    let menu_titles = vec!["Home", "Motorbikes", "Add", "Delete", "Quit"];
    let mut active_menu_item = MenuItem::Home;
    let mut pet_list_state = ListState::default();
    pet_list_state.select(Some(0));

    loop {
        terminal.draw(|rect| {
            let size = rect.size();
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .margin(2)
                .constraints(
                    [
                        Constraint::Length(3),
                        Constraint::Min(2),
                        Constraint::Length(3),
                    ]
                        .as_ref(),
                )
                .split(size);

            let copyright = Paragraph::new("- MotorbikesCLI 2024 -")
                .style(Style::default().fg(Color::Yellow))
                .alignment(Alignment::Center)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .style(Style::default().fg(Color::White))
                        .title("Copyright")
                        .border_type(BorderType::Plain),
                );

            let menu = menu_titles
                .iter()
                .map(|t| {
                    let (first, rest) = t.split_at(1);
                    Spans::from(vec![
                        Span::styled(
                            first,
                            Style::default()
                                .fg(Color::Yellow)
                                .add_modifier(Modifier::UNDERLINED),
                        ),
                        Span::styled(rest, Style::default().fg(Color::White)),
                    ])
                })
                .collect();

            let tabs = Tabs::new(menu)
                .select(active_menu_item.into())
                .block(Block::default().title("Menu").borders(Borders::ALL))
                .style(Style::default().fg(Color::White))
                .highlight_style(Style::default().fg(Color::Yellow))
                .divider(Span::raw("|"));

            rect.render_widget(tabs, chunks[0]);
            match active_menu_item {
                MenuItem::Home => rect.render_widget(render_home(), chunks[1]),
                MenuItem::Motorbikes => {
                    let pets_chunks = Layout::default()
                        .direction(Direction::Horizontal)
                        .constraints(
                            [Constraint::Percentage(20), Constraint::Percentage(80)].as_ref(),
                        )
                        .split(chunks[1]);
                    let (left, right) = render_pets(&pet_list_state);
                    rect.render_stateful_widget(left, pets_chunks[0], &mut pet_list_state);
                    rect.render_widget(right, pets_chunks[1]);
                }
            }
            rect.render_widget(copyright, chunks[2]);
        })?;

        match rx.recv()? {
            Event::Input(event) => match event.code {
                KeyCode::Char('q') => {
                    disable_raw_mode()?;
                    terminal.show_cursor()?;
                    break;
                }
                KeyCode::Char('h') => active_menu_item = MenuItem::Home,
                KeyCode::Char('m') => active_menu_item = MenuItem::Motorbikes,
                KeyCode::Char('a') => {
                    add_random_pet_to_db().expect("can add new random motorbike");
                }
                KeyCode::Char('d') => {
                    remove_pet_at_index(&mut pet_list_state).expect("can remove motorbike");
                }
                KeyCode::Down => {
                    if let Some(selected) = pet_list_state.selected() {
                        let amount_pets = read_db().expect("can fetch motorbike list").len();
                        if selected >= amount_pets - 1 {
                            pet_list_state.select(Some(0));
                        } else {
                            pet_list_state.select(Some(selected + 1));
                        }
                    }
                }
                KeyCode::Up => {
                    if let Some(selected) = pet_list_state.selected() {
                        let amount_pets = read_db().expect("can fetch motorbikes list").len();
                        if selected > 0 {
                            pet_list_state.select(Some(selected - 1));
                        } else {
                            pet_list_state.select(Some(amount_pets - 1));
                        }
                    }
                }
                _ => {}
            },
            Event::Tick => {}
        }
    }

    Ok(())
}

fn render_home<'a>() -> Paragraph<'a> {
    let home = Paragraph::new(vec![
        Spans::from(vec![Span::raw("")]),
        Spans::from(vec![Span::raw("Welcome")]),
        Spans::from(vec![Span::raw("")]),
        Spans::from(vec![Span::raw("to")]),
        Spans::from(vec![Span::raw("")]),
        Spans::from(vec![Span::styled(
            "MotorbikesCLI",
            Style::default().fg(Color::LightYellow),
        )]),
        Spans::from(vec![Span::raw("")]),
        Spans::from(vec![Span::raw("Press 'm' to access motorbikes list, 'a' to add random new motorbike and 'd' to delete the currently selected bike.")]),
    ])
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .style(Style::default().fg(Color::White))
                .title("Home")
                .border_type(BorderType::Plain),
        );
    home
}

fn render_pets<'a>(pet_list_state: &ListState) -> (List<'a>, Table<'a>) {
    let pets = Block::default()
        .borders(Borders::ALL)
        .style(Style::default().fg(Color::White))
        .title("Motorbikes")
        .border_type(BorderType::Plain);

    let pet_list = read_db().expect("can fetch motorbikes list");
    let items: Vec<_> = pet_list
        .iter()
        .map(|pet| ListItem::new(Spans::from(vec![Span::styled(
            pet.model.clone(),
            Style::default(),
        )])))
        .collect();

    let selected_pet = pet_list
        .get(
            pet_list_state
                .selected()
                .expect("there is always a selected motorbike"),
        )
        .expect("exists")
        .clone();

    let list = List::new(items).block(pets).highlight_style(
        Style::default()
            .bg(Color::Yellow)
            .fg(Color::Black)
            .add_modifier(Modifier::BOLD),
    );

    let pet_detail = Table::new(vec![Row::new(vec![
        Cell::from(Span::raw(selected_pet.id.to_string())),
        Cell::from(Span::raw(selected_pet.vendor.to_string())),
        Cell::from(Span::raw(selected_pet.model.to_string())),
        Cell::from(Span::raw(selected_pet.year_of_production.to_string())),
        Cell::from(Span::raw(selected_pet.engine.to_string()))
    ])])
        .header(Row::new(vec![
            Cell::from(Span::styled(
                "ID",
                Style::default().add_modifier(Modifier::BOLD),
            )),
            Cell::from(Span::styled(
                "Vendor",
                Style::default().add_modifier(Modifier::BOLD),
            )),
            Cell::from(Span::styled(
                "Model",
                Style::default().add_modifier(Modifier::BOLD),
            )),
            Cell::from(Span::styled(
                "Year of production",
                Style::default().add_modifier(Modifier::BOLD),
            )),
                Cell::from(Span::styled(
                "Engine",
                Style::default().add_modifier(Modifier::BOLD)
            ))
        ]))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .style(Style::default().fg(Color::White))
                .title("Detail")
                .border_type(BorderType::Plain),
        )
        .widths(&[
            Constraint::Percentage(5),
            Constraint::Percentage(20),
            Constraint::Percentage(30),
            Constraint::Percentage(25),
            Constraint::Percentage(20),
        ]);

    (list, pet_detail)
}

fn read_db() -> Result<Vec<Motorbike>, Error> {
    let db_content = fs::read_to_string(DB_PATH)?;
    let parsed: Vec<Motorbike> = serde_json::from_str(&db_content)?;
    Ok(parsed)
}

fn add_random_pet_to_db() -> Result<Vec<Motorbike>, Error> {
    let mut rng = rand::thread_rng();
    let db_content = fs::read_to_string(DB_PATH)?;
    let mut parsed: Vec<Motorbike> = serde_json::from_str(&db_content)?;

    let random_pet = Motorbike {
        id: rng.gen_range(0, 9999999),
        vendor: rng.sample_iter(Alphanumeric).take(10).collect(),
        model: rng.sample_iter(Alphanumeric).take(10).collect(),
        year_of_production: rng.gen_range(1970, 2024),
        engine: rng.sample_iter(Alphanumeric).take(10).collect(),
    };

    parsed.push(random_pet);
    fs::write(DB_PATH, &serde_json::to_vec(&parsed)?)?;
    Ok(parsed)
}

fn remove_pet_at_index(pet_list_state: &mut ListState) -> Result<(), Error> {
    if let Some(selected) = pet_list_state.selected() {
        let db_content = fs::read_to_string(DB_PATH)?;
        let mut parsed: Vec<Motorbike> = serde_json::from_str(&db_content)?;
        parsed.remove(selected);
        fs::write(DB_PATH, &serde_json::to_vec(&parsed)?)?;
        let amount_pets = read_db().expect("can fetch motorbikes list").len();
        if selected > 0 {
            pet_list_state.select(Some(selected - 1));
        } else {
            pet_list_state.select(Some(0));
        }
    }
    Ok(())
}