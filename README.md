# 🚀 Damage-Analyzer-GUI-RS 🎯

Welcome to **Damage-Analyzer-GUI-RS**! 🎮✨ This is a Rust-powered, blazingly fast 🚀 reimplementation of [Damage-Analyzer-GUI](https://github.com/NightKoneko/Damage-Analyzer-GUI), but with Rust 🦀 and **egui** for sleek, buttery-smooth visuals! 🎨🔥

## Made to be used with [Veritas](https://github.com/hessiser/veritas)

This is still a work in progress.

## Why Rust? 🤔
- 🚀 **Blazing fast** performance 💨
- 💪 **Memory safety** with zero cost abstractions 🛡️
- 🏎️ **Concurrency without data races** (Tokio-powered!) 🧵
- ❌ No more Python dependencies!

## Features 🎯
✅ **Real-time damage tracking** 📊 - View damage output as it happens! 🔥
✅ **Interactive graphs & charts** 📈 - Beautiful, dynamic visualization of combat stats! 🎨
✅ **Damage logs** 📝 - Export combat data into **CSV files** for further analysis! 💾
✅ **Live server connection** 🌐 - Connect & fetch data from a game server in real time! ⚡
✅ **Sleek, customizable UI** 🖥️ - Dark mode included by default! 🌑✨
✅ **Window pinning** 📌 - Keep it on top for uninterrupted tracking! 👀

## How to use:

1. `git clone https://github.com/NightKoneko/Damage-Analyzer-GUI-RS.git`

2. Inject [Veritas](https://github.com/hessiser/veritas) (You can download a prebuilt Veritas DLL from releases or alternatively build it yourself) into the game. This can be done with a tool like [Genshin Utility](https://github.com/lanylow/genshin-utility) or Cheat Engine.

   * In the case of [Genshin Utility](https://github.com/lanylow/genshin-utility), rename `veritas.dll` to `library.dll` and replace the previous `library.dll` with it. **Make sure to run `loader.exe` as administrator.**

3. `cargo run --release`

4. Click the 'Connect' button

5. Enter battle in-game

6. Damage should now be logging and visualizations (graphs) updating accordingly.

## How It Works ⚙️
- **Connect to a game server** 🎮
- **Track combat data in real time** 🔄
- **View visualized damage breakdowns** 📊
- **Export logs for analysis** 📑

## Contributing 🤝
Want to help make this better? PRs welcome! 🛠️

## License 📜
MIT License - Free to use, modify, and distribute! 👐

---

✨ Built with ❤️, Rust 🦀, and egui 🎨 by [NightKoneko](https://github.com/NightKoneko)!

