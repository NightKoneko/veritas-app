# Damage-Analyzer-GUI-RS

Made to be used with [Veritas](https://github.com/hessiser/veritas)

This is still a work in progress

## How to use:

1. `git clone https://github.com/NightKoneko/Damage-Analyzer-GUI-RS.git`

2. Inject [Veritas](https://github.com/hessiser/veritas) (You can download a prebuilt Veritas DLL from releases or alternatively build it yourself) into the game. This can be done with a tool like [Genshin Utility](https://github.com/lanylow/genshin-utility) or Cheat Engine.

   * In the case of [Genshin Utility](https://github.com/lanylow/genshin-utility), rename `veritas.dll` to `library.dll` and replace the previous `library.dll` with it. **Make sure to run `loader.exe` as administrator.**

3. `cargo run --release`

4. Click the 'Connect' button

5. Enter battle in-game

6. Damage should now be logging and visualizations (graphs) updating accordingly.
