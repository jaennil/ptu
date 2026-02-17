use std::cell::RefCell;
use std::collections::HashMap;
use std::time::Instant;

use tokio::runtime::Runtime;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

use crate::action::Action;
use crate::aur;
use crate::components::installed_table::InstalledTable;
use crate::components::package_info::PackageInfo;
use crate::components::packages_table::PackagesTable;
use crate::components::{package_input::PackageInput, Component};
use crate::config::{self, GlobalKeys, HelpKeys, Keymap, TabsKeys};
use crate::layout::{FILTER_HEIGHT, INPUT_HEIGHT, LEFT_PANEL_PERCENT};
use crate::pacman::{self, Package, Pacman};
use crate::tui::Tui;

use color_eyre::eyre;
use ratatui::crossterm;
use ratatui::crossterm::event::{Event, KeyEvent, MouseEvent, MouseEventKind};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};

const TAB_BAR_HEIGHT: u16 = 1;
const STATUS_BAR_HEIGHT: u16 = 1;
const STATUS_DISPLAY_SECS: u64 = 5;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AppTab {
    Search,
    Installed,
}

pub(crate) struct App {
    tui: Tui,
    package_input: PackageInput,
    components: Vec<Box<dyn Component>>,
    installed_table: InstalledTable,
    active_tab: AppTab,
    pacman: Pacman,
    should_exit: bool,
    runtime: Runtime,
    aur_sender: UnboundedSender<Vec<Package>>,
    aur_receiver: UnboundedReceiver<Vec<Package>>,
    file_sender: UnboundedSender<Vec<Package>>,
    file_receiver: UnboundedReceiver<Vec<Package>>,
    aur_update_sender: UnboundedSender<HashMap<String, String>>,
    aur_update_receiver: UnboundedReceiver<HashMap<String, String>>,
    focused: usize,
    packages_table_area: Rect,
    installed_table_area: Rect,
    package_info_area: Rect,
    show_help: bool,
    help_scroll: u16,
    global_keys: GlobalKeys,
    tabs_keys: TabsKeys,
    help_keys: HelpKeys,
    status_message: Option<String>,
    status_time: Option<Instant>,
    status_is_error: bool,
    pending_action: Option<Action>,
    confirmation_text: String,
    show_upgrade_menu: bool,
    system_upgrade_keys: Vec<KeyEvent>,
}

impl App {
    pub(crate) fn new(keymap: Keymap) -> eyre::Result<Self> {
        let tui = Tui::new()?;
        let should_exit = Default::default();
        let pacman = Pacman::new()?;

        // Create tokio runtime for async tasks
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(1)
            .enable_all()
            .build()?;
        let (aur_sender, aur_receiver) = tokio::sync::mpsc::unbounded_channel();
        let (file_sender, file_receiver) = tokio::sync::mpsc::unbounded_channel();
        let (aur_update_sender, aur_update_receiver) = tokio::sync::mpsc::unbounded_channel();

        tracing::debug!("tokio runtime created for async searches");

        let mut package_input = PackageInput::new(keymap.search_input);
        package_input.set_active(true); // Start with focus on input

        let mut packages_table = PackagesTable::new(keymap.packages_table);
        packages_table.set_active(false);

        let mut package_info = PackageInfo::new(keymap.package_info);
        package_info.set_active(false);

        // Check for repo updates and load installed packages
        let repo_updates = pacman.check_repo_updates();
        let installed_packages = pacman.get_installed_packages(&repo_updates);
        tracing::info!(count = installed_packages.len(), "loaded installed packages for Installed tab");

        // Spawn async AUR update check
        let aur_pkgs: Vec<(String, String)> = installed_packages
            .iter()
            .filter(|p| p.source == "aur")
            .map(|p| (p.name.clone(), p.version.clone()))
            .collect();
        if !aur_pkgs.is_empty() {
            tracing::info!(count = aur_pkgs.len(), "starting async AUR update check");
            let sender = aur_update_sender.clone();
            runtime.spawn(async move {
                match aur::check_updates(aur_pkgs).await {
                    Ok(updates) => {
                        tracing::debug!(count = updates.len(), "AUR update check completed");
                        if let Err(e) = sender.send(updates) {
                            tracing::warn!(%e, "failed to send AUR update results");
                        }
                    }
                    Err(e) => {
                        tracing::warn!(%e, "AUR update check failed");
                    }
                }
            });
        }

        let system_upgrade_keys = keymap.installed_table.system_upgrade.clone();
        let installed_table = InstalledTable::new(installed_packages, keymap.installed_table);

        Ok(Self {
            tui,
            package_input,
            components: vec![
                Box::new(packages_table),
                Box::new(package_info),
            ],
            installed_table,
            active_tab: AppTab::Search,
            pacman,
            should_exit,
            runtime,
            aur_sender,
            aur_receiver,
            file_sender,
            file_receiver,
            aur_update_sender,
            aur_update_receiver,
            focused: 0,
            packages_table_area: Rect::default(),
            installed_table_area: Rect::default(),
            package_info_area: Rect::default(),
            show_help: false,
            help_scroll: 0,
            global_keys: keymap.global,
            tabs_keys: keymap.tabs,
            help_keys: keymap.help,
            status_message: None,
            status_time: None,
            status_is_error: false,
            pending_action: None,
            confirmation_text: String::new(),
            show_upgrade_menu: false,
            system_upgrade_keys,
        })
    }

    /// Number of focusable components for the current tab
    fn num_focusable(&self) -> usize {
        match self.active_tab {
            AppTab::Search => 3,    // input(0), packages_table(1), package_info(2)
            AppTab::Installed => 2, // installed_table(0), package_info(1)
        }
    }

    fn deactivate_focused(&mut self) {
        match self.active_tab {
            AppTab::Search => match self.focused {
                0 => self.package_input.set_active(false),
                1 => self.components[0].set_active(false), // packages_table
                2 => self.components[1].set_active(false), // package_info
                _ => {}
            },
            AppTab::Installed => match self.focused {
                0 => self.installed_table.set_active(false),
                1 => self.components[1].set_active(false), // package_info
                _ => {}
            },
        }
    }

    fn activate_focused(&mut self) {
        match self.active_tab {
            AppTab::Search => match self.focused {
                0 => self.package_input.set_active(true),
                1 => self.components[0].set_active(true),
                2 => self.components[1].set_active(true),
                _ => {}
            },
            AppTab::Installed => match self.focused {
                0 => self.installed_table.set_active(true),
                1 => self.components[1].set_active(true),
                _ => {}
            },
        }
    }

    fn set_focus(&mut self, new_focus: usize) {
        let num = self.num_focusable();
        tracing::debug!(current = self.focused, new = new_focus, num, "set_focus called");

        if new_focus == self.focused || new_focus >= num {
            tracing::debug!("set_focus: no change needed");
            return;
        }

        self.deactivate_focused();
        self.focused = new_focus;
        self.activate_focused();

        tracing::info!(focused = self.focused, tab = ?self.active_tab, "focus changed");
    }

    fn cycle_focus(&mut self) {
        let num = self.num_focusable();
        let new_focus = (self.focused + 1) % num;
        self.set_focus(new_focus);
    }

    fn switch_tab(&mut self, tab: AppTab) {
        if self.active_tab == tab {
            return;
        }
        tracing::info!(from = ?self.active_tab, to = ?tab, "switching tab");

        // Deactivate current focus
        self.deactivate_focused();

        self.active_tab = tab;
        self.focused = 0;

        // Activate first component of new tab
        self.activate_focused();

        // When switching to installed tab, auto-select first package for info panel
        if tab == AppTab::Installed {
            if let Some(package) = self.installed_table.get_selected_package() {
                let event = crate::event::Event::PackageSelected(Box::new(package.clone()));
                for component in self.components.iter_mut() {
                    let _ = component.update(&event);
                }
            }
        }
    }

    pub(crate) fn run(&mut self) -> eyre::Result<()> {
        tracing::debug!("entering TUI mode");
        Tui::enter()?;

        let mut needs_render = true;

        while !self.should_exit {
            if needs_render {
                self.render()?;
                needs_render = false;
            }

            let (actions, event_needs_render) = self.handle_events()?;
            needs_render |= event_needs_render;

            if !actions.is_empty() {
                self.handle_actions(&actions)?;
                needs_render = true;
            }

            // Check if AUR debounce timer elapsed
            if let Some(query) = self.package_input.should_search_aur() {
                self.start_aur_search(&query);
                let event = crate::event::Event::AurSearchStarted;
                for component in self.components.iter_mut() {
                    component.update(&event)?;
                }
                needs_render = true;
            }

            // Check for AUR results from async task
            if let Ok(aur_packages) = self.aur_receiver.try_recv() {
                tracing::debug!(count = aur_packages.len(), "received AUR packages from async task");
                let event = crate::event::Event::AurPackagesFound(aur_packages);
                for component in self.components.iter_mut() {
                    component.update(&event)?;
                }
                needs_render = true;
            }

            // Auto-clear expired status messages
            if let Some(time) = self.status_time {
                if time.elapsed() >= std::time::Duration::from_secs(STATUS_DISPLAY_SECS) {
                    self.status_message = None;
                    self.status_time = None;
                    needs_render = true;
                }
            }

            // Check for AUR update results from async task
            if let Ok(aur_updates) = self.aur_update_receiver.try_recv() {
                tracing::debug!(count = aur_updates.len(), "received AUR update results");
                let event = crate::event::Event::AurUpdatesChecked(aur_updates);
                for component in self.components.iter_mut() {
                    component.update(&event)?;
                }
                self.installed_table.update(&event)?;
                needs_render = true;
            }

            // Check for file search results from async task
            if let Ok(file_packages) = self.file_receiver.try_recv() {
                self.package_input.set_loading(false);
                tracing::debug!(count = file_packages.len(), "received file search results");
                if let Some(first) = file_packages.first() {
                    let select_event = crate::event::Event::PackageSelected(Box::new(first.clone()));
                    for component in self.components.iter_mut() {
                        component.update(&select_event)?;
                    }
                }
                let event = crate::event::Event::FoundPackages(file_packages);
                for component in self.components.iter_mut() {
                    component.update(&event)?;
                }
                needs_render = true;
            }
        }

        tracing::debug!("exiting TUI mode");
        Tui::exit()?;

        Ok(())
    }

    fn render(&mut self) -> eyre::Result<()> {
        let render_error: RefCell<Option<eyre::Report>> = RefCell::new(None);
        let packages_table_area: RefCell<Rect> = RefCell::new(Rect::default());
        let installed_table_area: RefCell<Rect> = RefCell::new(Rect::default());
        let package_info_area: RefCell<Rect> = RefCell::new(Rect::default());
        let show_help = self.show_help;
        let help_scroll = self.help_scroll;
        let show_confirmation = self.pending_action.is_some();
        let confirmation_text = self.confirmation_text.clone();
        let show_upgrade_menu = self.show_upgrade_menu;
        let active_tab = self.active_tab;
        let status_message = self.status_message.clone();
        let status_is_error = self.status_is_error;

        self.tui.draw(|frame| {
            let area = frame.area();

            let has_status = status_message.is_some();

            // Split: tab bar (1 line) + content + optional status bar
            let main_layout = if has_status {
                Layout::vertical([
                    Constraint::Length(TAB_BAR_HEIGHT),
                    Constraint::Percentage(100),
                    Constraint::Length(STATUS_BAR_HEIGHT),
                ])
                .split(area)
            } else {
                Layout::vertical([
                    Constraint::Length(TAB_BAR_HEIGHT),
                    Constraint::Percentage(100),
                    Constraint::Length(0),
                ])
                .split(area)
            };

            let tab_bar_area = main_layout[0];
            let content_area = main_layout[1];
            let status_area = main_layout[2];

            // Draw tab bar
            Self::draw_tab_bar(frame, tab_bar_area, active_tab);

            // Compute layout areas based on content_area
            let horizontal = Layout::horizontal([
                Constraint::Percentage(LEFT_PANEL_PERCENT),
                Constraint::Percentage(100 - LEFT_PANEL_PERCENT),
            ])
            .split(content_area);

            *package_info_area.borrow_mut() = horizontal[1];

            match active_tab {
                AppTab::Search => {
                    let left_vertical = Layout::vertical([
                        Constraint::Length(INPUT_HEIGHT),
                        Constraint::Length(FILTER_HEIGHT),
                        Constraint::Percentage(100),
                    ])
                    .split(horizontal[0]);

                    let filter_and_table = Rect {
                        x: left_vertical[1].x,
                        y: left_vertical[1].y,
                        width: left_vertical[1].width,
                        height: left_vertical[1].height + left_vertical[2].height,
                    };
                    *packages_table_area.borrow_mut() = filter_and_table;

                    // Draw search tab components
                    if let Err(e) = self.package_input.draw(frame, &content_area) {
                        *render_error.borrow_mut() = Some(e);
                        return;
                    }
                    for component in self.components.iter_mut() {
                        if let Err(e) = component.draw(frame, &content_area) {
                            *render_error.borrow_mut() = Some(e);
                            return;
                        }
                    }
                }
                AppTab::Installed => {
                    let left_vertical = Layout::vertical([
                        Constraint::Length(FILTER_HEIGHT), // sort bar
                        Constraint::Percentage(100),       // table
                    ])
                    .split(horizontal[0]);

                    let sort_and_table = Rect {
                        x: left_vertical[0].x,
                        y: left_vertical[0].y,
                        width: left_vertical[0].width,
                        height: left_vertical[0].height + left_vertical[1].height,
                    };
                    *installed_table_area.borrow_mut() = sort_and_table;

                    // Draw installed table with explicit areas
                    self.installed_table.draw_in_area(frame, left_vertical[0], left_vertical[1]);

                    // Draw package info (right panel) - reuse component
                    if let Err(e) = self.components[1].draw(frame, &content_area) {
                        *render_error.borrow_mut() = Some(e);
                        return;
                    }
                }
            }

            // Draw status bar
            if let Some(ref msg) = status_message {
                let color = if status_is_error { Color::Red } else { Color::Green };
                let line = Line::from(Span::styled(
                    format!(" {} ", msg),
                    Style::default().fg(color).add_modifier(Modifier::BOLD),
                ));
                frame.render_widget(line, status_area);
            }

            // Draw help overlay on top of everything
            if show_help {
                Self::draw_help(frame, area, help_scroll);
            }

            // Draw confirmation dialog on top of everything
            if show_confirmation {
                Self::draw_confirmation(frame, area, &confirmation_text);
            }

            // Draw upgrade menu on top of everything
            if show_upgrade_menu {
                Self::draw_upgrade_menu(frame, area);
            }
        })?;

        self.packages_table_area = packages_table_area.into_inner();
        self.installed_table_area = installed_table_area.into_inner();
        self.package_info_area = package_info_area.into_inner();

        if let Some(e) = render_error.into_inner() {
            return Err(e);
        }

        Ok(())
    }

    fn handle_events(&mut self) -> eyre::Result<(Vec<Action>, bool)> {
        let mut actions = Vec::new();
        let mut needs_render = false;

        // Poll with 16ms timeout for ~60fps responsiveness
        if crossterm::event::poll(std::time::Duration::from_millis(16))? {
            match crossterm::event::read()? {
                Event::Key(key_event) => {
                    let component_actions = self.handle_key_event(&key_event)?;
                    actions.extend(component_actions);
                    needs_render = true;
                }
                Event::Mouse(mouse_event) => {
                    // Only process scroll events, ignore MouseMove etc.
                    if matches!(
                        mouse_event.kind,
                        MouseEventKind::ScrollUp | MouseEventKind::ScrollDown
                    ) {
                        let component_actions = self.handle_mouse_event(&mouse_event)?;
                        actions.extend(component_actions);
                        needs_render = true;
                    }
                }
                _ => {}
            }
        }

        Ok((actions, needs_render))
    }

    fn handle_key_event(&mut self, key_event: &KeyEvent) -> eyre::Result<Vec<Action>> {
        // When help is visible, intercept all keys
        if self.show_help {
            if config::key_matches(key_event, &self.help_keys.close) {
                tracing::debug!("closing help window");
                self.show_help = false;
                self.help_scroll = 0;
            } else if config::key_matches(key_event, &self.help_keys.scroll_down) {
                self.help_scroll = self.help_scroll.saturating_add(1);
                tracing::trace!(scroll = self.help_scroll, "help scroll down");
            } else if config::key_matches(key_event, &self.help_keys.scroll_up) {
                self.help_scroll = self.help_scroll.saturating_sub(1);
                tracing::trace!(scroll = self.help_scroll, "help scroll up");
            } else {
                tracing::trace!(key = ?key_event.code, "key swallowed by help window");
            }
            return Ok(Vec::new());
        }

        // When confirmation dialog is visible, intercept all keys
        if self.pending_action.is_some() {
            match key_event.code {
                crossterm::event::KeyCode::Char('y') => {
                    let action = self.pending_action.take().unwrap();
                    self.confirmation_text.clear();
                    tracing::info!("confirmation accepted, executing action");
                    let events = self.handle_action_confirmed(&action)?;
                    for component in self.components.iter_mut() {
                        for event in &events {
                            component.update(event)?;
                        }
                    }
                    for event in &events {
                        self.installed_table.update(event)?;
                    }
                    for event in &events {
                        match event {
                            crate::event::Event::PackageInstalled(name) => {
                                self.set_status(format!("Installed: {}", name), false);
                            }
                            crate::event::Event::PackageRemoved(name) => {
                                self.set_status(format!("Removed: {}", name), false);
                            }
                            crate::event::Event::PackagesInstalled(names) => {
                                self.set_status(format!("Installed {} packages", names.len()), false);
                            }
                            crate::event::Event::PackagesRemoved(names) => {
                                self.set_status(format!("Removed {} packages", names.len()), false);
                            }
                            crate::event::Event::OperationFailed { package, error } => {
                                self.set_status(format!("Failed: {} ({})", package, error), true);
                            }
                            crate::event::Event::SystemUpgraded => {
                                self.set_status("System upgraded".to_string(), false);
                                self.refresh_installed();
                            }
                            _ => {}
                        }
                    }
                }
                crossterm::event::KeyCode::Char('n') | crossterm::event::KeyCode::Esc => {
                    tracing::info!("confirmation cancelled");
                    self.pending_action = None;
                    self.confirmation_text.clear();
                    self.set_status("Cancelled".to_string(), false);
                }
                _ => {
                    tracing::trace!(key = ?key_event.code, "key swallowed by confirmation dialog");
                }
            }
            return Ok(Vec::new());
        }

        // When upgrade menu is visible, intercept all keys
        if self.show_upgrade_menu {
            match key_event.code {
                crossterm::event::KeyCode::Char('a') => {
                    tracing::info!("upgrade menu: selected all packages");
                    self.show_upgrade_menu = false;
                    let action = Action::SystemUpgrade;
                    let events = self.handle_action(&action)?;
                    for component in self.components.iter_mut() {
                        for event in &events {
                            component.update(event)?;
                        }
                    }
                    for event in &events {
                        self.installed_table.update(event)?;
                    }
                }
                crossterm::event::KeyCode::Char('r') => {
                    tracing::info!("upgrade menu: selected repo packages");
                    self.show_upgrade_menu = false;
                    let action = Action::RepoUpgrade;
                    let events = self.handle_action(&action)?;
                    for component in self.components.iter_mut() {
                        for event in &events {
                            component.update(event)?;
                        }
                    }
                    for event in &events {
                        self.installed_table.update(event)?;
                    }
                }
                crossterm::event::KeyCode::Char('u') => {
                    tracing::info!("upgrade menu: selected AUR packages");
                    self.show_upgrade_menu = false;
                    let action = Action::AurUpgrade;
                    let events = self.handle_action(&action)?;
                    for component in self.components.iter_mut() {
                        for event in &events {
                            component.update(event)?;
                        }
                    }
                    for event in &events {
                        self.installed_table.update(event)?;
                    }
                }
                crossterm::event::KeyCode::Esc => {
                    tracing::info!("upgrade menu: cancelled");
                    self.show_upgrade_menu = false;
                    self.set_status("Cancelled".to_string(), false);
                }
                _ => {
                    tracing::trace!(key = ?key_event.code, "key swallowed by upgrade menu");
                }
            }
            return Ok(Vec::new());
        }

        // Skip global keys when a text filter mode is active
        let in_filter_mode = (self.active_tab == AppTab::Installed
            && self.focused == 0
            && self.installed_table.is_filter_mode())
            || (self.active_tab == AppTab::Search
                && self.focused == 1
                && self.components[0].is_name_filter_mode());

        // Open help
        if !in_filter_mode && config::key_matches(key_event, &self.global_keys.toggle_help) {
            tracing::debug!("opening help window");
            self.show_help = true;
            self.help_scroll = 0;
            return Ok(Vec::new());
        }

        // Quit
        if !in_filter_mode && config::key_matches(key_event, &self.global_keys.quit) {
            self.should_exit = true;
        }

        tracing::trace!(code = ?key_event.code, modifiers = ?key_event.modifiers, "key event received");

        // When filter mode is active, route all keys directly to the filtered component
        if in_filter_mode {
            if self.active_tab == AppTab::Installed && self.focused == 0 {
                if let Some(component_actions) = self.installed_table.handle_key_event(key_event)? {
                    return Ok(component_actions);
                }
            } else if self.active_tab == AppTab::Search && self.focused == 1 {
                if let Some(component_actions) = self.components[0].handle_key_event(key_event)? {
                    return Ok(component_actions);
                }
            }
            return Ok(Vec::new());
        }

        // Handle tab switching
        if config::key_matches(key_event, &self.tabs_keys.search) {
            self.switch_tab(AppTab::Search);
            return Ok(Vec::new());
        }
        if config::key_matches(key_event, &self.tabs_keys.installed) {
            self.switch_tab(AppTab::Installed);
            return Ok(Vec::new());
        }

        // Handle focus switching (tab-aware)
        let info_focus = self.num_focusable() - 1; // info panel is always last

        if config::key_matches(key_event, &self.global_keys.cycle_focus) {
            self.cycle_focus();
            return Ok(Vec::new());
        }
        if config::key_matches(key_event, &self.global_keys.focus_down) {
            match self.active_tab {
                AppTab::Search => {
                    if self.focused < 1 {
                        self.set_focus(1);
                    }
                }
                AppTab::Installed => {
                    if self.focused != 0 {
                        self.set_focus(0);
                    }
                }
            }
            return Ok(Vec::new());
        }
        if config::key_matches(key_event, &self.global_keys.focus_up) {
            if self.focused > 0 {
                self.set_focus(0);
            }
            return Ok(Vec::new());
        }
        if config::key_matches(key_event, &self.global_keys.focus_right) {
            if self.focused != info_focus {
                self.set_focus(info_focus);
            }
            return Ok(Vec::new());
        }
        if config::key_matches(key_event, &self.global_keys.focus_left) {
            if self.focused == info_focus {
                match self.active_tab {
                    AppTab::Search => self.set_focus(1),
                    AppTab::Installed => self.set_focus(0),
                }
            }
            return Ok(Vec::new());
        }

        // Handle system upgrade key on Installed tab
        if self.active_tab == AppTab::Installed
            && config::key_matches(key_event, &self.system_upgrade_keys)
        {
            tracing::debug!("opening upgrade menu");
            self.show_upgrade_menu = true;
            return Ok(Vec::new());
        }

        let mut actions = Vec::new();

        // Route key events to the focused component (tab-aware)
        match self.active_tab {
            AppTab::Search => match self.focused {
                0 => {
                    if let Some(component_actions) = self.package_input.handle_key_event(key_event)? {
                        actions.extend(component_actions);
                    }
                }
                1 => {
                    if let Some(component_actions) = self.components[0].handle_key_event(key_event)? {
                        actions.extend(component_actions);
                    }
                }
                2 => {
                    if let Some(component_actions) = self.components[1].handle_key_event(key_event)? {
                        actions.extend(component_actions);
                    }
                }
                _ => {}
            },
            AppTab::Installed => match self.focused {
                0 => {
                    if let Some(component_actions) = self.installed_table.handle_key_event(key_event)? {
                        actions.extend(component_actions);
                    }
                }
                1 => {
                    if let Some(component_actions) = self.components[1].handle_key_event(key_event)? {
                        actions.extend(component_actions);
                    }
                }
                _ => {}
            },
        }

        Ok(actions)
    }

    fn handle_mouse_event(&mut self, mouse_event: &MouseEvent) -> eyre::Result<Vec<Action>> {
        let mut actions = Vec::new();

        // Only handle scroll events
        match mouse_event.kind {
            MouseEventKind::ScrollUp | MouseEventKind::ScrollDown => {
                let (x, y) = (mouse_event.column, mouse_event.row);

                match self.active_tab {
                    AppTab::Search => {
                        if self.packages_table_area.contains((x, y).into()) {
                            tracing::trace!(x, y, "mouse scroll on packages table");
                            if let Some(component_actions) =
                                self.components[0].handle_mouse_event(mouse_event, &self.packages_table_area)?
                            {
                                actions.extend(component_actions);
                            }
                        } else if self.package_info_area.contains((x, y).into()) {
                            tracing::trace!(x, y, "mouse scroll on package info");
                            if let Some(component_actions) =
                                self.components[1].handle_mouse_event(mouse_event, &self.package_info_area)?
                            {
                                actions.extend(component_actions);
                            }
                        }
                    }
                    AppTab::Installed => {
                        if self.installed_table_area.contains((x, y).into()) {
                            tracing::trace!(x, y, "mouse scroll on installed table");
                            if let Some(component_actions) =
                                self.installed_table.handle_mouse_event(mouse_event, &self.installed_table_area)?
                            {
                                actions.extend(component_actions);
                            }
                        } else if self.package_info_area.contains((x, y).into()) {
                            tracing::trace!(x, y, "mouse scroll on package info");
                            if let Some(component_actions) =
                                self.components[1].handle_mouse_event(mouse_event, &self.package_info_area)?
                            {
                                actions.extend(component_actions);
                            }
                        }
                    }
                }
            }
            _ => {}
        }

        Ok(actions)
    }

    fn start_aur_search(&self, query: &str) {
        let sender = self.aur_sender.clone();
        let installed = self.pacman.installed_packages().clone();
        let query = query.to_string();

        // Spawn async task on tokio runtime
        self.runtime.spawn(async move {
            tracing::debug!(query = %query, "starting async AUR search");
            match aur::search(&query, &installed).await {
                Ok(aur_packages) => {
                    tracing::debug!(count = aur_packages.len(), query = %query, "AUR search completed");
                    if let Err(e) = sender.send(aur_packages) {
                        tracing::warn!(%e, "failed to send AUR results");
                    }
                }
                Err(e) => {
                    tracing::warn!(%e, "AUR search failed");
                }
            }
        });
    }

    fn draw_tab_bar(frame: &mut ratatui::Frame, area: Rect, active_tab: AppTab) {
        let active_style = Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD);
        let inactive_style = Style::default().fg(Color::DarkGray);

        let search_style = if active_tab == AppTab::Search { active_style } else { inactive_style };
        let installed_style = if active_tab == AppTab::Installed { active_style } else { inactive_style };

        let line = Line::from(vec![
            Span::styled(" F1:Search ", search_style),
            Span::styled(" F2:Installed ", installed_style),
        ]);

        frame.render_widget(line, area);
    }

    fn draw_help(frame: &mut ratatui::Frame, area: Rect, scroll: u16) {
        // Centered area: ~70% width, ~80% height
        let help_width = (area.width as u32 * 70 / 100) as u16;
        let help_height = (area.height as u32 * 80 / 100) as u16;
        let help_x = area.x + (area.width.saturating_sub(help_width)) / 2;
        let help_y = area.y + (area.height.saturating_sub(help_height)) / 2;
        let help_area = Rect::new(help_x, help_y, help_width, help_height);

        let header_style = Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD);
        let key_style = Style::default().fg(Color::Cyan);
        let desc_style = Style::default().fg(Color::White);

        let lines = vec![
            Line::from(Span::styled(" Global", header_style)),
            Line::from(vec![
                Span::styled("   ?             ", key_style),
                Span::styled("Toggle help", desc_style),
            ]),
            Line::from(vec![
                Span::styled("   Esc           ", key_style),
                Span::styled("Quit / Close help", desc_style),
            ]),
            Line::from(vec![
                Span::styled("   Tab           ", key_style),
                Span::styled("Cycle focus", desc_style),
            ]),
            Line::from(vec![
                Span::styled("   Alt+j/k/h/l   ", key_style),
                Span::styled("Navigate focus", desc_style),
            ]),
            Line::from(vec![
                Span::styled("   y / n         ", key_style),
                Span::styled("Confirm / cancel action", desc_style),
            ]),
            Line::from(""),
            Line::from(Span::styled(" Tabs", header_style)),
            Line::from(vec![
                Span::styled("   F1 / Alt+1    ", key_style),
                Span::styled("Search tab", desc_style),
            ]),
            Line::from(vec![
                Span::styled("   F2 / Alt+2    ", key_style),
                Span::styled("Installed tab", desc_style),
            ]),
            Line::from(""),
            Line::from(Span::styled(" Search Input", header_style)),
            Line::from(vec![
                Span::styled("   <type>        ", key_style),
                Span::styled("Search packages", desc_style),
            ]),
            Line::from(vec![
                Span::styled("   Backspace     ", key_style),
                Span::styled("Delete character", desc_style),
            ]),
            Line::from(vec![
                Span::styled("   Ctrl+w        ", key_style),
                Span::styled("Delete word", desc_style),
            ]),
            Line::from(vec![
                Span::styled("   Ctrl+f        ", key_style),
                Span::styled("Toggle file/package search mode", desc_style),
            ]),
            Line::from(vec![
                Span::styled("   Enter         ", key_style),
                Span::styled("Search files (in file mode)", desc_style),
            ]),
            Line::from(""),
            Line::from(Span::styled(" Packages Table", header_style)),
            Line::from(vec![
                Span::styled("   j / k         ", key_style),
                Span::styled("Navigate down / up", desc_style),
            ]),
            Line::from(vec![
                Span::styled("   g / G         ", key_style),
                Span::styled("First / last", desc_style),
            ]),
            Line::from(vec![
                Span::styled("   i             ", key_style),
                Span::styled("Install package", desc_style),
            ]),
            Line::from(vec![
                Span::styled("   r             ", key_style),
                Span::styled("Remove package", desc_style),
            ]),
            Line::from(vec![
                Span::styled("   I             ", key_style),
                Span::styled("Update+install / batch install", desc_style),
            ]),
            Line::from(vec![
                Span::styled("   R             ", key_style),
                Span::styled("Batch remove", desc_style),
            ]),
            Line::from(vec![
                Span::styled("   Space         ", key_style),
                Span::styled("Toggle multi-select", desc_style),
            ]),
            Line::from(vec![
                Span::styled("   f             ", key_style),
                Span::styled("Enter filter mode", desc_style),
            ]),
            Line::from(vec![
                Span::styled("   /             ", key_style),
                Span::styled("Filter by name", desc_style),
            ]),
            Line::from(vec![
                Span::styled("   Enter         ", key_style),
                Span::styled("Apply name filter", desc_style),
            ]),
            Line::from(vec![
                Span::styled("   Esc           ", key_style),
                Span::styled("Clear name filter", desc_style),
            ]),
            Line::from(""),
            Line::from(Span::styled(" Filter Mode (f+key)", header_style)),
            Line::from(vec![
                Span::styled("   i             ", key_style),
                Span::styled("Cycle install filter", desc_style),
            ]),
            Line::from(vec![
                Span::styled("   a             ", key_style),
                Span::styled("Toggle AUR only", desc_style),
            ]),
            Line::from(vec![
                Span::styled("   p             ", key_style),
                Span::styled("Toggle Pacman only", desc_style),
            ]),
            Line::from(vec![
                Span::styled("   c             ", key_style),
                Span::styled("Clear all filters", desc_style),
            ]),
            Line::from(vec![
                Span::styled("   Esc           ", key_style),
                Span::styled("Exit filter mode", desc_style),
            ]),
            Line::from(""),
            Line::from(Span::styled(" Installed Table", header_style)),
            Line::from(vec![
                Span::styled("   j / k         ", key_style),
                Span::styled("Navigate down / up", desc_style),
            ]),
            Line::from(vec![
                Span::styled("   g / G         ", key_style),
                Span::styled("First / last", desc_style),
            ]),
            Line::from(vec![
                Span::styled("   s             ", key_style),
                Span::styled("Cycle sort column", desc_style),
            ]),
            Line::from(vec![
                Span::styled("   S             ", key_style),
                Span::styled("Toggle sort direction", desc_style),
            ]),
            Line::from(vec![
                Span::styled("   r             ", key_style),
                Span::styled("Remove package", desc_style),
            ]),
            Line::from(vec![
                Span::styled("   R             ", key_style),
                Span::styled("Batch remove", desc_style),
            ]),
            Line::from(vec![
                Span::styled("   Space         ", key_style),
                Span::styled("Toggle multi-select", desc_style),
            ]),
            Line::from(vec![
                Span::styled("   /             ", key_style),
                Span::styled("Filter by name", desc_style),
            ]),
            Line::from(vec![
                Span::styled("   Enter         ", key_style),
                Span::styled("Apply filter", desc_style),
            ]),
            Line::from(vec![
                Span::styled("   Esc           ", key_style),
                Span::styled("Clear filter", desc_style),
            ]),
            Line::from(vec![
                Span::styled("   u             ", key_style),
                Span::styled("Toggle update filter", desc_style),
            ]),
            Line::from(vec![
                Span::styled("   Ctrl+r        ", key_style),
                Span::styled("Refresh packages and updates", desc_style),
            ]),
            Line::from(vec![
                Span::styled("   U             ", key_style),
                Span::styled("System upgrade menu", desc_style),
            ]),
            Line::from(""),
            Line::from(Span::styled(" Package Info", header_style)),
            Line::from(vec![
                Span::styled("   j / k         ", key_style),
                Span::styled("Scroll down / up", desc_style),
            ]),
            Line::from(vec![
                Span::styled("   Ctrl+d / u    ", key_style),
                Span::styled("Page down / up", desc_style),
            ]),
            Line::from(vec![
                Span::styled("   o             ", key_style),
                Span::styled("Open URL in browser", desc_style),
            ]),
        ];

        // Clear the area behind the popup
        frame.render_widget(Clear, help_area);

        let help_block = Block::default()
            .title(" Help (?) ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Yellow))
            .style(Style::default().bg(Color::Black));

        let paragraph = Paragraph::new(lines)
            .block(help_block)
            .wrap(Wrap { trim: false })
            .scroll((scroll, 0));

        frame.render_widget(paragraph, help_area);
    }

    fn handle_actions(&mut self, actions: &[Action]) -> eyre::Result<()> {
        let mut events = Vec::new();

        for action in actions {
            let app_events = self.handle_action(action)?;
            events.extend(app_events);
        }

        for component in self.components.iter_mut() {
            for event in &events {
                component.update(event)?;
            }
        }

        // Also propagate events to installed_table
        for event in &events {
            self.installed_table.update(event)?;
        }

        // Set status messages from events
        for event in &events {
            match event {
                crate::event::Event::PackageInstalled(name) => {
                    self.set_status(format!("Installed: {}", name), false);
                }
                crate::event::Event::PackageRemoved(name) => {
                    self.set_status(format!("Removed: {}", name), false);
                }
                crate::event::Event::PackagesInstalled(names) => {
                    self.set_status(format!("Installed {} packages", names.len()), false);
                }
                crate::event::Event::PackagesRemoved(names) => {
                    self.set_status(format!("Removed {} packages", names.len()), false);
                }
                crate::event::Event::OperationFailed { package, error } => {
                    self.set_status(format!("Failed: {} ({})", package, error), true);
                }
                _ => {}
            }
        }

        Ok(())
    }

    fn refresh_installed(&mut self) {
        tracing::info!("refreshing installed packages");
        match Pacman::new() {
            Ok(new_pacman) => {
                self.pacman = new_pacman;
                let repo_updates = self.pacman.check_repo_updates();
                let new_packages = self.pacman.get_installed_packages(&repo_updates);
                let count = new_packages.len();
                tracing::info!(count, "reloaded installed packages");

                // Spawn async AUR update check
                let aur_pkgs: Vec<(String, String)> = new_packages
                    .iter()
                    .filter(|p| p.source == "aur")
                    .map(|p| (p.name.clone(), p.version.clone()))
                    .collect();
                if !aur_pkgs.is_empty() {
                    tracing::info!(count = aur_pkgs.len(), "starting async AUR update check after refresh");
                    let sender = self.aur_update_sender.clone();
                    self.runtime.spawn(async move {
                        match aur::check_updates(aur_pkgs).await {
                            Ok(updates) => {
                                tracing::debug!(count = updates.len(), "AUR update check completed after refresh");
                                if let Err(e) = sender.send(updates) {
                                    tracing::warn!(%e, "failed to send AUR update results after refresh");
                                }
                            }
                            Err(e) => {
                                tracing::warn!(%e, "AUR update check failed after refresh");
                            }
                        }
                    });
                }

                self.installed_table.reload(new_packages.clone());

                // Select first package if available
                if let Some(first) = new_packages.first() {
                    let event = crate::event::Event::PackageSelected(Box::new(first.clone()));
                    for component in self.components.iter_mut() {
                        let _ = component.update(&event);
                    }
                }
            }
            Err(e) => {
                tracing::warn!(%e, "failed to refresh installed packages");
            }
        }
    }

    fn set_status(&mut self, message: String, is_error: bool) {
        tracing::debug!(message, is_error, "status message set");
        self.status_message = Some(message);
        self.status_time = Some(Instant::now());
        self.status_is_error = is_error;
    }

    fn handle_action(&mut self, action: &Action) -> eyre::Result<Vec<crate::event::Event>> {
        // Intercept destructive actions: show confirmation dialog instead of executing
        match action {
            Action::InstallPackage { name, .. } => {
                self.confirmation_text = format!("Install {}?", name);
                self.pending_action = Some(action.clone());
                tracing::debug!(text = %self.confirmation_text, "showing confirmation dialog");
                return Ok(Vec::new());
            }
            Action::UpdateInstallPackage { name, .. } => {
                self.confirmation_text = format!("Update {}?", name);
                self.pending_action = Some(action.clone());
                tracing::debug!(text = %self.confirmation_text, "showing confirmation dialog");
                return Ok(Vec::new());
            }
            Action::RemovePackage { name, .. } => {
                self.confirmation_text = format!("Remove {}?", name);
                self.pending_action = Some(action.clone());
                tracing::debug!(text = %self.confirmation_text, "showing confirmation dialog");
                return Ok(Vec::new());
            }
            Action::InstallPackages { packages } => {
                self.confirmation_text = format!("Install {} packages?", packages.len());
                self.pending_action = Some(action.clone());
                tracing::debug!(text = %self.confirmation_text, "showing confirmation dialog");
                return Ok(Vec::new());
            }
            Action::RemovePackages { packages } => {
                self.confirmation_text = format!("Remove {} packages?", packages.len());
                self.pending_action = Some(action.clone());
                tracing::debug!(text = %self.confirmation_text, "showing confirmation dialog");
                return Ok(Vec::new());
            }
            Action::SystemUpgrade => {
                self.confirmation_text = "Upgrade all packages?".to_string();
                self.pending_action = Some(action.clone());
                tracing::debug!(text = %self.confirmation_text, "showing confirmation dialog");
                return Ok(Vec::new());
            }
            Action::RepoUpgrade => {
                self.confirmation_text = "Upgrade repo packages?".to_string();
                self.pending_action = Some(action.clone());
                tracing::debug!(text = %self.confirmation_text, "showing confirmation dialog");
                return Ok(Vec::new());
            }
            Action::AurUpgrade => {
                self.confirmation_text = "Upgrade AUR packages?".to_string();
                self.pending_action = Some(action.clone());
                tracing::debug!(text = %self.confirmation_text, "showing confirmation dialog");
                return Ok(Vec::new());
            }
            _ => {}
        }

        // Non-destructive actions execute immediately
        let mut events = Vec::new();

        match action {
            Action::SearchPackage(query) => {
                tracing::debug!(query, "searching for package (pacman only, instant)");

                // Pacman search only (fast, local) - AUR is triggered separately after debounce
                let packages = self.pacman.search_package(query)?;
                tracing::debug!(count = packages.len(), "found pacman packages");

                // Auto-select first package if available
                if let Some(first) = packages.first() {
                    events.push(crate::event::Event::PackageSelected(Box::new(first.clone())));
                }
                events.push(crate::event::Event::FoundPackages(packages));
            }
            Action::SearchFile(query) => {
                tracing::info!(query, "starting async file search (pacman -F)");
                self.package_input.set_loading(true);
                let sender = self.file_sender.clone();
                let installed = self.pacman.installed_packages().clone();
                let query = query.clone();

                self.runtime.spawn(async move {
                    let result = tokio::task::spawn_blocking(move || {
                        pacman::search_file(&query, &installed)
                    })
                    .await;

                    match result {
                        Ok(Ok(packages)) => {
                            tracing::debug!(count = packages.len(), "file search completed");
                            if let Err(e) = sender.send(packages) {
                                tracing::warn!(%e, "failed to send file search results");
                            }
                        }
                        Ok(Err(e)) => {
                            tracing::warn!(%e, "file search failed");
                        }
                        Err(e) => {
                            tracing::warn!(%e, "file search task panicked");
                        }
                    }
                });
            }
            Action::SelectPackage(package) => {
                tracing::debug!(package_name = package.name, "package selected");
                events.push(crate::event::Event::PackageSelected(package.clone()));
            }
            Action::RefreshInstalled => {
                self.refresh_installed();
                self.set_status("Refreshed packages".to_string(), false);
            }
            Action::OpenUrl(url) => {
                tracing::info!(url, "opening URL in browser");
                match std::process::Command::new("xdg-open")
                    .arg(url)
                    .stdin(std::process::Stdio::null())
                    .stdout(std::process::Stdio::null())
                    .stderr(std::process::Stdio::null())
                    .spawn()
                {
                    Ok(_) => tracing::debug!(url, "xdg-open spawned successfully"),
                    Err(e) => tracing::warn!(url, %e, "failed to open URL with xdg-open"),
                }
            }
            // Destructive actions are intercepted above, unreachable here
            Action::InstallPackage { .. }
            | Action::UpdateInstallPackage { .. }
            | Action::RemovePackage { .. }
            | Action::InstallPackages { .. }
            | Action::RemovePackages { .. }
            | Action::SystemUpgrade
            | Action::RepoUpgrade
            | Action::AurUpgrade => unreachable!(),
        };

        Ok(events)
    }

    fn handle_action_confirmed(&mut self, action: &Action) -> eyre::Result<Vec<crate::event::Event>> {
        let mut events = Vec::new();

        match action {
            Action::InstallPackage { name, source } => {
                tracing::info!(name, source, "installing package");
                let mut success = false;
                self.tui.suspend(|| -> eyre::Result<()> {
                    let status = if source == "aur" {
                        aur::install(name)?
                    } else {
                        pacman::install_package(name)?
                    };
                    success = status.success();
                    Ok(())
                })?;
                if success {
                    tracing::info!(name, "package installed successfully");
                    self.pacman.mark_installed(name);
                    events.push(crate::event::Event::PackageInstalled(name.clone()));
                } else {
                    let error = "install failed".to_string();
                    tracing::warn!(name, %error, "package installation failed");
                    events.push(crate::event::Event::OperationFailed {
                        package: name.clone(),
                        error,
                    });
                }
            }
            Action::UpdateInstallPackage { name, source } => {
                tracing::info!(name, source, "updating and installing package");
                let mut success = false;
                self.tui.suspend(|| -> eyre::Result<()> {
                    let status = if source == "aur" {
                        aur::install(name)?
                    } else {
                        pacman::update_install_package(name)?
                    };
                    success = status.success();
                    Ok(())
                })?;
                if success {
                    tracing::info!(name, "package updated and installed successfully");
                    self.pacman.mark_installed(name);
                    events.push(crate::event::Event::PackageInstalled(name.clone()));
                } else {
                    let error = "update/install failed".to_string();
                    tracing::warn!(name, %error, "package update/install failed");
                    events.push(crate::event::Event::OperationFailed {
                        package: name.clone(),
                        error,
                    });
                }
            }
            Action::RemovePackage { name, source } => {
                tracing::info!(name, source, "removing package");
                let mut success = false;
                self.tui.suspend(|| -> eyre::Result<()> {
                    let status = if source == "aur" {
                        aur::remove(name)?
                    } else {
                        pacman::remove_package(name)?
                    };
                    success = status.success();
                    Ok(())
                })?;
                if success {
                    tracing::info!(name, "package removed successfully");
                    self.pacman.mark_removed(name);
                    events.push(crate::event::Event::PackageRemoved(name.clone()));
                } else {
                    let error = "remove failed".to_string();
                    tracing::warn!(name, %error, "package removal failed");
                    events.push(crate::event::Event::OperationFailed {
                        package: name.clone(),
                        error,
                    });
                }
            }
            Action::InstallPackages { packages } => {
                tracing::info!(count = packages.len(), "batch installing packages");

                // Separate by source
                let pacman_pkgs: Vec<&str> = packages
                    .iter()
                    .filter(|(_, src)| src != "aur")
                    .map(|(name, _)| name.as_str())
                    .collect();
                let aur_pkgs: Vec<&str> = packages
                    .iter()
                    .filter(|(_, src)| src == "aur")
                    .map(|(name, _)| name.as_str())
                    .collect();

                let mut installed_names = Vec::new();

                self.tui.suspend(|| -> eyre::Result<()> {
                    // Install pacman packages first
                    if !pacman_pkgs.is_empty() {
                        tracing::info!(count = pacman_pkgs.len(), "installing pacman packages");
                        let status = pacman::install_packages(&pacman_pkgs)?;
                        if status.success() {
                            installed_names.extend(pacman_pkgs.iter().map(|s| s.to_string()));
                        } else {
                            tracing::warn!("pacman batch install failed");
                        }
                    }

                    // Install AUR packages
                    if !aur_pkgs.is_empty() {
                        tracing::info!(count = aur_pkgs.len(), "installing AUR packages");
                        let status = aur::install_packages(&aur_pkgs)?;
                        if status.success() {
                            installed_names.extend(aur_pkgs.iter().map(|s| s.to_string()));
                        } else {
                            tracing::warn!("AUR batch install failed");
                        }
                    }

                    Ok(())
                })?;

                if !installed_names.is_empty() {
                    tracing::info!(count = installed_names.len(), "packages installed successfully");
                    for name in &installed_names {
                        self.pacman.mark_installed(name);
                    }
                    events.push(crate::event::Event::PackagesInstalled(installed_names));
                }
            }
            Action::RemovePackages { packages } => {
                tracing::info!(count = packages.len(), "batch removing packages");

                // Separate by source
                let pacman_pkgs: Vec<&str> = packages
                    .iter()
                    .filter(|(_, src)| src != "aur")
                    .map(|(name, _)| name.as_str())
                    .collect();
                let aur_pkgs: Vec<&str> = packages
                    .iter()
                    .filter(|(_, src)| src == "aur")
                    .map(|(name, _)| name.as_str())
                    .collect();

                let mut removed_names = Vec::new();

                self.tui.suspend(|| -> eyre::Result<()> {
                    // Remove pacman packages first
                    if !pacman_pkgs.is_empty() {
                        tracing::info!(count = pacman_pkgs.len(), "removing pacman packages");
                        let status = pacman::remove_packages(&pacman_pkgs)?;
                        if status.success() {
                            removed_names.extend(pacman_pkgs.iter().map(|s| s.to_string()));
                        } else {
                            tracing::warn!("pacman batch remove failed");
                        }
                    }

                    // Remove AUR packages
                    if !aur_pkgs.is_empty() {
                        tracing::info!(count = aur_pkgs.len(), "removing AUR packages");
                        let status = aur::remove_packages(&aur_pkgs)?;
                        if status.success() {
                            removed_names.extend(aur_pkgs.iter().map(|s| s.to_string()));
                        } else {
                            tracing::warn!("AUR batch remove failed");
                        }
                    }

                    Ok(())
                })?;

                if !removed_names.is_empty() {
                    tracing::info!(count = removed_names.len(), "packages removed successfully");
                    for name in &removed_names {
                        self.pacman.mark_removed(name);
                    }
                    events.push(crate::event::Event::PackagesRemoved(removed_names));
                }
            }
            Action::SystemUpgrade => {
                tracing::info!("executing system upgrade (all packages)");
                let mut success = false;
                self.tui.suspend(|| -> eyre::Result<()> {
                    let status = aur::system_upgrade()
                        .or_else(|e| {
                            tracing::warn!(%e, "AUR helper not available, falling back to pacman");
                            pacman::system_upgrade()
                        })?;
                    success = status.success();
                    Ok(())
                })?;
                if success {
                    tracing::info!("system upgrade completed successfully");
                    events.push(crate::event::Event::SystemUpgraded);
                } else {
                    tracing::warn!("system upgrade failed");
                    events.push(crate::event::Event::OperationFailed {
                        package: "system".to_string(),
                        error: "upgrade failed".to_string(),
                    });
                }
            }
            Action::RepoUpgrade => {
                tracing::info!("executing repo upgrade (pacman -Syu)");
                let mut success = false;
                self.tui.suspend(|| -> eyre::Result<()> {
                    let status = pacman::system_upgrade()?;
                    success = status.success();
                    Ok(())
                })?;
                if success {
                    tracing::info!("repo upgrade completed successfully");
                    events.push(crate::event::Event::SystemUpgraded);
                } else {
                    tracing::warn!("repo upgrade failed");
                    events.push(crate::event::Event::OperationFailed {
                        package: "system".to_string(),
                        error: "repo upgrade failed".to_string(),
                    });
                }
            }
            Action::AurUpgrade => {
                tracing::info!("executing AUR upgrade (yay -Sua)");
                let mut success = false;
                self.tui.suspend(|| -> eyre::Result<()> {
                    let status = aur::aur_upgrade()?;
                    success = status.success();
                    Ok(())
                })?;
                if success {
                    tracing::info!("AUR upgrade completed successfully");
                    events.push(crate::event::Event::SystemUpgraded);
                } else {
                    tracing::warn!("AUR upgrade failed");
                    events.push(crate::event::Event::OperationFailed {
                        package: "system".to_string(),
                        error: "AUR upgrade failed".to_string(),
                    });
                }
            }
            _ => {
                tracing::warn!("handle_action_confirmed called with non-destructive action");
            }
        };

        Ok(events)
    }

    fn draw_confirmation(frame: &mut ratatui::Frame, area: Rect, text: &str) {
        let popup_width = (area.width as u32 * 40 / 100).max(30) as u16;
        let popup_height = 5u16;
        let popup_x = area.x + (area.width.saturating_sub(popup_width)) / 2;
        let popup_y = area.y + (area.height.saturating_sub(popup_height)) / 2;
        let popup_area = Rect::new(popup_x, popup_y, popup_width, popup_height);

        frame.render_widget(Clear, popup_area);

        let block = Block::default()
            .title(" Confirm ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Yellow))
            .style(Style::default().bg(Color::Black));

        let lines = vec![
            Line::from(Span::styled(
                text,
                Style::default().fg(Color::White),
            )),
            Line::from(""),
            Line::from(vec![
                Span::styled("[y]", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
                Span::styled(" Yes  ", Style::default().fg(Color::White)),
                Span::styled("[n/Esc]", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
                Span::styled(" Cancel", Style::default().fg(Color::White)),
            ]),
        ];

        let paragraph = Paragraph::new(lines)
            .block(block)
            .wrap(Wrap { trim: false });

        frame.render_widget(paragraph, popup_area);
    }

    fn draw_upgrade_menu(frame: &mut ratatui::Frame, area: Rect) {
        let popup_width = (area.width as u32 * 40 / 100).max(34) as u16;
        let popup_height = 6u16;
        let popup_x = area.x + (area.width.saturating_sub(popup_width)) / 2;
        let popup_y = area.y + (area.height.saturating_sub(popup_height)) / 2;
        let popup_area = Rect::new(popup_x, popup_y, popup_width, popup_height);

        frame.render_widget(Clear, popup_area);

        let block = Block::default()
            .title(" Upgrade ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Yellow))
            .style(Style::default().bg(Color::Black));

        let key_style = Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD);
        let desc_style = Style::default().fg(Color::White);
        let dim_style = Style::default().fg(Color::DarkGray);

        let lines = vec![
            Line::from(vec![
                Span::styled("[a]", key_style),
                Span::styled(" All   ", desc_style),
                Span::styled("[r]", key_style),
                Span::styled(" Repo   ", desc_style),
                Span::styled("[u]", key_style),
                Span::styled(" AUR", desc_style),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("[Esc]", dim_style),
                Span::styled(" Cancel", dim_style),
            ]),
        ];

        let paragraph = Paragraph::new(lines)
            .block(block)
            .wrap(Wrap { trim: false });

        frame.render_widget(paragraph, popup_area);
    }
}
