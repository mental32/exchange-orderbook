use crossterm::event::KeyCode;
use ratatui::{prelude::*, widgets::*};
use thiserror::Error;

/// # Usage
///
/// ```rust
/// let rect = centered_rect(f.size(), 50, 50);
/// ```
fn centered_rect(r: Rect, percent_x: u16, percent_y: u16) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Focus {
    total: usize,
    selected: usize,
    selected_style: Style,
    default_style: Style,
}

impl Focus {
    fn new(total: usize) -> Self {
        Self {
            total,
            selected: 0,
            selected_style: Style::default().fg(Color::Yellow),
            default_style: Style::default(),
        }
    }

    fn focus_next_wrapping(&mut self) {
        if self.selected == self.total {
            self.selected = 0;
        } else {
            self.selected += 1;
        }
    }

    fn focus_prev_wrapping(&mut self) {
        if self.selected == 0 {
            self.selected = self.total;
        } else {
            self.selected -= 1;
        }
    }

    fn is_last(&self) -> bool {
        self.selected == (self.total - 1)
    }

    fn get_selected_mut<'a, T>(&self, lst: &'a mut [T]) -> Option<&'a mut T> {
        lst.get_mut(self.selected)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InputType {
    Text,
    Password,
}

type InputF = for<'a> fn(&'a str, Style, InputType) -> Vec<Span<'a>>;

#[derive(Clone)]
struct Input {
    f: InputF,
    buf: String,
    style: Style,
    kind: InputType,
}

impl Input {
    fn new(f: InputF, buf: String, kind: InputType) -> Self {
        Self {
            f,
            buf,
            kind,
            style: Style::default(),
        }
    }

    fn into_line(&self) -> Line {
        Line {
            spans: (self.f)(&self.buf, self.style, self.kind),
            alignment: None,
        }
    }
}

trait StringExt {
    fn clip(&self, length: usize) -> String;
    fn fill(&self, ch: char, max: usize) -> String;
}

impl<S: std::ops::Deref<Target = str>> StringExt for S {
    fn clip(&self, length: usize) -> String {
        // show the first `length` characters of the string otherwise insert "[N more.]" at the end
        let size = self.chars().count();
        if size > length {
            let trailer = format!("[{} more.]", size - length);
            let trailer_len = trailer.chars().count();

            self.chars()
                .take(length - trailer_len)
                .chain(trailer.chars())
                .collect()
        } else {
            self.to_string()
        }
    }

    fn fill(&self, ch: char, max: usize) -> String {
        let size = self.chars().count();
        if size < max {
            format!("{}{}", self.deref(), ch.to_string().repeat(max - size))
        } else {
            self.to_string()
        }
    }
}

struct NotifAlertWindow {
    block: Block<'static>,
    text: Paragraph<'static>,
}

impl Window for NotifAlertWindow {
    fn draw(&self, frame: &mut Frame<'_>) {
        let area = centered_rect(frame.size(), 25, 15);
        for x in area.x..(area.x + area.width) {
            for y in area.y..(area.y + area.height) {
                frame.buffer_mut().get_mut(x, y).reset();
            }
        }

        frame.render_widget(self.block.clone(), area);
        frame.render_widget(self.text.clone(), self.block.inner(area));
    }

    fn handle_events(&mut self) -> Result<(), WindowError> {
        if crossterm::event::poll(std::time::Duration::from_millis(100)).unwrap() {
            // If a key event occurs, handle it
            if let crossterm::event::Event::Key(key) = crossterm::event::read().unwrap() {
                if key.kind == crossterm::event::KeyEventKind::Press {
                    match key.code {
                        KeyCode::Esc => return Err(WindowError::Exit),
                        KeyCode::Enter => return Err(WindowError::PopWindow),
                        key => unimplemented!("{key:?}"),
                    }
                }
            }

            Ok(())
        } else {
            Ok(())
        }
    }
}

struct MainWindow {}

impl MainWindow {
    pub fn new() -> Box<dyn Window> {
        let this = Self {};

        Box::new(this)
    }
}

impl Window for MainWindow {
    fn handle_events(&mut self) -> Result<(), WindowError> {
        Ok(())
    }

    fn draw(&self, frame: &mut Frame<'_>) {
        // draw a block for the whole screen
        let block = Block::default().borders(Borders::ALL).title("Exchange");
        frame.render_widget(block, frame.size());
    }
}

#[derive(Debug, Error)]
#[error("exit")]
struct Exit;

struct LoginWindow {
    block: Block<'static>,
    focus: Focus,
    exchange_url: Input,
    username: Input,
    password: Input,
}

pub const DEFAULT_EXCHANGE_API_URL: &'static str = "http://localhost:8080";

impl LoginWindow {
    pub fn new() -> Box<dyn Window> {
        fn render_username(buf: &str, style: Style, _: InputType) -> Vec<Span<'_>> {
            vec![Span::styled("username: ", style), Span::styled(buf, style)]
        }

        let this = Self {
            block: Block::new().borders(Borders::ALL).title("Login"),
            focus: Focus::new(3),
            exchange_url: Input::new(
                |buf, style, _| {
                    vec![
                        Span::styled("exchange-api: ", style),
                        Span::styled(buf, style),
                    ]
                },
                String::from(DEFAULT_EXCHANGE_API_URL),
                InputType::Text,
            ),
            username: Input::new(render_username, String::new(), InputType::Text),
            password: Input::new(
                |buf, style, kind| {
                    assert_eq!(kind, InputType::Password);
                    vec![
                        Span::styled("password: ", style),
                        Span::styled("*".repeat(buf.chars().count()), style),
                    ]
                },
                String::new(),
                InputType::Password,
            ),
        };

        Box::new(this)
    }
}

impl Window for LoginWindow {
    fn handle_events(&mut self) -> Result<(), WindowError> {
        // Check for user input every 250 milliseconds
        if crossterm::event::poll(std::time::Duration::from_millis(100))
            .map_err(WindowError::other)?
        {
            // If a key event occurs, handle it
            if let crossterm::event::Event::Key(key) = crossterm::event::read().unwrap() {
                if key.kind == crossterm::event::KeyEventKind::Press {
                    let mut lst = [
                        &mut self.exchange_url,
                        &mut self.username,
                        &mut self.password,
                    ];

                    match key.code {
                        KeyCode::Char(c) => {
                            if let Some(t) =
                                self.focus.get_selected_mut(&mut lst as &mut [&mut Input])
                            {
                                t.buf.push(c);
                            }
                        }
                        KeyCode::BackTab => {
                            self.focus.focus_prev_wrapping();
                        }
                        KeyCode::Backspace => {
                            if let Some(t) =
                                self.focus.get_selected_mut(&mut lst as &mut [&mut Input])
                            {
                                t.buf.pop();
                            }
                        }
                        KeyCode::Enter => {
                            if self.focus.is_last() {
                                return Err(WindowError::PushWindow(Box::new(NotifAlertWindow {
                                    block: Block::new()
                                        .borders(Borders::ALL)
                                        .title("Alert")
                                        .on_blue(),
                                    text: Paragraph::new("Not implemented yet!"),
                                })));
                            } else {
                                self.focus.focus_next_wrapping();
                            }
                        }
                        KeyCode::Tab => {
                            self.focus.focus_next_wrapping();
                        }
                        KeyCode::Esc => return Err(WindowError::Exit),
                        KeyCode::Down => {
                            self.focus.focus_next_wrapping();
                        }
                        KeyCode::Up => {
                            self.focus.focus_prev_wrapping();
                        }
                        KeyCode::Right => {
                            self.focus.focus_next_wrapping();
                        }
                        KeyCode::Left => {
                            self.focus.focus_prev_wrapping();
                        }
                        key => unimplemented!("{key:?}"),
                    }
                }
            }
        }

        Ok(())
    }

    fn draw(&self, frame: &mut Frame<'_>) {
        let area = centered_rect(frame.size(), 30, 30);

        let mut lines = vec![
            self.exchange_url.into_line(),
            self.username.into_line(),
            self.password.into_line(),
        ];

        if let Some(line) = self.focus.get_selected_mut(&mut lines) {
            line.spans.insert(0, Span::styled("> ", Style::default()));

            match line.spans.as_mut_slice() {
                [first, second, ..] => {
                    first.style = self.focus.selected_style;
                    second.style = self.focus.selected_style;
                }
                _ => (),
            }
        }

        let text = Paragraph::new(lines).wrap(Wrap { trim: true });

        frame.render_widget(text, self.block.inner(area));
        frame.render_widget(self.block.clone(), area);
    }
}

enum WindowError {
    Exit,
    PushWindow(Box<dyn Window>),
    PopWindow,
    Other(Box<dyn std::error::Error>),
}

impl WindowError {
    fn other<E: std::error::Error + 'static>(e: E) -> Self {
        Self::Other(Box::new(e))
    }
}

trait Window {
    fn draw(&self, frame: &mut Frame<'_>);
    fn handle_events(&mut self) -> Result<(), WindowError>;
}

enum Mode {
    Login,
    Main,
}

struct App {
    windows: Vec<Box<dyn Window>>,
    mode: Mode,
}
impl App {
    fn draw(&self, f: &mut Frame<'_>) {
        for window in &self.windows {
            window.draw(f);
        }
    }

    fn handle_events(&mut self) -> Result<(), WindowError> {
        if let Some(top) = self.windows.last_mut() {
            top.handle_events()?;
        }

        Ok(())
    }
}

const DEFAULT_LOGIN_TEXT_FILL: usize = 32;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // startup: Enable raw mode for the terminal, giving us fine control over user input
    crossterm::terminal::enable_raw_mode()?;
    crossterm::execute!(std::io::stderr(), crossterm::terminal::EnterAlternateScreen)?;

    // Initialize the terminal backend using crossterm
    let terminal = Terminal::new(CrosstermBackend::new(std::io::stderr()))?;

    let app = App {
        windows: vec![MainWindow::new(), LoginWindow::new()],
        mode: Mode::Login,
    };

    fn main_loop<W: std::io::Write>(
        mut terminal: Terminal<CrosstermBackend<W>>,
        mut app: App,
    ) -> Result<(), Box<dyn std::error::Error>> {
        loop {
            terminal.draw(|f| app.draw(f))?;
            match app.handle_events() {
                Ok(()) => continue,
                Err(WindowError::PushWindow(window)) => {
                    app.windows.push(window);
                }
                Err(WindowError::PopWindow) => {
                    app.windows.pop();
                }
                Err(WindowError::Other(err)) => return Err(err.into()),
                Err(WindowError::Exit) => return Ok(()),
            }
        }
    }

    let res = main_loop(terminal, app);

    // shutdown down: reset terminal back to original state
    crossterm::execute!(std::io::stderr(), crossterm::terminal::LeaveAlternateScreen)?;
    crossterm::terminal::disable_raw_mode()?;

    res
}
