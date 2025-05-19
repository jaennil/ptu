# PTU

A Terminal User Interface (TUI) for Arch Linux's Pacman package manager, built with Rust and Ratatui.

![PacTUI Screenshot](assets/preview.png)

## Features

- Interactive package search with real-time results
- Package installation, update and removal directly from the interface
- Detailed package information display
- Vim-like keyboard navigation

## Requirements

- Arch Linux or an Arch-based distribution (Manjaro, EndeavourOS, etc.)
- Rust and Cargo

## Installation

### From Source

```bash
# Clone the repository
git clone https://github.com/jaennil/ptu.git
cd pactui

# Build and install
cargo build --release
sudo cp target/release/ptu /usr/local/bin/
```

## Usage

Launch the application by running:

```bash
ptu
```

### Keyboard Controls

| Key                | Action                         |
|--------------------|--------------------------------|
| `Tab`              | Switch between panels          |
| `Ctrl+j` / `Ctrl+k`| Switch between panels          |
| `j` / `k`          | Navigate up/down in the packages list |
| `g` / `G`          | Jump to top/bottom of the list |
| `i`                | Install selected package       |
| `I`                | Update and install package     |
| `r`                | Remove selected package        |
| `Esc`              | Exit application               |

## Components

The TUI consists of three main components:

1. **Package Input** - Search for packages by name
2. **Packages Table** - Displays matching packages with installation status
3. **Package Info** - Shows detailed information about the selected package

## Architecture

PacTUI follows an event-driven architecture with:

- Components that handle rendering and input
- Actions that represent user intentions
- Events that update application state
- A main App loop that orchestrates everything

## Development

### Project Structure

```
src/
├── action.rs        # User actions (search, install, etc.)
├── app.rs           # Main application logic
├── components/      # UI components
│   ├── package_info.rs
│   ├── package_input.rs
│   └── packages_table.rs
├── components.rs    # Component trait definition
├── event.rs         # Internal events
├── main.rs          # Application entry point
├── pacman.rs        # Pacman interface logic
├── panic_hook.rs    # Error handling
├── theme.rs         # UI theme definition
└── tui.rs           # Terminal initialization and management
```

### Building

```bash
cargo build
```

### Running in Development Mode

```bash
cargo run
```

## Dependencies

- `ratatui` - Terminal UI framework
- `crossterm` - Terminal manipulation
- `alpm` - Arch Linux Package Manager interface
- `color-eyre` - Error handling

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

1. Fork the repository
2. Create your feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add some amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request
