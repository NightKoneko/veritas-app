# veritas
[![veritas](https://img.shields.io/badge/veritas-Discord-%235865F2.svg)](https://discord.gg/Y9kSnPk95H)



This is still a work in progress. This only works with global-beta. Do NOT attempt to use this while the official version is running. It will not work.

# Usage
Download a build from Releases. Then, download the executable's module [veritas](https://github.com/hessiser/veritas) and place in the root directory of the executable. This module will need to be downloaded almost every patch. 

1. Run the game.

2. In the executable's menubar, click `Tools→Spawn Server`.

3. Enter battle in-game and press `Ctrl+M` to toggle. Damage should now be logging and visualizations (graphs) updating accordingly.

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
