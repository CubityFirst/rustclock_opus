# Rust Clock

A small desktop clock with alarms, built in Rust with [egui/eframe](https://github.com/emilk/egui).

![Rust + egui](https://img.shields.io/badge/built%20with-egui%2Feframe-orange)

## Features

- **Live digital clock** — large HH:MM:SS display with the full date.
- **12h / 24h toggle.**
- **Alarms** — set by hour/minute with an optional label; the list auto-sorts by time.
- **Arm / disarm / delete** each alarm individually.
- **Ringing** — when an alarm's minute arrives, a centered modal **flashes** with the time and label and a **Dismiss** button. Each alarm rings at most once per calendar day.
- **Persistence** — alarms and the 12/24h setting are saved to your OS config dir and reloaded on startup:
  - Windows: `%APPDATA%\rustclock\alarms.json`
  - Linux: `$XDG_CONFIG_HOME/rustclock/alarms.json` (or `~/.config/rustclock/...`)

## Run

```sh
cargo run            # debug build
cargo run --release  # optimized; no console window on Windows
```

## Build

```sh
cargo build --release
# binary at target/release/rustclock_opus(.exe)
```

## License

MIT
