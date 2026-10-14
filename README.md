# Rusteau-1D

[![DOI](https://zenodo.org/badge/DOI/10.5281/zenodo.23343103.svg)](https://doi.org/10.5281/zenodo.23362225)

<img src="crates/rusteau-gui/assets/icon-512.png" alt="Rusteau-1D icon" width="96" align="right">

Rusteau-1D is a Rust port of the numerical engine of **HYDRUS-1D**, the program for one-dimensional water flow, heat
transport and solute transport in variably saturated soils. It comes with a desktop interface for Windows, macOS and
Linux and a command line program.

> Rusteau-1D is an independent project. It is not affiliated with or endorsed by PC-Progress or the HYDRUS authors.
> "HYDRUS" is used here only to name the program this port follows.

## TL;DR

- **Download** the programs for Windows, macOS or Linux from
  [Releases](https://github.com/muteNut/Rusteau-1D/releases), unpack the archive and start `rusteau` (on macOS
  `Rusteau-1D.app`). Nothing needs to be installed.
- **Compatible with HYDRUS-1D:** opens and saves HYDRUS-1D 4.x projects (`.h1d`); the input pages follow the dialogs
  of HYDRUS-1D 4.17.
- **Close to HYDRUS 4.17:** the 23 direct and 12 inverse examples of the HYDRUS-1D distribution are part of the test
  suite. 14 direct examples agree within the print precision of HYDRUS. The other 9 differ slightly, for the reasons
  listed [below](#results-that-differ-from-hydrus-417). Water and solute totals agree within 0.07 % in all of them.
- **Parameter estimation iterates differently by default.** It reaches the same or a lower objective function than
  HYDRUS 4.17, but can end at other parameter values. The iteration of HYDRUS 4.17 is available as an option.
- **Not supported:** HP1, UnsatChem and CO₂ transport.
- **Beyond HYDRUS-1D:** a figure editor for publication figures, comparison of runs, Rosetta 3, options of the HYDRUS
  code that HYDRUS-1D does not offer, and a help with the theory and 109 references.
- **License:** GPL-3.0 or later.

## Contents

- [Download and run](#download-and-run)
- [Building from source](#building-from-source)
- [Features](#features)
- [Command line](#command-line)
- [Differences to HYDRUS-1D](#differences-to-hydrus-1d)
- [Testing](#testing)
- [Documentation](#documentation)
- [Credits](#credits)
- [License](#license)

## Download and run

| Archive | System |
|---|---|
| `rusteau-windows-x86_64.zip` | Windows 10 or 11, 64-bit |
| `rusteau-macos-universal.zip` | macOS 11 or newer, Apple Silicon or Intel |
| `rusteau-linux-x86_64.tar.gz` | Linux x86-64 with glibc 2.39 or newer (Ubuntu 24.04, Debian 13, Fedora 40 or later) and an X11 desktop (Wayland through XWayland) |

Each archive contains the interface `rusteau` (on macOS the app `Rusteau-1D.app`), the command line program
`rusteau-cli`, the help as HTML pages (`help/index.html`) and the licenses of the bundled fonts (`fonts`).

- **Windows:** unpack and start `rusteau.exe`. The programs are not code-signed, so SmartScreen may warn the first
  time: *More info → Run anyway*.
- **macOS:** unpack and start `Rusteau-1D.app`. The app is not notarized by Apple, so macOS refuses to open it the
  first time: *System Settings → Privacy & Security → Open Anyway*, or in the Terminal
  `xattr -dr com.apple.quarantine Rusteau-1D.app`.
- **Linux:** `mkdir rusteau && tar xzf rusteau-linux-x86_64.tar.gz -C rusteau && ./rusteau/rusteau`

To start, open an example with *File → Examples*, or a HYDRUS-1D project with *File → Open project*. **F1** opens the
help of the current page.

## Building from source

Requirements: Rust (stable; developed with Rust 1.98) and, on Linux, the X11 and OpenGL development libraries, e.g. on
Ubuntu `sudo apt install libxkbcommon-dev libwayland-dev libx11-dev libxcb1-dev libgl1-mesa-dev`.

```sh
cargo build --release
```

The programs are `target/release/rusteau` (interface) and `target/release/rusteau-cli` (command line), with `.exe` on
Windows. On macOS (with the Xcode command line tools), `tools/package-macos.sh` builds the app bundle the releases
contain.

| Crate | Content |
|---|---|
| `crates/rusteau-core` | the numerical engine, parameter estimation and Rosetta |
| `crates/rusteau-io` | HYDRUS-1D project files, native project files (`.h1dr`, JSON), result writers, comparison with HYDRUS output |
| `crates/rusteau-cli` | the command line program `rusteau-cli` |
| `crates/rusteau-gui` | the interface `rusteau` (egui / eframe), figure editor and help |

## Features

**Simulation**

- Water flow (Richards equation) with all soil hydraulic models of HYDRUS-1D: van Genuchten–Mualem (also with an
  air-entry value of −2 cm), modified van Genuchten, Brooks–Corey, Kosugi, Durner, mobile–immobile water, dual
  permeability and look-up tables. Hysteresis, including the Lenhard model.
- Root water uptake and root solute uptake, including active uptake (Michaelis–Menten).
- Solute transport: equilibrium and non-equilibrium, decay chains, dual porosity and dual permeability.
- Heat transport.
- Atmospheric and meteorological boundary conditions: Penman–Monteith, Hargreaves, surface energy balance with vapor
  flow, snow, interception, triggered irrigation.
- Parameter estimation (inverse problems, `Fit.in` / `Fit.out`).
- Rosetta pedotransfer functions: Rosetta 3, and the Rosetta 1 models of HYDRUS' Rosetta Lite.

**Interface**

- Input pages in the order of a project, with a bar that leads to the next step and finally runs the calculation.
- A graphical soil profile editor for materials, initial conditions, root distribution, scaling factors and
  observation points.
- Results pages: observation points, profiles at the print times (with playback), boundary fluxes, soil hydraulic
  properties, run-time information and mass balance. Measured data can be shown with the results.
- Parameter estimation with a live iteration table, confidence limits, correlation matrix and plots of the fit.
- Unit conversion of all inputs, a recovery copy of unsaved work, and checks of the inputs before a run.

**Figure editor**

- Figures for papers, reports and presentations, starting from templates.
- Up to 4 × 4 panels with layers of results, input, measured data, parameter estimation results or imported tables.
  Kept runs can be compared in one figure.
- Themes, colour-blind safe palettes or custom colours, renaming of series in the legend, logarithmic and reversed
  axes, a second y axis, annotations.
- Sub- and superscripts, Greek letters and mathematical signs in all texts (Noto Sans is bundled).
- Export as PNG (with its resolution), SVG, or to the clipboard.

**Help**

- A tooltip on every input (about 580): meaning, unit, typical values and the HYDRUS variable.
- A help window with about 70 topics, a keyword index and full-text search. It describes every page and the theory
  with its equations, and cites 109 publications.
- The same help as HTML pages: [`docs/help`](docs/help/index.html), and in the release archives.

## Command line

```
rusteau-cli <project-dir | name.h1d | project.h1dr> [options]
  -o, --out DIR        output directory (default: <project-dir>/rust_out, or <file>.results)
  -c, --compare [DIR]  compare all columns with the HYDRUS output files in DIR (default: the project directory);
                       exit code 4 on differences
  -q, --quiet          no progress display
  -s, --summary        summary of the run
      --json FILE      complete results as JSON (readable by the interface)
      --direct         run an inverse project as a direct problem with the initial estimates
      --hydrus-stop    inverse problems: iterate exactly as HYDRUS 4.17 does
      --starts N       inverse problems: number of additional starting points (default 8)
      --save-h1d FILE  write the project as a HYDRUS-1D 4.x project (FILE.h1d + folder FILE) and exit
```

The results are written as CSV: `T_LEVEL.csv`, `A_LEVEL.csv`, `NOD_INF.csv`, `OBS_NODE.csv`, `BALANCE.csv`, and when
produced `T_LEVEL1.csv`, `METEO.csv`, `Meteo.out`, `IRRIG.csv`. Inverse projects also write `Fit.out`. The interface
writes the HTML help with `rusteau --export-help DIR`.

## Differences to HYDRUS-1D

Rusteau follows the HYDRUS-1D Fortran source (version 4.08, 2009). Where HYDRUS 4.17 behaves differently from that
source, Rusteau follows 4.17 (nine corrections, e.g. in the heat flux at the top and the solute balance). The full
list, with the code involved, is in the [technical notes](docs/technical-notes.md#differences-to-hydrus-1d).

### Results that differ from HYDRUS 4.17

| Examples | What differs | Why |
|---|---|---|
| TEST1, TEST2, TEST11, ROOTUPTK, VOLATILE | heads and fluxes up to 1.2 %, single values up to 4 % | HYDRUS computes in single precision. Near the tolerance limits its iteration counts change, and with them the time steps. On the time levels of 4.17, TEST1 agrees exactly. |
| 1DRAINAG | top flux and early heads | HYDRUS 4.17 reports a top flux of up to 0.49 cm/h at a no-flux top (and a balance error of 0.031 cm); Rusteau keeps it at 0. The final outflow differs by 0.04 %. |
| 1INFILTR | results after t ≈ 0.86 h | With the coarse head tolerance of the example (TolH = 1 cm), the onset of ponding is chaotic. |
| 2HYSTER | conductivity of the top node, 5 % | After ponding, 4.17 starts the drying curve of the top node from a lower conductivity than its own source and manual. The rule is unknown, so Rusteau keeps the source. |
| EnBal2b | 17 of 1623 latent heat values | A dry surface alternates between evaporation and zero flux in both programs. With a smaller maximum time step both agree. |

### Other differences

- **Precision:** Rusteau computes in double precision; HYDRUS stores heads, water contents, conductivities and
  temperatures in single precision. The time step control emulates that of HYDRUS, so the time levels match.
- **Parameter estimation:** the 2009 source contains no inverse code. Rusteau reconstructs it from the manual and the
  output of 4.17; the objective function, the weighting, the statistics and the first iteration reproduce 4.17. By
  default Rusteau then iterates on frozen time levels from 8 additional starting points. In all examples this ends at
  the same or a lower objective than 4.17, but it can find another minimum: in HORIZINF n ends at 13.5 instead of
  2.76, because `Fit.in` gives n no upper limit. Set constraints that bound the physically sensible range. The
  iteration of HYDRUS 4.17 is available (`--hydrus-stop`, or *Iteration: HYDRUS 4.17* in the interface).
- **Vapor flow depends on the length unit, as in HYDRUS.** A term of the vapor content in the HYDRUS code is only
  correct in metres. Rusteau keeps it so that its results match HYDRUS. Converting the example EnBal2b from cm/d to
  m/h changes the cumulative top flux by 3.4 %.
- **Saving as a HYDRUS-1D project drops settings HYDRUS cannot store;** a note lists them when saving. These are the
  additions below, the interface parameters of the dual-permeability model, flux-averaged output concentrations, the
  trapezoidal water-content dependence of reactions, the extra flux output, the analytic evaluation of the hydraulic
  functions, the Rusteau iteration of the parameter estimation, and material and solute names. Figures are kept in
  `.h1dr` projects only.
- **Restart with hysteresis:** when HYDRUS reads the hysteresis state (`Hysteresis.in`), the drying curves of the
  conductivity vanish. Rusteau uses the main curve of the conductivity instead; its own restarts also keep the
  conductivities.

### Additions not in HYDRUS-1D

- Options of the HYDRUS code that HYDRUS-1D never offered (*Run → Further options*): non-equilibrium water flow,
  centrifuge, density-dependent flow, the enhancement factor of thermal vapor flow and the extrapolation of the heads.
  The first three are not available with dual permeability.
- Rosetta 3, besides the Rosetta 1 models of Rosetta Lite.
- Triggered irrigation for the whole duration across records (HYDRUS 4.17 stops it at the next atmospheric record).
- Native project files (`.h1dr`), CSV output, measured data shown with the results, comparison of runs, the figure
  editor.

### Not supported

- HP1 (coupling with PHREEQC), UnsatChem (major-ion chemistry) and CO₂ transport. They are separate modules beside
  the HYDRUS-1D core and not part of the source this port follows.

## Testing

```sh
cargo test --release --workspace
```

The comparison with HYDRUS 4.17 needs the HYDRUS-1D examples with their 4.17 output: set `RUSTEAU_EXAMPLES_DIR` to
`Examples/Direct` and `RUSTEAU_INVERSE_DIR` to `Examples/Inverse` (the defaults are the folders of a HYDRUS-1D
installation on Windows). Without them the inverse comparison is skipped and `test_all_direct_examples_comparison`
fails; add `-- --skip test_all_direct_examples_comparison` to run everything else. The other variables, the accepted
differences and the check runs are described in the [technical notes](docs/technical-notes.md#testing).

## Documentation

- **Help:** in the program (**F1**, *Help* menu), and as HTML pages in [`docs/help`](docs/help/index.html). The
  topic *Relation to HYDRUS-1D* summarizes compatibility and verification.
- **Technical notes:** [`docs/technical-notes.md`](docs/technical-notes.md): the verification in detail, the
  parameter estimation algorithm, unit conversion, HYDRUS-1D files, restarts with hysteresis, the further options,
  Rosetta, the test setup, and how the help is maintained.
- **Release notes:** [`RELEASE_NOTES.md`](RELEASE_NOTES.md).

## Credits

To cite Rusteau-1D, use its DOI [10.5281/zenodo.23343103](https://doi.org/10.5281/zenodo.23343103), archived on
Zenodo; *Cite this repository* on the GitHub page gives the reference from [`CITATION.cff`](CITATION.cff).

The numerical methods are those of HYDRUS-1D:

Šimůnek, J., M. Šejna, H. Saito, M. Sakai, and M. Th. van Genuchten (2013). *The HYDRUS-1D Software Package for
Simulating the One-Dimensional Movement of Water, Heat, and Multiple Solutes in Variably-Saturated Media*, Version 4.17.
Department of Environmental Sciences, University of California Riverside.
<https://www.pc-progress.com/en/Default.aspx?hydrus-1d>

The Rosetta network weights come from rosetta-soil (USDA-ARS U.S. Salinity Laboratory, CC0 1.0); the models are those
of Schaap et al. (2001, 2004) and Zhang & Schaap (2017).

The interface bundles the fonts Noto Sans 2.015 and Noto Sans Math 3.000 (© The Noto Project Authors), unmodified,
under the SIL Open Font License 1.1. The license texts are in [`crates/rusteau-gui/fonts`](crates/rusteau-gui/fonts),
in the folder `fonts` of the release archives and under *Help → About*. Figures are drawn with tiny-skia
(BSD-3-Clause) and ab_glyph / ttf-parser (Apache-2.0 / MIT), and written with png (MIT / Apache-2.0).

## License

Rusteau-1D is free software: you can redistribute it and/or modify it under the terms of the GNU General Public License
as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version.
It is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of
MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See [LICENSE](LICENSE) for details.
