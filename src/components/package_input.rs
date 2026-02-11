use std::time::{Duration, Instant};

use color_eyre::eyre;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Style};
use ratatui::widgets::{Block, Paragraph};
use ratatui::Frame;

use crate::action::Action;
use crate::components::Component;
use crate::layout::{INPUT_HEIGHT, LEFT_PANEL_PERCENT};
use crate::pacman::SearchMode;
use crate::theme::Theme;

const AUR_DEBOUNCE_MS: u64 = 300;

pub(crate) struct PackageInput {
    text: String,
    theme: Theme,
    active: bool,
    last_input: Option<Instant>,
    pending_aur_search: bool,
    search_mode: SearchMode,
    loading: bool,
}

impl Default for PackageInput {
    fn default() -> Self {
        Self {
            text: Default::default(),
            theme: Default::default(),
            active: true,
            last_input: None,
            pending_aur_search: false,
            search_mode: SearchMode::Package,
            loading: false,
        }
    }
}

impl PackageInput {
    /// Check if we should trigger AUR search (debounce elapsed)
    pub(crate) fn should_search_aur(&mut self) -> Option<String> {
        // No AUR search in File mode
        if self.search_mode == SearchMode::File {
            return None;
        }

        if self.pending_aur_search
            && let Some(last) = self.last_input
            && last.elapsed() >= Duration::from_millis(AUR_DEBOUNCE_MS)
        {
            self.pending_aur_search = false;
            tracing::debug!(query = %self.text, "AUR debounce elapsed, triggering AUR search");
            return Some(self.text.clone());
        }
        None
    }

    pub(crate) fn set_loading(&mut self, loading: bool) {
        tracing::debug!(loading, "set loading state");
        self.loading = loading;
    }
}

impl Component for PackageInput {
    fn handle_key_event(&mut self, key_event: &KeyEvent) -> eyre::Result<Option<Vec<Action>>> {
        if !self.active {
            return Ok(None);
        }

        // Ctrl+f: toggle search mode (keep text)
        if key_event.modifiers == KeyModifiers::CONTROL && key_event.code == KeyCode::Char('f') {
            self.search_mode = match self.search_mode {
                SearchMode::Package => {
                    tracing::info!("switched to file search mode");
                    SearchMode::File
                }
                SearchMode::File => {
                    tracing::info!("switched to package search mode");
                    SearchMode::Package
                }
            };
            self.pending_aur_search = false;
            self.last_input = None;

            // When switching to package mode, trigger instant search with current text
            if self.search_mode == SearchMode::Package {
                self.pending_aur_search = true;
                self.last_input = Some(Instant::now());
                return Ok(Some(vec![Action::SearchPackage(self.text.clone())]));
            }
            // When switching to file mode, clear results (user needs to press Enter)
            return Ok(Some(vec![Action::SearchPackage(String::new())]));
        }

        // In File mode, Enter triggers search
        if self.search_mode == SearchMode::File {
            match *key_event {
                KeyEvent {
                    modifiers: KeyModifiers::NONE,
                    code: KeyCode::Char(char),
                    ..
                } => {
                    self.text.push(char);
                }
                KeyEvent {
                    modifiers: KeyModifiers::NONE,
                    code: KeyCode::Backspace,
                    ..
                } => {
                    self.text.pop();
                }
                KeyEvent {
                    modifiers: KeyModifiers::CONTROL,
                    code: KeyCode::Char('w'),
                    ..
                } => {
                    if let Some((prefix, _)) = self.text.rsplit_once(' ') {
                        self.text = prefix.to_string();
                    } else {
                        self.text.clear();
                    }
                }
                KeyEvent {
                    modifiers: KeyModifiers::NONE,
                    code: KeyCode::Enter,
                    ..
                } => {
                    if !self.text.is_empty() {
                        tracing::info!(query = %self.text, "file search triggered via Enter");
                        return Ok(Some(vec![Action::SearchFile(self.text.clone())]));
                    }
                }
                _ => {}
            }
            return Ok(Some(Vec::new()));
        }

        // Package mode: instant search per keystroke
        let mut text_changed = false;

        match *key_event {
            KeyEvent {
                modifiers: KeyModifiers::NONE,
                code: KeyCode::Char(char),
                ..
            } => {
                self.text.push(char);
                text_changed = true;
            }
            KeyEvent {
                modifiers: KeyModifiers::NONE,
                code: KeyCode::Backspace,
                ..
            } => {
                self.text.pop();
                text_changed = true;
            }
            KeyEvent {
                modifiers: KeyModifiers::CONTROL,
                code: KeyCode::Char('w'),
                ..
            } => {
                if let Some((prefix, _)) = self.text.rsplit_once(' ') {
                    self.text = prefix.to_string();
                } else {
                    self.text.clear();
                }
                text_changed = true;
            }
            _ => {}
        }

        if text_changed {
            self.last_input = Some(Instant::now());
            self.pending_aur_search = true;
            tracing::trace!(text = %self.text, "input changed, immediate pacman search");
            // Return SearchPackage immediately for instant pacman results
            return Ok(Some(vec![Action::SearchPackage(self.text.clone())]));
        }

        Ok(Some(Vec::new()))
    }

    fn draw(&mut self, frame: &mut Frame, area: &Rect) -> eyre::Result<()> {
        let horizontal_layout = Layout::horizontal([
            Constraint::Percentage(LEFT_PANEL_PERCENT),
            Constraint::Percentage(100 - LEFT_PANEL_PERCENT),
        ])
        .split(*area)[0];
        let area = Layout::vertical([Constraint::Length(INPUT_HEIGHT), Constraint::Percentage(100)])
            .split(horizontal_layout)[0];

        let (border_color, title) = match self.search_mode {
            SearchMode::Package => {
                let color = if self.active {
                    self.theme.active
                } else {
                    self.theme.inactive
                };
                (color, String::new())
            }
            SearchMode::File => {
                let color = if self.active {
                    Color::Magenta
                } else {
                    self.theme.inactive
                };
                let title = if self.loading {
                    " File Search (searching...) ".to_string()
                } else {
                    " File Search (Enter to search) ".to_string()
                };
                (color, title)
            }
        };

        let block = if title.is_empty() {
            Block::bordered().border_style(Style::default().fg(border_color))
        } else {
            Block::bordered()
                .border_style(Style::default().fg(border_color))
                .title(title)
        };

        let search = Paragraph::new(self.text.clone()).block(block);
        frame.render_widget(search, area);
        Ok(())
    }

    fn set_active(&mut self, active: bool) {
        self.active = active;
    }
}
