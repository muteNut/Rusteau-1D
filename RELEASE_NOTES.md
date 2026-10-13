Rusteau-1D is a Rust port of the numerical engine of **HYDRUS-1D**: one-dimensional variably saturated water flow, heat
transport and solute transport in porous media. It comes with a desktop interface for Windows, macOS and Linux, a figure
editor for publication figures and a complete help. Version 1.0.0 is the first release.

> Rusteau-1D is an independent project. It is not affiliated with or endorsed by PC-Progress or the HYDRUS authors.

## Download

| File | System |
|---|---|
| `rusteau-windows-x86_64.zip` | Windows 10 or 11, 64-bit |
| `rusteau-macos-universal.zip` | macOS 11 or newer, Apple Silicon or Intel |
| `rusteau-linux-x86_64.tar.gz` | Linux x86-64 with glibc 2.39 or newer (Ubuntu 24.04, Debian 13, Fedora 40 or later) and an X11 desktop (Wayland through XWayland) |

Each archive contains:

- `rusteau` (`rusteau.exe`; on macOS the app `Rusteau-1D.app`): the interface
- `rusteau-cli` (`rusteau-cli.exe`): the command line program
- `help`: the help as HTML pages (open `help/index.html`)
- `fonts`: the licenses of the bundled fonts

Nothing needs to be installed:

- **Windows:** unpack the archive and start `rusteau.exe`. The programs are not code-signed, so Windows SmartScreen
  may warn when they are started the first time: *More info → Run anyway*.
- **macOS:** unpack the archive and start `Rusteau-1D.app` (it can be moved to *Applications*). The app is not
  notarized by Apple, so macOS refuses to open it the first time: open *System Settings → Privacy & Security* and
  click *Open Anyway*. Or remove the quarantine flag in the Terminal: `xattr -dr com.apple.quarantine Rusteau-1D.app`
  (and the same for `rusteau-cli`).
- **Linux:** `mkdir rusteau && tar xzf rusteau-linux-x86_64.tar.gz -C rusteau && ./rusteau/rusteau`

To start, open an example with *File → Examples*, or a HYDRUS-1D project (`.h1d`) with *File → Open project*. **F1**
opens the help of the current page.

## Highlights

### Simulation

- Water flow (Richards equation) with all soil hydraulic models of HYDRUS-1D: van Genuchten–Mualem (also with an
  air-entry value of −2 cm), modified van Genuchten, Brooks–Corey, Kosugi, Durner, mobile–immobile water,
  dual permeability and look-up tables. Hysteresis, including the Lenhard model.
- Root water uptake and root solute uptake: Feddes or S-shape stress models, osmotic stress, active uptake
  (Michaelis–Menten).
- Solute transport: equilibrium and non-equilibrium, decay chains, dual porosity and dual permeability.
- Heat transport.
- Atmospheric and meteorological boundary conditions: Penman–Monteith, Hargreaves, surface energy balance with vapor
  flow, snow, interception, triggered irrigation.
- Parameter estimation (inverse problems, `Fit.in` / `Fit.out`): objective function, weighting of the measurement sets
  and statistics as in HYDRUS 4.17. Rusteau runs a more robust iteration by default (frozen time levels, several
  starting points in parallel); `--hydrus-stop` iterates exactly as HYDRUS 4.17.
- Rosetta pedotransfer functions: Rosetta 3, and the Rosetta 1 models of HYDRUS' Rosetta Lite.

### Checked against HYDRUS 4.17

- All 23 direct and all 12 inverse examples of the HYDRUS-1D distribution are part of the test suite.
- 14 of the direct examples agree with HYDRUS 4.17 within the print precision of its output files. The differences
  in the others are documented and explained in the README (*Differences to HYDRUS-1D*).
- End-of-run water and solute totals agree within 0.07 % in all examples.
- Parameter estimation ends at the same or a lower objective function than HYDRUS 4.17 in all inverse examples.

### Projects and files

- HYDRUS-1D 4.x projects (`name.h1d` and the folder `name`) are read and written, so a project made in Rusteau opens
  in HYDRUS-1D.
- Rusteau's own project files (`.h1dr`) also keep the figures.
- Results are written as CSV (`T_LEVEL`, `A_LEVEL`, `NOD_INF`, `OBS_NODE`, `BALANCE`, …), plus `Fit.out` and
  `Meteo.out`.

### Interface

- The input pages follow the order of a project. A bar at the bottom of each page leads to the next step and finally
  runs the calculation.
- Graphical soil profile editor: materials, initial conditions, root distribution and scaling factors; observation
  points can be dragged and edited.
- Results pages: observation points, profiles at the print times (with playback), boundary fluxes, soil hydraulic
  properties, run-time information and mass balance. Measured data can be loaded and shown with the results.
- Parameter estimation pages with a live iteration table, confidence limits, correlation matrix and plots of the fit.
- When the length or time unit is changed, all inputs can be converted to the new units.
- A recovery copy protects unsaved work, and errors are caught instead of closing the program.

### Figure editor

- Figures for papers, reports and presentations, with ready-made templates for the usual diagrams.
- Up to 4 × 4 panels. Each panel shows layers of data: results, input, measured data, parameter estimation results
  or imported tables. Kept runs can be compared in one figure.
- Themes; colour-blind safe palettes or custom colours with a colour picker, for the whole figure or per panel.
  Series can be renamed in the legend, whose position and width can be set.
- Linear, logarithmic and reversed axes, a second y axis, and text, line and arrow annotations.
- Sub- and superscripts, Greek letters and mathematical signs in all texts. Noto Sans is bundled, so all the
  symbols of soil physics can be drawn.
- Export as PNG (the resolution is written into the file) or SVG, or copy to the clipboard.

### Help

- A tooltip on every input, about 580 in all: meaning, unit, typical values and the HYDRUS variable.
- A help window with about 70 topics, a keyword index and full-text search. It describes every page and the theory
  with its equations, and cites 109 publications with their DOIs.
- The same help as HTML pages, in the folder `help` of the archives.

## Command line

```
rusteau-cli MyProject.h1d -o out                       # run a HYDRUS-1D project, results as CSV in out/
rusteau-cli MyProject -c                               # compare with the HYDRUS output files in the project folder
rusteau-cli project.h1dr --save-h1d C:\Hydrus\Copy.h1d # write a Rusteau project as a HYDRUS-1D project
```

`rusteau-cli --help` lists all options; the help topic *Command line* describes them.

## Not included

- HP1 (coupling with PHREEQC) and UnsatChem are not supported.

## License

Rusteau-1D is free software under the GNU General Public License, version 3 or later. The numerical methods are those
of HYDRUS-1D (Šimůnek et al., version 4.17, University of California Riverside). The bundled fonts Noto Sans and
Noto Sans Math (© The Noto Project Authors) are licensed under the SIL Open Font License 1.1; the license texts are in
the folder `fonts` of the archives.
