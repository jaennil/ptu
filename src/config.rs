use std::path::PathBuf;

use color_eyre::eyre;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use serde::Deserialize;

// ---------------------------------------------------------------------------
// Raw TOML structs (deserialized directly from config file)
// ---------------------------------------------------------------------------

#[derive(Deserialize, Default)]
struct ConfigFile {
    keys: Option<KeysConfig>,
}

#[derive(Deserialize, Default)]
struct KeysConfig {
    global: Option<GlobalKeysRaw>,
    tabs: Option<TabsKeysRaw>,
    search_input: Option<SearchInputKeysRaw>,
    packages_table: Option<PackagesTableKeysRaw>,
    installed_table: Option<InstalledTableKeysRaw>,
    package_info: Option<PackageInfoKeysRaw>,
    help: Option<HelpKeysRaw>,
}

#[derive(Deserialize, Default)]
struct GlobalKeysRaw {
    toggle_help: Option<KeyBinding>,
    quit: Option<KeyBinding>,
    cycle_focus: Option<KeyBinding>,
    focus_down: Option<KeyBinding>,
    focus_up: Option<KeyBinding>,
    focus_right: Option<KeyBinding>,
    focus_left: Option<KeyBinding>,
}

#[derive(Deserialize, Default)]
struct TabsKeysRaw {
    search: Option<KeyBinding>,
    installed: Option<KeyBinding>,
}

#[derive(Deserialize, Default)]
struct SearchInputKeysRaw {
    toggle_search_mode: Option<KeyBinding>,
    search_files: Option<KeyBinding>,
    delete_word: Option<KeyBinding>,
    delete_char: Option<KeyBinding>,
}

#[derive(Deserialize, Default)]
struct PackagesTableKeysRaw {
    next: Option<KeyBinding>,
    previous: Option<KeyBinding>,
    first: Option<KeyBinding>,
    last: Option<KeyBinding>,
    install: Option<KeyBinding>,
    remove: Option<KeyBinding>,
    batch_install: Option<KeyBinding>,
    batch_remove: Option<KeyBinding>,
    multi_select: Option<KeyBinding>,
    filter_mode: Option<KeyBinding>,
    filter: Option<FilterKeysRaw>,
}

#[derive(Deserialize, Default)]
struct FilterKeysRaw {
    cycle_install: Option<KeyBinding>,
    toggle_aur: Option<KeyBinding>,
    toggle_pacman: Option<KeyBinding>,
    clear_all: Option<KeyBinding>,
    exit: Option<KeyBinding>,
}

#[derive(Deserialize, Default)]
struct InstalledTableKeysRaw {
    next: Option<KeyBinding>,
    previous: Option<KeyBinding>,
    first: Option<KeyBinding>,
    last: Option<KeyBinding>,
    cycle_sort: Option<KeyBinding>,
    toggle_sort_direction: Option<KeyBinding>,
    remove: Option<KeyBinding>,
    batch_remove: Option<KeyBinding>,
    multi_select: Option<KeyBinding>,
}

#[derive(Deserialize, Default)]
struct PackageInfoKeysRaw {
    scroll_down: Option<KeyBinding>,
    scroll_up: Option<KeyBinding>,
    page_down: Option<KeyBinding>,
    page_up: Option<KeyBinding>,
    open_url: Option<KeyBinding>,
}

#[derive(Deserialize, Default)]
struct HelpKeysRaw {
    close: Option<KeyBinding>,
    scroll_down: Option<KeyBinding>,
    scroll_up: Option<KeyBinding>,
}

/// A key binding can be a single string or a list of strings.
#[derive(Deserialize, Clone, Debug)]
#[serde(untagged)]
enum KeyBinding {
    Single(String),
    Multiple(Vec<String>),
}

impl KeyBinding {
    fn into_strings(self) -> Vec<String> {
        match self {
            KeyBinding::Single(s) => vec![s],
            KeyBinding::Multiple(v) => v,
        }
    }
}

// ---------------------------------------------------------------------------
// Resolved structs (contain Vec<KeyEvent>)
// ---------------------------------------------------------------------------

pub struct Keymap {
    pub global: GlobalKeys,
    pub tabs: TabsKeys,
    pub search_input: SearchInputKeys,
    pub packages_table: PackagesTableKeys,
    pub installed_table: InstalledTableKeys,
    pub package_info: PackageInfoKeys,
    pub help: HelpKeys,
}

pub struct GlobalKeys {
    pub toggle_help: Vec<KeyEvent>,
    pub quit: Vec<KeyEvent>,
    pub cycle_focus: Vec<KeyEvent>,
    pub focus_down: Vec<KeyEvent>,
    pub focus_up: Vec<KeyEvent>,
    pub focus_right: Vec<KeyEvent>,
    pub focus_left: Vec<KeyEvent>,
}

pub struct TabsKeys {
    pub search: Vec<KeyEvent>,
    pub installed: Vec<KeyEvent>,
}

pub struct SearchInputKeys {
    pub toggle_search_mode: Vec<KeyEvent>,
    pub search_files: Vec<KeyEvent>,
    pub delete_word: Vec<KeyEvent>,
    pub delete_char: Vec<KeyEvent>,
}

pub struct PackagesTableKeys {
    pub next: Vec<KeyEvent>,
    pub previous: Vec<KeyEvent>,
    pub first: Vec<KeyEvent>,
    pub last: Vec<KeyEvent>,
    pub install: Vec<KeyEvent>,
    pub remove: Vec<KeyEvent>,
    pub batch_install: Vec<KeyEvent>,
    pub batch_remove: Vec<KeyEvent>,
    pub multi_select: Vec<KeyEvent>,
    pub filter_mode: Vec<KeyEvent>,
    pub filter: FilterKeys,
}

pub struct FilterKeys {
    pub cycle_install: Vec<KeyEvent>,
    pub toggle_aur: Vec<KeyEvent>,
    pub toggle_pacman: Vec<KeyEvent>,
    pub clear_all: Vec<KeyEvent>,
    pub exit: Vec<KeyEvent>,
}

pub struct InstalledTableKeys {
    pub next: Vec<KeyEvent>,
    pub previous: Vec<KeyEvent>,
    pub first: Vec<KeyEvent>,
    pub last: Vec<KeyEvent>,
    pub cycle_sort: Vec<KeyEvent>,
    pub toggle_sort_direction: Vec<KeyEvent>,
    pub remove: Vec<KeyEvent>,
    pub batch_remove: Vec<KeyEvent>,
    pub multi_select: Vec<KeyEvent>,
}

pub struct PackageInfoKeys {
    pub scroll_down: Vec<KeyEvent>,
    pub scroll_up: Vec<KeyEvent>,
    pub page_down: Vec<KeyEvent>,
    pub page_up: Vec<KeyEvent>,
    pub open_url: Vec<KeyEvent>,
}

pub struct HelpKeys {
    pub close: Vec<KeyEvent>,
    pub scroll_down: Vec<KeyEvent>,
    pub scroll_up: Vec<KeyEvent>,
}

// ---------------------------------------------------------------------------
// Defaults (current hardcoded bindings)
// ---------------------------------------------------------------------------

impl Default for Keymap {
    fn default() -> Self {
        Self {
            global: GlobalKeys::default(),
            tabs: TabsKeys::default(),
            search_input: SearchInputKeys::default(),
            packages_table: PackagesTableKeys::default(),
            installed_table: InstalledTableKeys::default(),
            package_info: PackageInfoKeys::default(),
            help: HelpKeys::default(),
        }
    }
}

impl Default for GlobalKeys {
    fn default() -> Self {
        Self {
            toggle_help: vec![key('?')],
            quit: vec![key_special(KeyCode::Esc)],
            cycle_focus: vec![key_special(KeyCode::Tab)],
            focus_down: vec![key_mod('j', KeyModifiers::ALT)],
            focus_up: vec![key_mod('k', KeyModifiers::ALT)],
            focus_right: vec![key_mod('l', KeyModifiers::ALT)],
            focus_left: vec![key_mod('h', KeyModifiers::ALT)],
        }
    }
}

impl Default for TabsKeys {
    fn default() -> Self {
        Self {
            search: vec![
                key_special(KeyCode::F(1)),
                key_mod('1', KeyModifiers::ALT),
                key_mod('+', KeyModifiers::ALT),
            ],
            installed: vec![
                key_special(KeyCode::F(2)),
                key_mod('2', KeyModifiers::ALT),
                key_mod('[', KeyModifiers::ALT),
            ],
        }
    }
}

impl Default for SearchInputKeys {
    fn default() -> Self {
        Self {
            toggle_search_mode: vec![key_mod('f', KeyModifiers::CONTROL)],
            search_files: vec![key_special(KeyCode::Enter)],
            delete_word: vec![key_mod('w', KeyModifiers::CONTROL)],
            delete_char: vec![key_special(KeyCode::Backspace)],
        }
    }
}

impl Default for PackagesTableKeys {
    fn default() -> Self {
        Self {
            next: vec![key('j')],
            previous: vec![key('k')],
            first: vec![key('g')],
            last: vec![key_shift('G')],
            install: vec![key('i')],
            remove: vec![key('r')],
            batch_install: vec![key_shift('I')],
            batch_remove: vec![key_shift('R')],
            multi_select: vec![key_special(KeyCode::Char(' '))],
            filter_mode: vec![key('f')],
            filter: FilterKeys::default(),
        }
    }
}

impl Default for FilterKeys {
    fn default() -> Self {
        Self {
            cycle_install: vec![key('i')],
            toggle_aur: vec![key('a')],
            toggle_pacman: vec![key('p')],
            clear_all: vec![key('c')],
            exit: vec![key_special(KeyCode::Esc)],
        }
    }
}

impl Default for InstalledTableKeys {
    fn default() -> Self {
        Self {
            next: vec![key('j')],
            previous: vec![key('k')],
            first: vec![key('g')],
            last: vec![key_shift('G')],
            cycle_sort: vec![key('s')],
            toggle_sort_direction: vec![key_shift('S')],
            remove: vec![key('r')],
            batch_remove: vec![key_shift('R')],
            multi_select: vec![key_special(KeyCode::Char(' '))],
        }
    }
}

impl Default for PackageInfoKeys {
    fn default() -> Self {
        Self {
            scroll_down: vec![key('j')],
            scroll_up: vec![key('k')],
            page_down: vec![key_mod('d', KeyModifiers::CONTROL)],
            page_up: vec![key_mod('u', KeyModifiers::CONTROL)],
            open_url: vec![key('o')],
        }
    }
}

impl Default for HelpKeys {
    fn default() -> Self {
        Self {
            close: vec![key('?'), key_special(KeyCode::Esc)],
            scroll_down: vec![key('j')],
            scroll_up: vec![key('k')],
        }
    }
}

// ---------------------------------------------------------------------------
// Helper constructors for KeyEvent
// ---------------------------------------------------------------------------

fn key(c: char) -> KeyEvent {
    if c.is_ascii_uppercase() {
        KeyEvent::new(KeyCode::Char(c), KeyModifiers::SHIFT)
    } else {
        KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE)
    }
}

fn key_shift(c: char) -> KeyEvent {
    KeyEvent::new(KeyCode::Char(c), KeyModifiers::SHIFT)
}

fn key_mod(c: char, modifiers: KeyModifiers) -> KeyEvent {
    KeyEvent::new(KeyCode::Char(c), modifiers)
}

fn key_special(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

// ---------------------------------------------------------------------------
// Key string parsing
// ---------------------------------------------------------------------------

/// Parse a key string like "Ctrl+f", "Alt+j", "Esc", "G", "F1", "Space"
pub fn parse_key(s: &str) -> Result<KeyEvent, String> {
    let parts: Vec<&str> = s.split('+').collect();

    let mut modifiers = KeyModifiers::NONE;
    let key_part;

    if parts.len() == 1 {
        key_part = parts[0];
    } else {
        // Last part is the key, everything before is modifiers
        key_part = parts.last().unwrap();
        for &modifier in &parts[..parts.len() - 1] {
            match modifier.to_lowercase().as_str() {
                "ctrl" => modifiers |= KeyModifiers::CONTROL,
                "alt" => modifiers |= KeyModifiers::ALT,
                "shift" => modifiers |= KeyModifiers::SHIFT,
                other => return Err(format!("unknown modifier: {}", other)),
            }
        }
    }

    let code = parse_key_code(key_part)?;

    // Auto-add SHIFT for uppercase single characters (unless SHIFT already set via modifier)
    if let KeyCode::Char(c) = code {
        if c.is_ascii_uppercase() {
            modifiers |= KeyModifiers::SHIFT;
        }
    }

    Ok(KeyEvent::new(code, modifiers))
}

fn parse_key_code(s: &str) -> Result<KeyCode, String> {
    match s.to_lowercase().as_str() {
        "esc" | "escape" => Ok(KeyCode::Esc),
        "tab" => Ok(KeyCode::Tab),
        "enter" | "return" => Ok(KeyCode::Enter),
        "backspace" => Ok(KeyCode::Backspace),
        "space" => Ok(KeyCode::Char(' ')),
        "up" => Ok(KeyCode::Up),
        "down" => Ok(KeyCode::Down),
        "left" => Ok(KeyCode::Left),
        "right" => Ok(KeyCode::Right),
        "home" => Ok(KeyCode::Home),
        "end" => Ok(KeyCode::End),
        "pageup" => Ok(KeyCode::PageUp),
        "pagedown" => Ok(KeyCode::PageDown),
        "delete" | "del" => Ok(KeyCode::Delete),
        "insert" | "ins" => Ok(KeyCode::Insert),
        "f1" => Ok(KeyCode::F(1)),
        "f2" => Ok(KeyCode::F(2)),
        "f3" => Ok(KeyCode::F(3)),
        "f4" => Ok(KeyCode::F(4)),
        "f5" => Ok(KeyCode::F(5)),
        "f6" => Ok(KeyCode::F(6)),
        "f7" => Ok(KeyCode::F(7)),
        "f8" => Ok(KeyCode::F(8)),
        "f9" => Ok(KeyCode::F(9)),
        "f10" => Ok(KeyCode::F(10)),
        "f11" => Ok(KeyCode::F(11)),
        "f12" => Ok(KeyCode::F(12)),
        _ => {
            // Single character
            let chars: Vec<char> = s.chars().collect();
            if chars.len() == 1 {
                Ok(KeyCode::Char(chars[0]))
            } else {
                Err(format!("unknown key: {}", s))
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Key matching
// ---------------------------------------------------------------------------

/// Check if a key event matches any of the bindings.
pub fn key_matches(event: &KeyEvent, bindings: &[KeyEvent]) -> bool {
    bindings.iter().any(|b| {
        b.code == event.code && b.modifiers == event.modifiers
    })
}

// ---------------------------------------------------------------------------
// Resolve: merge raw config onto defaults
// ---------------------------------------------------------------------------

fn resolve_binding(raw: Option<KeyBinding>, default: Vec<KeyEvent>) -> Vec<KeyEvent> {
    match raw {
        None => default,
        Some(kb) => {
            let strings = kb.into_strings();
            let mut resolved = Vec::new();
            for s in strings {
                match parse_key(&s) {
                    Ok(ke) => resolved.push(ke),
                    Err(e) => {
                        tracing::warn!(key = %s, error = %e, "invalid key binding in config, using default for this binding");
                        return default;
                    }
                }
            }
            if resolved.is_empty() {
                default
            } else {
                resolved
            }
        }
    }
}

fn resolve_global(raw: Option<GlobalKeysRaw>) -> GlobalKeys {
    let defaults = GlobalKeys::default();
    let raw = raw.unwrap_or_default();
    GlobalKeys {
        toggle_help: resolve_binding(raw.toggle_help, defaults.toggle_help),
        quit: resolve_binding(raw.quit, defaults.quit),
        cycle_focus: resolve_binding(raw.cycle_focus, defaults.cycle_focus),
        focus_down: resolve_binding(raw.focus_down, defaults.focus_down),
        focus_up: resolve_binding(raw.focus_up, defaults.focus_up),
        focus_right: resolve_binding(raw.focus_right, defaults.focus_right),
        focus_left: resolve_binding(raw.focus_left, defaults.focus_left),
    }
}

fn resolve_tabs(raw: Option<TabsKeysRaw>) -> TabsKeys {
    let defaults = TabsKeys::default();
    let raw = raw.unwrap_or_default();
    TabsKeys {
        search: resolve_binding(raw.search, defaults.search),
        installed: resolve_binding(raw.installed, defaults.installed),
    }
}

fn resolve_search_input(raw: Option<SearchInputKeysRaw>) -> SearchInputKeys {
    let defaults = SearchInputKeys::default();
    let raw = raw.unwrap_or_default();
    SearchInputKeys {
        toggle_search_mode: resolve_binding(raw.toggle_search_mode, defaults.toggle_search_mode),
        search_files: resolve_binding(raw.search_files, defaults.search_files),
        delete_word: resolve_binding(raw.delete_word, defaults.delete_word),
        delete_char: resolve_binding(raw.delete_char, defaults.delete_char),
    }
}

fn resolve_filter(raw: Option<FilterKeysRaw>) -> FilterKeys {
    let defaults = FilterKeys::default();
    let raw = raw.unwrap_or_default();
    FilterKeys {
        cycle_install: resolve_binding(raw.cycle_install, defaults.cycle_install),
        toggle_aur: resolve_binding(raw.toggle_aur, defaults.toggle_aur),
        toggle_pacman: resolve_binding(raw.toggle_pacman, defaults.toggle_pacman),
        clear_all: resolve_binding(raw.clear_all, defaults.clear_all),
        exit: resolve_binding(raw.exit, defaults.exit),
    }
}

fn resolve_packages_table(raw: Option<PackagesTableKeysRaw>) -> PackagesTableKeys {
    let defaults = PackagesTableKeys::default();
    let raw = raw.unwrap_or_default();
    PackagesTableKeys {
        next: resolve_binding(raw.next, defaults.next),
        previous: resolve_binding(raw.previous, defaults.previous),
        first: resolve_binding(raw.first, defaults.first),
        last: resolve_binding(raw.last, defaults.last),
        install: resolve_binding(raw.install, defaults.install),
        remove: resolve_binding(raw.remove, defaults.remove),
        batch_install: resolve_binding(raw.batch_install, defaults.batch_install),
        batch_remove: resolve_binding(raw.batch_remove, defaults.batch_remove),
        multi_select: resolve_binding(raw.multi_select, defaults.multi_select),
        filter_mode: resolve_binding(raw.filter_mode, defaults.filter_mode),
        filter: resolve_filter(raw.filter),
    }
}

fn resolve_installed_table(raw: Option<InstalledTableKeysRaw>) -> InstalledTableKeys {
    let defaults = InstalledTableKeys::default();
    let raw = raw.unwrap_or_default();
    InstalledTableKeys {
        next: resolve_binding(raw.next, defaults.next),
        previous: resolve_binding(raw.previous, defaults.previous),
        first: resolve_binding(raw.first, defaults.first),
        last: resolve_binding(raw.last, defaults.last),
        cycle_sort: resolve_binding(raw.cycle_sort, defaults.cycle_sort),
        toggle_sort_direction: resolve_binding(raw.toggle_sort_direction, defaults.toggle_sort_direction),
        remove: resolve_binding(raw.remove, defaults.remove),
        batch_remove: resolve_binding(raw.batch_remove, defaults.batch_remove),
        multi_select: resolve_binding(raw.multi_select, defaults.multi_select),
    }
}

fn resolve_package_info(raw: Option<PackageInfoKeysRaw>) -> PackageInfoKeys {
    let defaults = PackageInfoKeys::default();
    let raw = raw.unwrap_or_default();
    PackageInfoKeys {
        scroll_down: resolve_binding(raw.scroll_down, defaults.scroll_down),
        scroll_up: resolve_binding(raw.scroll_up, defaults.scroll_up),
        page_down: resolve_binding(raw.page_down, defaults.page_down),
        page_up: resolve_binding(raw.page_up, defaults.page_up),
        open_url: resolve_binding(raw.open_url, defaults.open_url),
    }
}

fn resolve_help(raw: Option<HelpKeysRaw>) -> HelpKeys {
    let defaults = HelpKeys::default();
    let raw = raw.unwrap_or_default();
    HelpKeys {
        close: resolve_binding(raw.close, defaults.close),
        scroll_down: resolve_binding(raw.scroll_down, defaults.scroll_down),
        scroll_up: resolve_binding(raw.scroll_up, defaults.scroll_up),
    }
}

fn resolve_config(config: ConfigFile) -> Keymap {
    let keys = config.keys.unwrap_or_default();
    Keymap {
        global: resolve_global(keys.global),
        tabs: resolve_tabs(keys.tabs),
        search_input: resolve_search_input(keys.search_input),
        packages_table: resolve_packages_table(keys.packages_table),
        installed_table: resolve_installed_table(keys.installed_table),
        package_info: resolve_package_info(keys.package_info),
        help: resolve_help(keys.help),
    }
}

// ---------------------------------------------------------------------------
// Config file path
// ---------------------------------------------------------------------------

fn config_path() -> PathBuf {
    let base = std::env::var("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
            PathBuf::from(home).join(".config")
        });
    base.join("ptu").join("config.toml")
}

// ---------------------------------------------------------------------------
// Public load function
// ---------------------------------------------------------------------------

const DEFAULT_CONFIG: &str = r#"# ptu keybindings configuration
# Customize any keybinding below. Delete a line to use the default.
#
# Key format:
#   Single char:    "j", "?", "G", "+"
#   With modifier:  "Ctrl+f", "Alt+j", "Shift+G"
#   Special keys:   "Esc", "Tab", "Enter", "Backspace", "Space", "F1".."F12"
#   Multiple keys:  ["F1", "Alt+1"]

[keys.global]
toggle_help = "?"
quit = "Esc"
cycle_focus = "Tab"
focus_down = "Alt+j"
focus_up = "Alt+k"
focus_right = "Alt+l"
focus_left = "Alt+h"

[keys.tabs]
search = ["F1", "Alt+1", "Alt++"]
installed = ["F2", "Alt+2", "Alt+["]

[keys.search_input]
toggle_search_mode = "Ctrl+f"
search_files = "Enter"
delete_word = "Ctrl+w"
delete_char = "Backspace"

[keys.packages_table]
next = "j"
previous = "k"
first = "g"
last = "G"
install = "i"
remove = "r"
batch_install = "I"
batch_remove = "R"
multi_select = "Space"
filter_mode = "f"

[keys.packages_table.filter]
cycle_install = "i"
toggle_aur = "a"
toggle_pacman = "p"
clear_all = "c"
exit = "Esc"

[keys.installed_table]
next = "j"
previous = "k"
first = "g"
last = "G"
cycle_sort = "s"
toggle_sort_direction = "S"
remove = "r"
batch_remove = "R"
multi_select = "Space"

[keys.package_info]
scroll_down = "j"
scroll_up = "k"
page_down = "Ctrl+d"
page_up = "Ctrl+u"
open_url = "o"

[keys.help]
close = ["?", "Esc"]
scroll_down = "j"
scroll_up = "k"
"#;

fn write_default_config(path: &std::path::Path) {
    if let Some(parent) = path.parent() {
        if let Err(e) = std::fs::create_dir_all(parent) {
            tracing::warn!(path = %parent.display(), error = %e, "failed to create config directory");
            return;
        }
    }
    match std::fs::write(path, DEFAULT_CONFIG) {
        Ok(()) => tracing::info!(path = %path.display(), "created default config file"),
        Err(e) => tracing::warn!(path = %path.display(), error = %e, "failed to write default config file"),
    }
}

pub fn load() -> eyre::Result<Keymap> {
    let path = config_path();
    tracing::debug!(path = %path.display(), "looking for config file");

    if !path.exists() {
        tracing::info!("no config file found, creating default");
        write_default_config(&path);
        return Ok(Keymap::default());
    }

    tracing::info!(path = %path.display(), "loading config file");
    let content = std::fs::read_to_string(&path)?;

    let config: ConfigFile = toml::from_str(&content).map_err(|e| {
        tracing::error!(path = %path.display(), error = %e, "failed to parse config file");
        eyre::eyre!("failed to parse config file {}: {}", path.display(), e)
    })?;

    let keymap = resolve_config(config);
    tracing::info!("config loaded successfully");
    Ok(keymap)
}
