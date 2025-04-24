# veritas
[![veritas](https://img.shields.io/badge/veritas-Discord-%235865F2.svg)](https://discord.gg/Y9kSnPk95H)

This is still a work in progress.

# Usage
Download a build of this app from Releases. Then, download a build of this app's module [veritas](https://github.com/hessiser/veritas) from Releases and place in the root directory of this app. This module will need to be downloaded almost every patch. 

1. Run the game and the app.

2. In the app's menubar, click `Tools→Spawn Server`. Once the server is spawned, this app will be connected to the server, as indicated in the statusbar.

3. Enter battle in-game and press `Ctrl+M` to toggle. Damage should now be logging and visualizations (graphs) updating accordingly.

# Overlay Shortcuts
- `Ctrl+M` to toggle menu
- `Ctrl+H` to hide the overlay

# Troubleshooting
- **The in-game overlay is not showing.**

  Disable any other apps that uses an overlay with the game (e.g. Discord) and restart the game.


# Building
## Prerequisites
[Rust](https://www.rust-lang.org/tools/install)

## Steps
1. In a terminal, run:
```
git clone https://github.com/NightKoneko/veritas-app.git
cd veritas-app
cargo build --release
```
