use std::cell::RefCell;

use tokio::runtime::Runtime;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

use crate::action::Action;
use crate::aur;
use crate::components::package_info::PackageInfo;
use crate::components::packages_table::PackagesTable;
use crate::components::{package_input::PackageInput, Component};
use crate::layout::{FILTER_HEIGHT, INPUT_HEIGHT, LEFT_PANEL_PERCENT};
use crate::pacman::{self, Package, Pacman};
use crate::tui::Tui;

use color_eyre::eyre;
use ratatui::crossterm;
use ratatui::crossterm::event::{Event, KeyCode, KeyEvent, MouseEvent, MouseEventKind};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};

const NUM_COMPONENTS: usize = 3; // package_input, packages_table, package_info

pub(crate) struct App {
    tui: Tui,
    package_input: PackageInput,
    components: Vec<Box<dyn Component>>,
    pacman: Pacman,
    should_exit: bool,
    runtime: Runtime,
    aur_sender: UnboundedSender<Vec<Package>>,
    aur_receiver: UnboundedReceiver<Vec<Package>>,
    focused: usize, // 0 = package_input, 1 = packages_table, 2 = package_info
    packages_table_area: Rect,
    package_info_area: Rect,
    show_help: bool,
    help_scroll: u16,
}

impl App {
    pub(crate) fn new() -> eyre::Result<Self> {
        let tui = Tui::new()?;
        let should_exit = Default::default();
        let pacman = Pacman::new()?;

        // Create tokio runtime for async tasks
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(1)
            .enable_all()
            .build()?;
        let (aur_sender, aur_receiver) = tokio::sync::mpsc::unbounded_channel();

        tracing::debug!("tokio runtime created for async AUR searches");

        let mut package_input = PackageInput::default();
        package_input.set_active(true); // Start with focus on input

        let mut packages_table = PackagesTable::default();
        packages_table.set_active(false);

        let mut package_info = PackageInfo::default();
        package_info.set_active(false);

        Ok(Self {
            tui,
            package_input,
            components: vec![
                Box::new(packages_table),
                Box::new(package_info),
            ],
            pacman,
            should_exit,
            runtime,
            aur_sender,
            aur_receiver,
            focused: 0,
            packages_table_area: Rect::default(),
            package_info_area: Rect::default(),
            show_help: false,
            help_scroll: 0,
        })
    }

    fn set_focus(&mut self, new_focus: usize) {
        tracing::debug!(current = self.focused, new = new_focus, "set_focus called");

        if new_focus == self.focused || new_focus >= NUM_COMPONENTS {
            tracing::debug!("set_focus: no change needed");
            return;
        }

        // Deactivate current
        match self.focused {
            0 => self.package_input.set_active(false),
            n => self.components[n - 1].set_active(false),
        }

        self.focused = new_focus;

        // Activate new
        match self.focused {
            0 => self.package_input.set_active(true),
            n => self.components[n - 1].set_active(true),
        }

        tracing::info!(focused = self.focused, "focus changed");
    }

    fn cycle_focus(&mut self) {
        let new_focus = (self.focused + 1) % NUM_COMPONENTS;
        self.set_focus(new_focus);
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
        }

        tracing::debug!("exiting TUI mode");
        Tui::exit()?;

        Ok(())
    }

    fn render(&mut self) -> eyre::Result<()> {
        let render_error: RefCell<Option<eyre::Report>> = RefCell::new(None);
        let packages_table_area: RefCell<Rect> = RefCell::new(Rect::default());
        let package_info_area: RefCell<Rect> = RefCell::new(Rect::default());
        let show_help = self.show_help;
        let help_scroll = self.help_scroll;

        self.tui.draw(|frame| {
            let area = frame.area();

            // Compute component areas for mouse event routing
            let horizontal = Layout::horizontal([
                Constraint::Percentage(LEFT_PANEL_PERCENT),
                Constraint::Percentage(100 - LEFT_PANEL_PERCENT),
            ])
            .split(area);

            let left_vertical = Layout::vertical([
                Constraint::Length(INPUT_HEIGHT),
                Constraint::Length(FILTER_HEIGHT),
                Constraint::Percentage(100),
            ])
            .split(horizontal[0]);

            // Packages table area includes filter bar (index 1) and table (index 2)
            let filter_and_table = Rect {
                x: left_vertical[1].x,
                y: left_vertical[1].y,
                width: left_vertical[1].width,
                height: left_vertical[1].height + left_vertical[2].height,
            };
            *packages_table_area.borrow_mut() = filter_and_table;
            *package_info_area.borrow_mut() = horizontal[1];

            // Draw package input first
            if let Err(e) = self.package_input.draw(frame, &area) {
                *render_error.borrow_mut() = Some(e);
                return;
            }
            // Draw other components
            for component in self.components.iter_mut() {
                if let Err(e) = component.draw(frame, &area) {
                    *render_error.borrow_mut() = Some(e);
                    return;
                }
            }

            // Draw help overlay on top of everything
            if show_help {
                Self::draw_help(frame, area, help_scroll);
            }
        })?;

        self.packages_table_area = packages_table_area.into_inner();
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
            match key_event.code {
                KeyCode::Char('?') | KeyCode::Esc => {
                    tracing::debug!("closing help window");
                    self.show_help = false;
                    self.help_scroll = 0;
                }
                KeyCode::Char('j') => {
                    self.help_scroll = self.help_scroll.saturating_add(1);
                    tracing::trace!(scroll = self.help_scroll, "help scroll down");
                }
                KeyCode::Char('k') => {
                    self.help_scroll = self.help_scroll.saturating_sub(1);
                    tracing::trace!(scroll = self.help_scroll, "help scroll up");
                }
                _ => {
                    tracing::trace!(key = ?key_event.code, "key swallowed by help window");
                }
            }
            return Ok(Vec::new());
        }

        // Open help with ?
        if key_event.code == KeyCode::Char('?') {
            tracing::debug!("opening help window");
            self.show_help = true;
            self.help_scroll = 0;
            return Ok(Vec::new());
        }

        if key_event.code == KeyCode::Esc {
            self.should_exit = true;
        }

        // Handle focus switching
        use ratatui::crossterm::event::KeyModifiers;
        match (key_event.code, key_event.modifiers) {
            (KeyCode::Tab, KeyModifiers::NONE) => {
                self.cycle_focus();
                return Ok(Vec::new());
            }
            // Alt+j - focus down (input -> packages)
            (KeyCode::Char('j'), KeyModifiers::ALT) => {
                if self.focused < 1 {
                    self.set_focus(1);
                }
                return Ok(Vec::new());
            }
            // Alt+k - focus up (packages -> input, info -> input)
            (KeyCode::Char('k'), KeyModifiers::ALT) => {
                if self.focused > 0 {
                    self.set_focus(0);
                }
                return Ok(Vec::new());
            }
            // Alt+l - focus right (to info panel)
            (KeyCode::Char('l'), KeyModifiers::ALT) => {
                if self.focused != 2 {
                    self.set_focus(2);
                }
                return Ok(Vec::new());
            }
            // Alt+h - focus left (from info to packages)
            (KeyCode::Char('h'), KeyModifiers::ALT) => {
                if self.focused == 2 {
                    self.set_focus(1);
                }
                return Ok(Vec::new());
            }
            _ => {}
        }

        let mut actions = Vec::new();

        // Only send key events to the focused component
        match self.focused {
            0 => {
                if let Some(component_actions) = self.package_input.handle_key_event(key_event)? {
                    actions.extend(component_actions);
                }
            }
            n => {
                if let Some(component_actions) = self.components[n - 1].handle_key_event(key_event)? {
                    actions.extend(component_actions);
                }
            }
        }

        Ok(actions)
    }

    fn handle_mouse_event(&mut self, mouse_event: &MouseEvent) -> eyre::Result<Vec<Action>> {
        let mut actions = Vec::new();

        // Only handle scroll events
        match mouse_event.kind {
            MouseEventKind::ScrollUp | MouseEventKind::ScrollDown => {
                let (x, y) = (mouse_event.column, mouse_event.row);

                // Route to component based on cursor position
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

        Ok(())
    }

    fn handle_action(&mut self, action: &Action) -> eyre::Result<Vec<crate::event::Event>> {
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
            Action::SelectPackage(package) => {
                tracing::debug!(package_name = package.name, "package selected");
                events.push(crate::event::Event::PackageSelected(package.clone()));
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
        };

        Ok(events)
    }
}
