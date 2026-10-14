Rusteau-1D simulates one-dimensional water flow, heat transport and solute transport in variably saturated soils: a Rust
port of the HYDRUS-1D engine with a desktop interface for Windows, macOS and Linux. Documentation: the
[website](https://mutenut.github.io/Rusteau-1D/) and the [README](https://github.com/muteNut/Rusteau-1D#readme).

## What's new in 1.2

- **File dialogs of the operating system:** Open and Save use the dialogs of Windows and macOS, and on Linux the dialog
  of the desktop (through the XDG desktop portal, or zenity). Without either, the built-in file browser is used; it
  can also be chosen with `RUSTEAU_FILE_DIALOG=builtin`.
- **Root growth only with root water uptake:** the option is shown under root water uptake and is switched off with
  it, as in HYDRUS-1D. Projects with root growth but without root water uptake are calculated without root growth.
- **Figures:** the last tick label of an x axis is no longer cut off at the right edge of the figure.
- **Version:** the programs report their version correctly (*Help → About*, `rusteau-cli --version`); 1.0 and 1.1
  reported 1.0.0.

## Download

| File | System |
|---|---|
| `rusteau-windows-x86_64.zip` | Windows 10 or 11, 64-bit |
| `rusteau-macos-universal.zip` | macOS 11 or newer, Apple Silicon or Intel |
| `rusteau-linux-x86_64.tar.gz` | Linux x86-64 with glibc 2.39 or newer (Ubuntu 24.04, Debian 13, Fedora 40 or later) |

Unpack the archive and start `rusteau` (`rusteau.exe`; on macOS `Rusteau-1D.app`); nothing needs to be installed. The
programs are not code-signed: Windows SmartScreen may warn the first time (*More info → Run anyway*), and on macOS
*System Settings → Privacy & Security → Open Anyway* allows the app.

Rusteau-1D is free software (GPL-3.0 or later), an independent project not affiliated with PC-Progress or the HYDRUS
authors.
