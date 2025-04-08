# veritas
This is still a work in progress. This only works with global-beta. Do NOT attempt to use this while the official version is running. It will not work.

# Usage
Download a build from Releases. Then, download the executable's module [veritas](https://github.com/hessiser/veritas) and place in the root directory of the executable. This module will need to be downloaded almost every patch.

# Building
## Prerequisites
[Rust](https://www.rust-lang.org/tools/install)

## Steps
1. In a terminal, run:
```
git clone https://github.com/NightKoneko/veritas-app.git
cd veritas-app
cargo run --release
target/release/veritas.exe
```

2. Run the game.

3. In the executable's menubar, click `Tools→Spawn Server`.

3. Enter battle in-game. Damage should now be logging and visualizations (graphs) updating accordingly.
