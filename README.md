# Rusteau-1D

Rusteau-1D is a Rust port of the numerical engine of **HYDRUS-1D** (one-dimensional variably saturated water flow, heat
transport and solute transport in porous media) with a cross-platform desktop interface for Windows and Linux.

The engine follows the HYDRUS-1D Fortran source (version 4.08, 2009). Where HYDRUS 4.17 behaves differently from that
source, Rusteau follows 4.17. All 23 examples of the HYDRUS-1D distribution (`Examples/Direct`), recalculated with
HYDRUS 4.17, are part of the test suite.

> Rusteau-1D is an independent project. It is not affiliated with or endorsed by PC-Progress or the HYDRUS authors.
> "HYDRUS" is used here only to name the program this port follows.

## Contents

- [Status](#status)
- [Workspace](#workspace)
- [Building](#building)
- [Running](#running)
- [Interface](#interface)
- [Unit conversion](#unit-conversion)
- [Testing](#testing)
- [Differences to HYDRUS-1D](#differences-to-hydrus-1d)
- [Not supported](#not-supported)
- [Credits](#credits)
- [License](#license)

## Status

Verification against HYDRUS 4.17 (all columns of `T_Level.out`, `A_Level.out`, `Nod_Inf.out`, `Obs_Node.out`,
`Balance.out`, `Solute<i>.out`, `Meteo.out`, `Irrig.out`; tolerance = print precision of the reference files):

| Result | Examples |
|---|---|
| identical within print precision | 1SCALING, 2NOHYSTR, 3LAIJURI, 3SELIM, 4HEAT, 5SEASON, DRAINAGE, TEST3, TEST4, TEST5, TEST9a, Test9, Test10, UPINFIL |
| known differences (see [Known differences to HYDRUS 4.17](#known-differences-to-hydrus-417)) | 1DRAINAG, 1INFILTR, 2HYSTER, EnBal2b, ROOTUPTK, TEST1, TEST2, TEST11, VOLATILE |

Check runs made with HYDRUS 4.17 during the port (EnBal2b_A, EnBal2b_B and 5SEASON_IRR are part of the test set):

| Run | Purpose | Result |
|---|---|---|
| EnBal2b_A, EnBal2b_B | observation nodes at the surface, surface energy balance with vapor flow | B identical; A = EnBal2b |
| EnBal2b_DT | EnBal2b with dtMax = 0.002 | identical (6056 time levels in both) |
| 5SEASON_IRR | triggered irrigation | identical except 2 bookkeeping columns |
| 2HYSTER_P, _R2, _G, _D, _K | hysteresis variants (print times, no ponding, finer grid, longer drying step, KsW = KsD) | R2 identical; others: top-node K after ponding |
| 1DRAINAG, TEST1, 1INFILTR with TolTh = 1e-5, TolH = 0.01 | precision / convergence check | see known differences |

End-of-run water and solute totals agree with HYDRUS 4.17 within 0.07 % in all examples.

Not yet verified: the inverse module (parameter estimation), see [Not supported](#not-supported).

## Workspace

| Crate | Content |
|---|---|
| `crates/rusteau-core` | numerical engine: water flow (Richards equation, all HYDRUS-1D soil hydraulic models, hysteresis incl. Lenhard), root water and solute uptake, solute transport (equilibrium and non-equilibrium, chains, dual porosity / permeability), heat transport, atmospheric and meteorological boundary conditions (Penman-Monteith, Hargreaves, surface energy balance, vapor flow, snow, interception), triggered irrigation |
| `crates/rusteau-io` | readers for HYDRUS-1D projects (`SELECTOR.IN`, `PROFILE.DAT`, `ATMOSPH.IN`, `METEO.IN`, `Mater.in`, `MoistDep.in`, `.h1d`), native project files (`.h1dr`, JSON), result writers (CSV, `Meteo.out`), comparison with HYDRUS reference output |
| `crates/rusteau-cli` | command line program `rusteau` |
| `crates/rusteau-gui` | desktop interface `rusteau-gui` (egui / eframe) |

## Building

Requirements: Rust (stable, edition 2021; developed with Rust 1.98) and, on Linux, the X11 and OpenGL development
libraries for the interface (e.g. `libx11-dev libxcursor-dev libxrandr-dev libxi-dev libgl1-mesa-dev`). On Wayland the
interface runs through XWayland.

```sh
cargo build --release --workspace
```

The programs are `target/release/rusteau` and `target/release/rusteau-gui` (`.exe` on Windows).

## Running

```sh
rusteau <project-dir | name.h1d | project.h1dr> [options]
  -o, --out DIR        output directory (default: <project-dir>/rust_out, or <file>.results)
  -c, --compare [DIR]  compare all columns of the HYDRUS output files in DIR (default: the project directory); exit code 4 on differences
  -q, --quiet          no progress display
  -s, --summary        summary of the run
      --json FILE      complete results as JSON (readable by the interface)
```

The CLI accepts a HYDRUS-1D project folder (the folder with `SELECTOR.IN`), the HYDRUS-1D 4.x project file `name.h1d`
(the folder `name` next to it is read), or a native `.h1dr` project. It writes `T_LEVEL.csv`, `A_LEVEL.csv`,
`NOD_INF.csv`, `OBS_NODE.csv`, `BALANCE.csv` and, when produced, `T_LEVEL1.csv`, `METEO.csv`, `Meteo.out` and `IRRIG.csv`.

The interface (`rusteau-gui [project]`) opens `.h1dr` projects and imports HYDRUS-1D projects (`.h1d` or project
folder), edits all inputs, runs the simulation and plots the results.

## Interface

- **Workflow:** the input pages are grouped in the order of a project: 1 Project (processes & units, time & output),
  2 Soil (hydraulic model, materials & parameters, soil profile), 3 Boundary conditions (water flow, time-variable
  data, meteorological data), 4 Uptake & transport (root water uptake, heat, solute pages; shown when the process is
  switched on), 5 Run (iteration criteria, check & run), and Parameter estimation. A bar at the bottom leads to the
  previous and next step; the last step runs the calculation and opens the results.
- **Soil profile editor** (graphical view and nodal table on one page):
  - The selected quantity is drawn as a coloured column next to its profile: material, sub-region (mass balance),
    initial pressure head or water content, root distribution, scaling factors αh / αK / αθ, temperature,
    concentrations and non-equilibrium concentrations. Nodes, observation nodes and fixed points are marked.
  - Selection with the mouse (drag a depth range; click selects a node, or a whole layer for material and sub-region;
    Shift+click extends; Ctrl+A selects all) or by depth. Wheel scrolls, Ctrl+wheel zooms, double-click shows the
    whole profile.
  - Values in the selection: constant, linear between top and bottom, hydrostatic equilibrium from the lowest selected
    node (initial pressure head, with cos α), root distribution after Hoffman & van Genuchten (1983) or Vrugt et al.
    (2001), or the simulated values of the last run at any print time (initial h / θ, temperature, concentrations).
  - Observation nodes and fixed points are added and removed by clicking (or by depth).
  - Discretization: profile depth, number of nodes and fixed points with the relative element size above and below
    each point (fixed points are always nodes; they can be placed at all material boundaries). Generating the nodes
    interpolates all nodal values onto the new nodes; observation nodes keep their depth. The fixed points are read
    from and kept with the project (header of `PROFILE.DAT`).
- **Materials** are added (as a copy of the selected one, with all water, heat, solute and root parameters), removed
  and renamed on the page *Materials & parameters*.
- **Results:** observation points, profiles (several print times together, or one at a time with a time slider
  and playback on fixed axes), boundary fluxes and cumulative quantities, soil hydraulic properties (θ(h), K(h), K(θ),
  C(h); linear or logarithmic axes; main wetting curve with hysteresis; several materials together), run-time
  information and mass balance. All axes carry their units.
- **Measured data:** each results page loads text or CSV files with two columns (time and value, depth and value at a
  print time, or h and θ / K) and shows them as points with the simulated curves. Columns may be separated by blanks,
  tabs, commas or semicolons (with semicolons a decimal comma is accepted); lines without two numbers are skipped. The
  observations of a `Fit.in` in an imported project folder are loaded automatically.
- **Plots** are for inspection only; for figures use the CSV output (e.g. with ggplot2 or matplotlib).

## Unit conversion

When the length or time unit is changed on the page *Processes & units* with "Convert all values" switched on,
every input is rescaled, `value_new = value_old · fL^a · fT^b` with `fL`, `fT` the ratios of the units and `a`, `b`
the dimension of the quantity: heads, depths and lengths, conductivities and fluxes, `α` (1/L), times and print times,
atmospheric records, root uptake parameters, heat parameters, transport parameters (dispersivities, diffusion
coefficients, rate constants, Freundlich / Langmuir `Ks` and `η` with the exponent β), concentrations (M/L³), the
initial profile and the tables of tabular soil properties. Not converted: the mass unit (only a label), dimensionless
quantities, temperatures, activation energies and the meteorological inputs that HYDRUS reads in fixed units
(radiation, wind speed, sunshine hours, crop height and measurement heights, latitude, altitude, interception
constant); the time and rooting-depth columns of `METEO.IN` are converted. With the option switched off only the unit
label changes.

Checked by running every example of `Examples/Direct` in its own units and in converted units (cm/d → m/h,
cm/h → mm/min, cm/s → m/d) and comparing the rescaled results at all print times: identical within 1e-3, except
1DRAINAG (heads 0.3 %, the no-flux top noise above), 1INFILTR (chaotic onset of ponding) and the two vapor-flow
examples:

**Vapor flow depends on the length unit, as in HYDRUS.** The capacity term of the equivalent vapor content in
`VaporContent` (WATFLOW.FOR) is `ThetaV · M g / (R T) · xConv`. The derivative of the vapor content with respect to h
calls for `/ xConv` (h is converted to m inside the routine), so the term is right for m, 10⁴ times too large for cm and
10⁶ times too large for mm. Rusteau keeps the HYDRUS expression so that results match HYDRUS 4.17. Converting EnBal2b
from cm/d to m/h changes the cumulative top flux by 3.4 % and the surface temperature by up to 1 °C; TEST11 changes
the bottom flux by 0.7 %.

## Testing

```sh
cargo test --workspace
```

| Variable | Meaning |
|---|---|
| `RUSTEAU_EXAMPLES_DIR` | folder with the HYDRUS-1D examples and their HYDRUS output (default: `C:\Users\Public\Documents\PC-Progress\Hydrus-1D 4.xx\Examples\Direct`) |
| `RUSTEAU_COMPARE_ONLY` | comma-separated example names |
| `RUSTEAU_COMPARE_VERBOSE=1` | print every problem |
| `RUSTEAU_COMPARE_STRICT=1` | treat the known differences as failures too |
| `RUSTEAU_COMPARE_REPORT` | report file (default `reference_comparison_report.txt`) |
| `RUSTEAU_INVERSE_DIR`, `RUSTEAU_INVERSE_CASE` | inverse examples folder and a single case |

`test_all_direct_examples_comparison` runs every subfolder with a `SELECTOR.IN` and compares all reference files found
there. Differences listed in the table `KNOWN` of `crates/rusteau-io/tests/reference_comparison.rs` are accepted within
an envelope: at most 1.5 times the deviation and 1.25 times the number of differing values found on 9 Oct 2026. The
table covers the 23 examples of `Examples/Direct` and the check runs EnBal2b_A, EnBal2b_B and 5SEASON_IRR; any other
folder must match exactly. Any other difference, or a known one that grows, fails the test. A run that stops on non-convergence counts as known only
when the HYDRUS reference run stopped as well (`Error.msg` in the folder). The report lists both kinds.

The tests in `reference_comparison.rs` and `writer.rs` that use `crates/rusteau-io/tests/data/Meteo_sample.out` need
that file (a `Meteo.out` with 1217 time steps written by the Fortran code).

## Differences to HYDRUS-1D

### Precision

HYDRUS-1D stores pressure heads, water contents, conductivities and temperatures as REAL*4. Rusteau computes in double
precision throughout. The time step control emulates the single-precision `dt` of HYDRUS (`dt`, `dtOpt`, `dtMax`,
`dMul`, `dMul2` and `dtMin` rounded to f32, `t` in double precision), so the time levels match HYDRUS. Hydraulic
properties are interpolated from tables (`hTab1` to `hTabN`) as in HYDRUS.

### Deviations from the 2009 source that HYDRUS 4.17 confirms

1. **Initial `lMinStep = .false.`** (1SCALING: time levels 0.01, 0.023, 0.0399, …).
2. **Constant-flux top with infiltration** (`KodTop = -1`, vTop < 0) is booked in `sum(Infil)`.
3. **Sub-region water volume with dual porosity** contains the mobile water only.
4. **Heat top boundary condition**: no additional `min(0, HeatFl)` term.
5. **Heat flux at the top without energy balance**: `HeatFl = Cw (tTopA(t-dt) vOld(N) + tTopA(t) vNew(N)) / 2`, with the
   same weighting for the vapor part. The source uses `tTopA(t)` for both.
6. **Temperature adjustment at interior nodes**: `vVGrad = (vV(i+1) - vV(i-1)) / (x(i+1) - x(i-1))`, without the factor 2.
7. **Runoff only with a head top condition** (`KodTop > 0`). With a flux top, `rTop - vTop` is storage, not runoff.
8. **Relative solute balance error** of each solute is based on that solute's own mass (the source uses solute 1 for all).
9. **Surface energy balance with vapor flow**: the latent heat carried through the surface by the soil vapor flux is
   returned to the top node, `F(N) = F(N) - HeatFl + xLat(T(N)) (vVNew(N) + vVOld(N)) / 2`. Without it the surface
   is 0.034 °C too cold after the first step of EnBal2b, and 231 values differ.
10. Older corrections (interception with daily meteorological data, gas decay columns with dual non-equilibrium, rates
    of the second colloid sorption site, relative humidity input with energy balance, day number in `Evapor`, the
    Lenhard branch without a reversal path) are documented in the code.

Points where the port follows the source and 4.17 confirms it, although a different reading is easy:

- **Vapor volume:** the vapor volume is not part of the `T_Level` volume and the initial `Balance` volume.
- **Solute pulse:** a solute pulse ends at `tPulse` only without an atmospheric boundary condition.
- **Output lines:** `Run_Inf` and `Solute<i>` lines are written at every printed level.

### HYDRUS 4.17 features not in the 2009 source

- **Triggered irrigation** (`lIrrig`, the record `Node Pressure Irrig_rate Duration Lag_period` in `SELECTOR.IN`,
  `Irrig.out`).
  - **Trigger and timing:** an irrigation is triggered at the first time level with the pressure head at the observation
    node below the threshold. It starts after the lag period and lasts for the given duration, and start and end are
    time levels.
  - **4.17 behaviour:** the irrigation rate replaces the precipitation of the current record. An atmospheric record read
    during the irrigation ends it. The end of the irrigation removes the precipitation until the next record.
  - **Rusteau option:** `IrrigationMode::FullDuration` (in the interface "Behaviour: full duration") adds the rate to
    the precipitation for the whole duration, across records. HYDRUS 4.17 has no such mode, and HYDRUS projects are
    always read with the 4.17 behaviour.

### Rusteau only

- **Boundary-condition cycles** (`lBCCycles`) count a cycle from `t + 0.001 dt`, because the time reaches a record time
  only within the synchronisation tolerance (e.g. 5.9999999925 for 6).
- **Native project format** `.h1dr` (JSON), plus CSV output.

### Known differences to HYDRUS 4.17

These are the cases the test suite accepts (`Cause` in `reference_comparison.rs`). In each case Rusteau follows the
2009 source, or the cause is numerical.

| Cause | Examples | Size | Explanation |
|---|---|---|---|
| `RealPrecision` | TEST1, TEST2, TEST11, ROOTUPTK, VOLATILE | heads and fluxes ≤ 1.2 %, single values up to 4 % | Single precision in HYDRUS changes the iteration count near the TolTh / TolH limit, and with it the time steps. With 10× tighter tolerances TEST1 drops from 12 to 5 problems. |
| `NoFluxTopNoise` | 1DRAINAG | vTop where rTop = 0, early heads | HYDRUS 4.17 prints a top flux of up to 0.49 cm/h for a no-flux top, and its water balance error grows to 0.031 cm. With tighter tolerances it needs 47622 time steps. Rusteau keeps vTop = 0 (balance error 1e-13); the final outflow differs by 0.04 %. |
| `PondingChaos` | 1INFILTR | after t ≈ 0.86 h | With TolH = 1 cm the onset of ponding is chaotic. With tighter tolerances both programs stop on non-convergence at the same transition (0.329 / 0.331 h). |
| `HysteresisTopNode` | 2HYSTER | K of the top node 5 % (14 % with a 0.01 h first drying step) | After ponding, HYDRUS 4.17 starts the drying scanning curve of the top node from a lower conductivity at the reversal point than the 2009 source. The factor depends on the length of the first drying step and on the grid. The formulas in the manual (Eqs. 2.84–2.93) are those of the source. The rule 4.17 uses is unknown, so Rusteau keeps the source. Retention and all other nodes agree, and the run without ponding (2HYSTER_R2) is identical. |
| `LatentOscillation` | EnBal2b | 17 of 1623 `Meteo.out` latent heat values | At a dry surface at hCritA, vTop alternates every time step between evaporation and 0 in both programs. With dtMax = 0.002 the oscillation disappears and both agree exactly. |
| `IrrigationSign` | 5SEASON_IRR | `sum(Infil)`, `sum(Evap)` after t = 226 | vTop ≈ ±1e-6 is booked as infiltration or evaporation according to its sign. |

## Not supported

- **Inverse module** (parameter estimation, `FIT.IN`): in the code but not yet verified; this is the next step.
- **HP1** (PHREEQC coupling) and **UnsatChem**.
- **Restart from `Hysteresis.in`**: not read; scanning curves start from the initial condition.
- **`Options.in`** (optional HYDRUS settings file): not read.

## Credits

The numerical methods are those of HYDRUS-1D:

Šimůnek, J., M. Šejna, H. Saito, M. Sakai, and M. Th. van Genuchten (2013). *The HYDRUS-1D Software Package for
Simulating the One-Dimensional Movement of Water, Heat, and Multiple Solutes in Variably-Saturated Media*, Version 4.17.
Department of Environmental Sciences, University of California Riverside.
<https://www.pc-progress.com/en/Default.aspx?hydrus-1d>

## License

Rusteau-1D is free software: you can redistribute it and/or modify it under the terms of the GNU General Public License
as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version.
It is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of
MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See [LICENSE](LICENSE) for details.
