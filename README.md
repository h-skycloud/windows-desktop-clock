# Desktop Clock for Windows

[**Türkçe**](README.tr.md)

A lightweight desktop clock for Windows 11, written in Rust with native Win32 APIs. It displays local time in 24-hour format and updates once a minute by default. Enable seconds to update once a second.

## Getting started

Run `MasaustuSaati.exe`. The clock opens at the top center of the primary display by default. Drag it to move it, or right-click for quick settings and **Options**. At 100% background transparency, right-click a digit to open the menu.

## Stopwatch, countdown, and alarms

- Select the **Stopwatch** or **Countdown** panel beneath the clock to start, pause, or resume it. Open the matching right-click submenu to reset the timer, change its font or color, or hide its panel.
- Set a countdown duration in `HH:MM:SS` format under **Countdown → Set duration**. The clock shows a Windows alert when the countdown ends.
- Select **Alarms → Add daily alarm** and enter local time in `HH:MM` format. Use the alarm's submenu to enable, disable, or remove it. Alarms repeat daily while the app is running and Windows is awake.

Panel styles, countdown duration, and alarms are saved. A running stopwatch or countdown starts fresh when you reopen the app.

## Options

- **General** sets whether the clock starts at Windows sign-in and whether seconds appear.
- **Appearance** lets you choose a font, set separate sizes for hours, minutes, and seconds, change digit and background colors, and adjust transparency. The live preview shows changes immediately.
- **Position** shows the connected displays in their Windows layout. Drag the clock on the map or choose an alignment. Set the inset to 0 px to dock flush with the edge. Enter an angle or use the 45° shortcut.

Select **Save** to apply changes. Settings are stored in `%LOCALAPPDATA%\MasaustuSaati\settings.txt`. The program includes Space Mono Bold under the [SIL Open Font License 1.1](assets/Space-Mono-LICENSE.txt).

## Build

On Windows, install Rust and the Microsoft C++ Build Tools, then run:

```powershell
cargo build --locked --release
```

The executable is created at `target/release/MasaustuSaati.exe`.
