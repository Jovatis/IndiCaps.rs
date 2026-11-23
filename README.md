<img width="65" height="65" alt="IndiCapsRustLogo" src="https://github.com/user-attachments/assets/422f53ad-8815-4c37-9cdc-db1efcf00ec4" /> # IndiCaps.rs

**IndiCaps v2.0.0** — a native Rust Caps Lock & Num Lock indicator for Windows.

Every time you toggle Caps Lock or Num Lock, a small notification built from the original PNG artwork appears at the position you chose, then fades away. No main window, no click interception, no telemetry — just a tiny tray-resident app.

Designed and crafted by **Jovatis**.

> This repository (`indicaps.rs`) is the native Rust rewrite of the [original C# IndiCaps](https://github.com/Jovatis/IndiCaps). The application itself is still called **IndiCaps**.

---

## Download

Two archives are published on the **Releases** page:

- **`IndiCaps-v2.0.0.zip`** — the ready-to-run build. Extract it anywhere and launch `IndiCaps.exe`; no runtime and no `assets/` folder are needed.
- **`IndiCaps.rs-v2.0.0-source.zip`** — a clean source archive containing only `Cargo.toml`, `Cargo.lock`, `build.rs`, `.cargo/`, `src/`, `assets/`, `README.md`, `LICENSE` and `.gitignore`. No compiled binary, no `target/`, no dev-machine files.

You can also `git clone` this repository and build it yourself — see [Building from source](#building-from-source).

---

## Features

- **Official IndiCaps logo** embedded as the EXE icon and shown in the settings panel header, next to the title
- **Caps Lock and Num Lock** notifications using the original `bgon.png` / `bgoff.png` artwork and embedded Poppins font
- Always on top, but **click-through**: it never steals a click or the focus
- Original WAV sounds with a 0–100 % volume slider; a sound plays only when the global sound **and** the matching indicator are enabled
- **Settings panel** rendered entirely in GDI+, with a 4-category burger menu: **Settings**, **Stats**, **Shortcuts**, **Display**
- **18 languages** with proper fonts and RTL support for Arabic — English by default
- **Stats**: Caps/Num toggle counts, time spent with each key on, tracking duration shown in whole minutes, and a reset protected by an explicit **Yes / No** confirmation
- Configurable panel shortcut (default `Ctrl + Shift + I`) with one-key capture and `Esc` to cancel
- Notification duration (`0.8 s / 1.5 s / 3 s / ∞`), panel/overlay opacity (`20–100 %`) and sound volume (`0–100 %`) on notched sliders
- **7 snap positions** plus a smooth drag editor with a dimmed full-screen preview; `Esc` cancels
- Tray icon: left-click opens the settings, right-click gives Open / Quit
- Single instance (mutex) — no duplicate tray icons on double launch
- **Machine-bound JSON config** in `%LOCALAPPDATA%\IndiCaps\<machine-id>\config.json`: non-roaming, tied to a volume serial + computer-name hash, never to an IP address
- **No user settings baked into the EXE**: a fresh machine always starts with the factory profile
- **One self-contained EXE (~514 KB)**: the official ICO, PNG, TTF and WAV assets are embedded at compile time — no runtime, no `assets/` folder, no Rust dependencies

---

## Installation (plug & play)

Download `IndiCaps-v2.0.0.zip` from the **GitHub Releases** page. The zip contains:

```
IndiCaps-v2.0.0.zip
├── IndiCaps.exe
├── README.md
└── LICENSE
```

Extract it anywhere you like and run `IndiCaps.exe`. There is no `assets/` folder to keep next to the executable anymore — everything is inside the EXE.

To quit: right-click the tray icon → **Quit**.

---

## Factory profile (new machine)

- Caps indicator: on · Num indicator: on
- Sound: on, volume 100 % · Fade animation: on
- Launch at Windows startup: off
- Language: English
- Stats: all zero
- Shortcut: `Ctrl + Shift + I`
- Notification duration: `0.8 s` · Panel opacity: `100 %` · Overlay opacity: `100 %`
- Position: bottom-right (automatic)

If a `config.json` is copied from another PC, its `MachineId` will not match and IndiCaps will ignore it, falling back to the factory profile.

---

## Usage

- Left-click the tray icon: open/close settings
- Right-click the tray icon: Open settings / Quit
- `Esc` or loss of focus: close the panel
- `Ctrl + Shift + I` (configurable): toggle the panel
- In the position editor: drag the indicator, release to validate; `Esc` cancels
- `IndiCaps.exe --debug`: writes `%APPDATA%\IndiCaps\debug.log`

---

## Building from source

Requirements: **Rust stable**, Windows x64 with the MSVC toolchain.

```powershell
rustup default stable-x86_64-pc-windows-msvc
cargo build --release
```

The output is `target\release\IndiCaps.exe`. Optional quality check:

```powershell
cargo clippy --release -- -D warnings
```

The release profile uses `opt-level = "z"`, LTO, `panic = "abort"`, stripped symbols and no incremental builds. `.cargo/config.toml` adds `/Brepro` on MSVC so identical source produces an identical EXE hash. `build.rs` calls the Windows SDK `rc.exe` to embed `assets/IndiCaps.ico` as the PE icon — no Rust crate involved. The project declares **no Rust crates**: only DLLs already present in Windows are called (`user32`, `gdi32`, `gdiplus`, `ole32`, `kernel32`, `shell32`, `advapi32`, `winmm`). Assets are included with `include_bytes!` and loaded from memory — no temporary files at runtime.

---

## Why it is so small

| Original .NET build | Rust build |
|---|---|
| ~68 MB compressed, self-contained runtime | One small zip, one self-contained EXE |
| WinForms + System.Drawing | Win32 + GDI/GDI+ directly |
| PNG, TTF and WAV loaded from `assets/` | Original assets embedded in the EXE |
| External ICO file | Official ICO embedded in the PE + logo drawn in the panel |
| Many timers and controls | One message window, layered overlay, panel created on demand |

---

## Project structure

```
Cargo.toml        optimized release profile, zero Rust dependencies
Cargo.lock
build.rs          embeds assets/IndiCaps.ico with the Windows SDK rc.exe
.cargo/config.toml  reproducible MSVC builds (/Brepro)
src/main.rs       app, keyboard hook, overlay, panel, tray, message loop
src/win.rs        minimal Win32 declarations (no windows crate)
src/draw.rs       32-bit DIB surface, blending, rounded corners
src/gfx.rs        GDI+ bridge: PNG, icon and Poppins font loaded from memory
src/settings.rs   JSON load/save, translations, registry, machine ID
assets/           official logo, ICO, original PNG/WAV/TTF assets
dist/             release binary and distributable zips (ignored by git)
```

Build output (`target/`, `dist/`, `bin/`, `obj/`) is **excluded from git** because it is regenerated on every build. The repository holds only source + assets; the compiled EXE is distributed through Releases.

---

## Credits

**IndiCaps** — designed and crafted by **Jovatis** · v2.0.0

Thanks for testing, reporting bugs, and starring the project if you like it.

<sub><sup>Shoutout to BigPickle</sup></sub>

---

## License note

IndiCaps is MIT-licensed (see `LICENSE`). The original assets (images, sounds) remain the property of their authors. The bundled font is **Poppins**, released under the SIL Open Font License — free to use and redistribute, including in commercial applications.
