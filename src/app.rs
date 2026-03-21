use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::{
    DefaultTerminal, Frame,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
};
use tokio::runtime::Runtime;

use crate::{
    api,
    conf::{self},
};

struct BookmarkItem {
    bookmark: api::Bookmark,
    selected_tags: Vec<String>,
    selected_lists: Vec<String>,
    dirty: bool,
}

pub struct App {
    // config
    config: conf::Config,
    // api
    bookmarks: Vec<BookmarkItem>,
    available_tags: Vec<String>,
    available_lists: Vec<String>,
    // state
    mode: Mode,
    list_state: ListState,
    edit_state: ListState,
    input: String,
    suggestions: Vec<String>,
    suggestion_index: usize,
    status_message: String,
    exit: bool,
}

#[derive(PartialEq)]
enum Mode {
    Normal,
    TagInput,
    TagEdit,
    ListInput,
    ListEdit,
}

impl App {
    pub fn new(
        config: conf::Config,
        bookmarks: &[api::Bookmark],
        available_tags: &[String],
        available_lists: &[String],
    ) -> Self {
        let mut app = Self {
            config,
            bookmarks: bookmarks
                .iter()
                .map(|b| BookmarkItem {
                    selected_tags: b.tags.clone(),
                    selected_lists: b.lists.clone(),
                    bookmark: b.clone(),
                    dirty: false,
                })
                .collect(),
            available_tags: available_tags.to_owned(),
            available_lists: available_lists.to_owned(),

            list_state: ListState::default(),
            edit_state: ListState::default(),
            input: String::new(),
            suggestions: Vec::new(),
            suggestion_index: 0,

            mode: Mode::Normal,
            status_message: format!(
                "Loaded {} bookmarks, {} tags, {} lists",
                bookmarks.len(),
                available_tags.len(),
                available_lists.len()
            ),
            exit: false,
        };
        app.list_state.select(Some(0));
        app
    }

    pub fn run(&mut self, terminal: &mut DefaultTerminal) -> Result<()> {
        while !self.exit {
            terminal.draw(|frame| self.draw(frame))?;
            self.handle_events()?;
        }
        Ok(())
    }

    fn draw(&mut self, frame: &mut Frame) {
        let middle_height = if self.mode == Mode::TagEdit {
            if let Some(idx) = self.list_state.selected()
                && let Some(item) = self.bookmarks.get(idx)
            {
                (item.selected_tags.len() + 2).clamp(3, 10) as u16
            } else {
                3
            }
        } else if self.mode == Mode::ListEdit {
            if let Some(idx) = self.list_state.selected()
                && let Some(item) = self.bookmarks.get(idx)
            {
                (item.selected_lists.len() + 2).clamp(3, 10) as u16
            } else {
                3
            }
        } else if self.mode == Mode::Normal {
            0
        } else {
            3
        };

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(5),
                Constraint::Length(middle_height),
                Constraint::Length(3),
            ])
            .split(frame.area());

        // Bookmarks list
        let items: Vec<ListItem> = self
            .bookmarks
            .iter()
            .map(|item| {
                let dirty_marker = if item.dirty { "*" } else { " " };
                let tags_str = if item.selected_tags.is_empty() {
                    "#".to_string()
                } else {
                    format!("#{}", item.selected_tags.join(" #"))
                };
                let lists_str = if item.selected_lists.is_empty() {
                    "/".to_string()
                } else {
                    format!("/{}", item.selected_lists.join(" /"))
                };

                let content = vec![Line::from(vec![
                    Span::styled(dirty_marker, Style::default().fg(Color::Yellow)),
                    Span::raw(" "),
                    Span::styled(lists_str, Style::default().fg(Color::Blue)),
                    Span::raw(" "),
                    Span::styled(tags_str, Style::default().fg(Color::Green)),
                    Span::raw(" "),
                    Span::styled(&item.bookmark.title, Style::default().fg(Color::Cyan)),
                ])];
                ListItem::new(content)
            })
            .collect();

        let list = List::new(items)
            .block(Block::default().borders(Borders::ALL).title("Bookmarks"))
            .highlight_style(
                Style::default()
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol(">> ");

        frame.render_stateful_widget(list, chunks[0], &mut self.list_state);

        // Middle section: Tag input or Tag edit mode
        match self.mode {
            Mode::TagInput | Mode::ListInput => {
                let mode_name = if self.mode == Mode::TagInput {
                    "Tags"
                } else {
                    "Lists"
                };
                let input_block = Block::default()
                    .borders(Borders::ALL)
                    .title(format!(
                        "Add {} (Enter to confirm, Esc to cancel)",
                        mode_name
                    ))
                    .border_style(Style::default().fg(Color::Yellow));

                let input_text = format!("> {}", self.input);
                let input = Paragraph::new(input_text).block(input_block);
                frame.render_widget(input, chunks[1]);
            }
            Mode::TagEdit | Mode::ListEdit => {
                let mode_name = if self.mode == Mode::TagEdit {
                    "Tags"
                } else {
                    "Lists"
                };
                if let Some(idx) = self.list_state.selected()
                    && let Some(item) = self.bookmarks.get(idx)
                {
                    let source_iter = if self.mode == Mode::TagEdit {
                        item.selected_tags.iter()
                    } else {
                        item.selected_lists.iter()
                    };
                    let items: Vec<ListItem> = source_iter
                        .map(|item| {
                            ListItem::new(Line::from(vec![
                                Span::raw("  "),
                                Span::styled(item, Style::default().fg(Color::Green)),
                            ]))
                        })
                        .collect();

                    let list = List::new(items)
                        .block(
                            Block::default()
                                .borders(Borders::ALL)
                                .title(format!("Edit {} (d to delete, Esc to exit)", mode_name))
                                .border_style(Style::default().fg(Color::Yellow)),
                        )
                        .highlight_style(
                            Style::default()
                                .bg(Color::DarkGray)
                                .add_modifier(Modifier::BOLD),
                        )
                        .highlight_symbol(">> ");

                    frame.render_stateful_widget(list, chunks[1], &mut self.edit_state);
                }
            }
            Mode::Normal => {}
        }

        // Bottom section: Suggestions or status
        let bottom_content = if self.mode == Mode::TagInput || self.mode == Mode::ListInput {
            let suggestions: Vec<Span> = self
                .suggestions
                .iter()
                .enumerate()
                .map(|(i, val)| {
                    if i == self.suggestion_index {
                        Span::styled(
                            format!("[{}]", val),
                            Style::default().bg(Color::DarkGray).fg(Color::Yellow),
                        )
                    } else {
                        Span::raw(format!(" {} ", val))
                    }
                })
                .collect();
            Line::from(suggestions)
        } else {
            Line::from(vec![
                Span::raw("Status: "),
                Span::styled(&self.status_message, Style::default().fg(Color::Green)),
            ])
        };

        let help_text = match self.mode {
            Mode::Normal => {
                "[↑/↓] Navigate | [o] Open URL | [t] Add tag | [T] Edit tags | [l] Add list | [L] Edit lists | [s] Save all | [q] Quit"
            }
            Mode::TagInput | Mode::ListInput => {
                "[Tab] Next suggestion | [Shift+Tab] Prev | [Enter] Confirm | [Esc] Cancel"
            }
            Mode::TagEdit | Mode::ListEdit => "[↑/↓] Select | [d] Delete | [Esc] Exit edit mode",
        };

        let bottom = Paragraph::new(bottom_content)
            .block(Block::default().borders(Borders::ALL).title(help_text));
        frame.render_widget(bottom, chunks[2]);
    }

    fn handle_events(&mut self) -> Result<()> {
        if let Event::Key(key) = event::read()? {
            if key.kind != KeyEventKind::Press {
                return Ok(());
            }

            match self.mode {
                Mode::Normal => match key.code {
                    KeyCode::Char('q') => self.exit(),
                    KeyCode::Down | KeyCode::Char('j') => self.next(),
                    KeyCode::Up | KeyCode::Char('k') => self.previous(),
                    KeyCode::Char('o') => self.open_link(),
                    KeyCode::Char('t') => {
                        self.mode = Mode::TagInput;
                        self.update_tag_suggestions();
                    }
                    KeyCode::Char('T') => self.enter_tag_edit_mode(),
                    KeyCode::Char('l') => {
                        self.mode = Mode::ListInput;
                        self.update_list_suggestions();
                    }
                    KeyCode::Char('L') => self.enter_list_edit_mode(),
                    KeyCode::Char('s') => {
                        let dirty_count = self.bookmarks.iter().filter(|b| b.dirty).count();
                        let mut dirty_bookmarks: Vec<&api::Bookmark> = Vec::new();
                        for item in &mut self.bookmarks {
                            if item.dirty {
                                item.bookmark.tags = item.selected_tags.clone();
                                item.bookmark.lists = item.selected_lists.clone();
                                item.dirty = false;
                                dirty_bookmarks.push(&item.bookmark);
                            }
                        }
                        self.status_message = format!("Saving {} bookmarks...", dirty_count);
                        let rt = Runtime::new()?;
                        rt.block_on(async {
                            api::save_bookmarks(&self.config, &dirty_bookmarks).await
                        })?;
                        self.status_message = format!("Saved {} bookmarks!", dirty_count);
                    }
                    _ => {}
                },
                Mode::TagInput => match key.code {
                    KeyCode::Esc => {
                        self.mode = Mode::Normal;
                        self.input.clear();
                    }
                    KeyCode::Enter => {
                        self.add_tag_to_current();
                    }
                    KeyCode::Tab => {
                        self.next_suggestion();
                    }
                    KeyCode::BackTab => {
                        self.prev_suggestion();
                    }
                    KeyCode::Char(c) => {
                        self.input.push(c);
                        self.update_tag_suggestions();
                    }
                    KeyCode::Backspace => {
                        self.input.pop();
                        self.update_tag_suggestions();
                    }
                    _ => {}
                },
                Mode::TagEdit => match key.code {
                    KeyCode::Esc => {
                        self.mode = Mode::Normal;
                    }
                    KeyCode::Down | KeyCode::Char('j') => self.next_tag(),
                    KeyCode::Up | KeyCode::Char('k') => self.previous_tag(),
                    KeyCode::Char('d') => self.remove_selected_tag(),
                    _ => {}
                },
                Mode::ListInput => match key.code {
                    KeyCode::Esc => {
                        self.mode = Mode::Normal;
                        self.input.clear();
                    }
                    KeyCode::Enter => {
                        self.add_list_to_current();
                    }
                    KeyCode::Tab => {
                        self.next_suggestion();
                    }
                    KeyCode::BackTab => {
                        self.prev_suggestion();
                    }
                    KeyCode::Char(c) => {
                        self.input.push(c);
                        self.update_list_suggestions();
                    }
                    KeyCode::Backspace => {
                        self.input.pop();
                        self.update_list_suggestions();
                    }
                    _ => {}
                },
                Mode::ListEdit => match key.code {
                    KeyCode::Esc => {
                        self.mode = Mode::Normal;
                    }
                    KeyCode::Down | KeyCode::Char('j') => self.next_list(),
                    KeyCode::Up | KeyCode::Char('k') => self.previous_list(),
                    KeyCode::Char('d') => self.remove_selected_list(),
                    _ => {}
                },
            }
        }
        Ok(())
    }

    fn next(&mut self) {
        if self.bookmarks.is_empty() {
            return;
        }
        let i = match self.list_state.selected() {
            Some(i) => (i + 1) % self.bookmarks.len(),
            None => 0,
        };
        self.list_state.select(Some(i));
    }

    fn previous(&mut self) {
        if self.bookmarks.is_empty() {
            return;
        }
        let len = self.bookmarks.len();
        let i = match self.list_state.selected() {
            Some(i) => (len + i - 1) % len,
            None => 0,
        };
        self.list_state.select(Some(i));
    }

    fn open_link(&mut self) {
        if let Some(idx) = self.list_state.selected()
            && let Some(item) = self.bookmarks.get(idx)
            && open::that(&item.bookmark.url).is_err()
        {
            self.status_message = format!("Failed to open {}", item.bookmark.url);
        }
    }

    fn update_tag_suggestions(&mut self) {
        if !self.input.is_empty() {
            self.suggestions = vec![self.input.clone()];
        } else {
            self.suggestions = Vec::new();
        }

        let current_tags = if let Some(bookmark_idx) = self.list_state.selected() {
            self.bookmarks[bookmark_idx].selected_tags.clone()
        } else {
            Vec::new()
        };

        let suggestions: Vec<String> = self
            .available_tags
            .iter()
            .filter(|tag| {
                tag.to_lowercase().contains(&self.input.to_lowercase())
                    && tag != &&self.input
                    && !current_tags.contains(tag)
            })
            .cloned()
            .collect();
        self.suggestions.extend(suggestions);
        self.suggestion_index = 0;
    }

    fn add_tag_to_current(&mut self) {
        if let Some(idx) = self.list_state.selected()
            && let Some(item) = self.bookmarks.get_mut(idx)
        {
            let tag =
                if !self.suggestions.is_empty() && self.suggestion_index < self.suggestions.len() {
                    self.suggestions[self.suggestion_index].clone()
                } else {
                    return;
                };

            if !item.selected_tags.contains(&tag) {
                item.selected_tags.push(tag.clone());
                item.dirty = true;

                if !self.available_tags.contains(&tag) {
                    self.available_tags.push(tag.clone());
                }
            }

            self.input.clear();
            self.status_message = format!("Added tag: {}", tag);
            self.update_tag_suggestions();
        }
    }

    fn enter_tag_edit_mode(&mut self) {
        if let Some(idx) = self.list_state.selected()
            && let Some(item) = self.bookmarks.get(idx)
        {
            if !item.selected_tags.is_empty() {
                self.mode = Mode::TagEdit;
                self.edit_state.select(Some(0));
            } else {
                self.status_message = "No tags to edit".to_string();
            }
        }
    }

    fn next_tag(&mut self) {
        if let Some(idx) = self.list_state.selected()
            && let Some(item) = self.bookmarks.get(idx)
        {
            let num_tags = item.selected_tags.len();
            if num_tags == 0 {
                return;
            }
            let i = match self.edit_state.selected() {
                Some(i) => (i + 1) % num_tags,
                None => 0,
            };
            self.edit_state.select(Some(i));
        }
    }

    fn previous_tag(&mut self) {
        if let Some(idx) = self.list_state.selected()
            && let Some(item) = self.bookmarks.get(idx)
        {
            let num_tags = item.selected_tags.len();
            if num_tags == 0 {
                return;
            }
            let i = match self.edit_state.selected() {
                Some(i) => (num_tags + i - 1) % num_tags,
                None => 0,
            };
            self.edit_state.select(Some(i));
        }
    }

    fn remove_selected_tag(&mut self) {
        if let Some(bookmark_idx) = self.list_state.selected()
            && let Some(tag_idx) = self.edit_state.selected()
            && let Some(item) = self.bookmarks.get_mut(bookmark_idx)
            && tag_idx < item.selected_tags.len()
        {
            let removed = item.selected_tags.remove(tag_idx);
            item.dirty = true;
            self.status_message = format!("Removed tag: {}", removed);

            // Adjust selection after removal
            if item.selected_tags.is_empty() {
                self.mode = Mode::Normal;
            } else if tag_idx >= item.selected_tags.len() {
                self.edit_state.select(Some(item.selected_tags.len() - 1));
            }
        }
    }

    fn update_list_suggestions(&mut self) {
        let current_lists = if let Some(bookmark_idx) = self.list_state.selected() {
            self.bookmarks[bookmark_idx].selected_lists.clone()
        } else {
            Vec::new()
        };

        self.suggestions = self
            .available_lists
            .iter()
            .filter(|list| {
                list.to_lowercase().contains(&self.input.to_lowercase())
                    && list != &&self.input
                    && !current_lists.contains(list)
            })
            .cloned()
            .collect();
        self.suggestion_index = 0;
    }

    fn add_list_to_current(&mut self) {
        if let Some(idx) = self.list_state.selected()
            && let Some(item) = self.bookmarks.get_mut(idx)
        {
            let list =
                if !self.suggestions.is_empty() && self.suggestion_index < self.suggestions.len() {
                    self.suggestions[self.suggestion_index].clone()
                } else {
                    return;
                };

            if !item.selected_lists.contains(&list) {
                item.selected_lists.push(list.clone());
                item.dirty = true;

                if !self.available_lists.contains(&list) {
                    self.available_lists.push(list.clone());
                }
            }

            self.input.clear();
            self.status_message = format!("Added list: {}", list);
            self.update_list_suggestions();
        }
    }

    fn enter_list_edit_mode(&mut self) {
        if let Some(idx) = self.list_state.selected()
            && let Some(item) = self.bookmarks.get(idx)
        {
            if !item.selected_lists.is_empty() {
                self.mode = Mode::ListEdit;
                self.edit_state.select(Some(0));
            } else {
                self.status_message = "No lists to edit".to_string();
            }
        }
    }

    fn next_list(&mut self) {
        if let Some(idx) = self.list_state.selected()
            && let Some(item) = self.bookmarks.get(idx)
        {
            let num_lists = item.selected_lists.len();
            if num_lists == 0 {
                return;
            }
            let i = match self.edit_state.selected() {
                Some(i) => (i + 1) % num_lists,
                None => 0,
            };
            self.edit_state.select(Some(i));
        }
    }

    fn previous_list(&mut self) {
        if let Some(idx) = self.list_state.selected()
            && let Some(item) = self.bookmarks.get(idx)
        {
            let num_lists = item.selected_lists.len();
            if num_lists == 0 {
                return;
            }
            let i = match self.edit_state.selected() {
                Some(i) => (num_lists + i - 1) % num_lists,
                None => 0,
            };
            self.edit_state.select(Some(i));
        }
    }

    fn remove_selected_list(&mut self) {
        if let Some(bookmark_idx) = self.list_state.selected()
            && let Some(list_idx) = self.edit_state.selected()
            && let Some(item) = self.bookmarks.get_mut(bookmark_idx)
            && list_idx < item.selected_lists.len()
        {
            let removed = item.selected_lists.remove(list_idx);
            item.dirty = true;
            self.status_message = format!("Removed list: {}", removed);

            // Adjust selection after removal
            if item.selected_lists.is_empty() {
                self.mode = Mode::Normal;
            } else if list_idx >= item.selected_lists.len() {
                self.edit_state.select(Some(item.selected_lists.len() - 1));
            }
        }
    }

    fn next_suggestion(&mut self) {
        if !self.suggestions.is_empty() {
            self.suggestion_index = (self.suggestion_index + 1) % self.suggestions.len();
        }
    }

    fn prev_suggestion(&mut self) {
        if !self.suggestions.is_empty() {
            let len = self.suggestions.len();
            self.suggestion_index = (len + self.suggestion_index - 1) % len;
        }
    }

    fn exit(&mut self) {
        self.exit = true;
    }
}
