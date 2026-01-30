# PTU

A Terminal User Interface (TUI) for Arch Linux's Pacman package manager, built with Rust and Ratatui.

![PTU Screenshot](assets/preview.png)

## Features

- Interactive package search with instant results
- **AUR support** via yay/paru
- Package installation, update and removal
- **Multi-select packages** for batch install/remove
- Detailed package information display (scrollable)
- Vim-like keyboard navigation
- **Mouse scroll support** for packages table and info panel
- Async AUR search (non-blocking UI)

## Requirements

- Arch Linux or Arch-based distribution (Manjaro, EndeavourOS, etc.)
- Rust and Cargo
- `yay` or `paru` (optional, for AUR support)

## Installation

### From Source

```bash
git clone https://github.com/jaennil/ptu.git
cd ptu

cargo build --release
sudo cp target/release/ptu /usr/local/bin/
```

## Usage

```bash
ptu
```

Logs are written to `/tmp/ptu.log`.

### Keyboard Controls

| Key                | Action                              |
|--------------------|-------------------------------------|
| `Tab`              | Cycle between panels                |
| `Alt+j`            | Focus packages table                |
| `Alt+k`            | Focus search input                  |
| `Alt+l`            | Focus package info                  |
| `Alt+h`            | Focus packages table (from info)    |
| `j` / `k`          | Navigate up/down (table) or scroll (info) |
| `Ctrl+d` / `Ctrl+u`| Page down/up in package info        |
| `g` / `G`          | Jump to top/bottom of the list      |
| `Space`            | Toggle package selection (multi-select) |
| `i`                | Install selected package            |
| `I`                | Batch install selected packages     |
| `r`                | Remove selected package             |
| `R`                | Batch remove selected packages      |
| `Mouse scroll`     | Scroll packages table or info panel |
| `Esc`              | Exit application                    |

## Components

1. **Package Input** - Search for packages by name
2. **Packages Table** - Displays matching packages with installation status
3. **Package Info** - Shows detailed information about the selected package (scrollable)

## Architecture

Event-driven architecture with async support:

- **Instant pacman search** - results appear immediately on keystroke
- **Async AUR search** - non-blocking HTTP requests via tokio/reqwest
- **Debounced AUR queries** - 300ms delay to avoid excessive API calls
- Components handle rendering and input
- Actions represent user intentions
- Events update application state

## Project Structure

```
src/
├── action.rs           # User actions (search, install, etc.)
├── app.rs              # Main application logic + focus management + tokio runtime
├── aur.rs              # AUR API interface (async)
├── components/
│   ├── package_info.rs # Scrollable package details
│   ├── package_input.rs
│   └── packages_table.rs
├── components.rs       # Component trait
├── event.rs            # Internal events
├── layout.rs           # Layout constants
├── logging.rs          # Log configuration
├── main.rs             # Entry point
├── pacman.rs           # Pacman/alpm interface
├── panic_hook.rs       # Error handling
├── theme.rs            # UI theme
└── tui.rs              # Terminal management
```

## Dependencies

- `ratatui` - Terminal UI framework
- `alpm` - Arch Linux Package Manager interface
- `reqwest` - Async HTTP client (AUR)
- `tokio` - Async runtime
- `color-eyre` - Error handling
- `tracing` - Logging

## Contributing

Contributions are welcome!

1. Fork the repository
2. Create your feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes
4. Push to the branch
5. Open a Pull Request
