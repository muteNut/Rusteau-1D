# Rusteau-1D

Rusteau-1D is a Rust port of the numerical engine of **HYDRUS-1D** (one-dimensional variably saturated water flow, heat
transport and solute transport in porous media) with a cross-platform desktop interface for Windows and Linux.

The engine follows the HYDRUS-1D Fortran source (version 4.08, 2009). Where HYDRUS 4.17 behaves differently from that
source, Rusteau follows 4.17. All 23 examples of the HYDRUS-1D distribution (`Examples/Direct`) and the 12 parameter
estimation examples (`Examples/Inverse`), recalculated with HYDRUS 4.17, are part of the test suite.

Projects are read from and written to HYDRUS-1D 4.x project folders (`name.h1d` + folder `name`), so a project made
in Rusteau opens in HYDRUS-1D. Soil hydraulic parameters can be predicted from the texture with the Rosetta
pedotransfer functions (Rosetta 3 and the Rosetta 1 models of HYDRUS' Rosetta Lite).

> Rusteau-1D is an independent project. It is not affiliated with or endorsed by PC-Progress or the HYDRUS authors.
> "HYDRUS" is used here only to name the program this port follows.

## Contents

- [Status](#status)
- [Workspace](#workspace)
- [Building](#building)
- [Running](#running)
- [Interface](#interface)
- [Unit conversion](#unit-conversion)
- [Parameter estimation](#parameter-estimation)
- [HYDRUS-1D project files](#hydrus-1d-project-files)
- [Restart with hysteresis (Hysteresis.in)](#restart-with-hysteresis-hysteresisin)
- [Further options of the HYDRUS code (Options.in)](#further-options-of-the-hydrus-code-optionsin)
- [Pedotransfer functions (Rosetta)](#pedotransfer-functions-rosetta)
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

Check runs made with HYDRUS 4.17 during the port (EnBal2b_A, EnBal2b_B, 5SEASON_IRR and the five model checks are part
of the test set):

| Run | Purpose | Result |
|---|---|---|
| EnBal2b_A, EnBal2b_B | observation nodes at the surface, surface energy balance with vapor flow | B identical; A = EnBal2b |
| EnBal2b_DT | EnBal2b with dtMax = 0.002 | identical (6056 time levels in both) |
| 5SEASON_IRR | triggered irrigation | identical except 2 bookkeeping columns |
| 2HYSTER_P, _R2, _G, _D, _K | hysteresis variants (print times, no ponding, finer grid, longer drying step, KsW = KsD) | R2 identical; others: top-node K after ponding |
| 1DRAINAG, TEST1, 1INFILTR with TolTh = 1e-5, TolH = 0.01 | precision / convergence check | see known differences |
| 2NOHYSTR_DUR, 2NOHYSTR_AE, 1SCALING_BC | Durner dual-porosity, van Genuchten with −2 cm air entry, Brooks–Corey | identical |
| 2NOHYSTR_LN, 2HYSTER_RET | Kosugi log-normal; hysteresis in the retention curve only | identical except the first steps after the end of ponding (within the iteration tolerance) |
| ROOTUPTK_ACT, ROOTUPTK_ACT2 | active solute uptake: one solute; the second of two solutes | identical except the single-precision differences of ROOTUPTK (cRoot at two times, one node of the concentration front) |

End-of-run water and solute totals agree with HYDRUS 4.17 within 0.07 % in all examples.

Parameter estimation (inverse problem, `Fit.in` / `Fit.out`): the objective function, the weighting of the measurement
sets, the statistics and the first Marquardt iteration reproduce HYDRUS 4.17. The Rusteau iteration (frozen time
levels, several starting points) ends at the same or a lower objective than 4.17 in all examples, see
[Parameter estimation](#parameter-estimation).

## Workspace

| Crate | Content |
|---|---|
| `crates/rusteau-core` | numerical engine: water flow (Richards equation, all HYDRUS-1D soil hydraulic models, hysteresis incl. Lenhard), root water and solute uptake, solute transport (equilibrium and non-equilibrium, chains, dual porosity / permeability), heat transport, atmospheric and meteorological boundary conditions (Penman-Monteith, Hargreaves, surface energy balance, vapor flow, snow, interception), triggered irrigation, parameter estimation, Rosetta pedotransfer functions |
| `crates/rusteau-io` | readers and writers for HYDRUS-1D projects (`SELECTOR.IN`, `PROFILE.DAT`, `ATMOSPH.IN`, `METEO.IN`, `Mater.in`, `MoistDep.in`, `Fit.in`, `HYDRUS1D.DAT`, `DESCRIPT.TXT`, `.h1d`), native project files (`.h1dr`, JSON), result writers (CSV, `Meteo.out`, `Fit.out`), comparison with HYDRUS reference output |
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
      --direct         run an inverse project as a direct problem with the initial estimates
      --hydrus-stop    inverse problems: iterate exactly as HYDRUS 4.17 does (one starting point, adaptive time
                       steps in every run)
      --starts N       inverse problems: number of additional starting points (default 8)
      --save-h1d FILE  write the project as a HYDRUS-1D 4.x project (FILE.h1d + folder FILE) and exit
```

`rusteau project.h1dr --save-h1d C:\Hydrus\MyProject.h1d` converts a Rusteau project for HYDRUS-1D;
`rusteau MyProject.h1d -o out` runs a HYDRUS-1D project.

A project with an inverse problem (`lInverse` in `SELECTOR.IN` with a `Fit.in`, or the inverse settings of a `.h1dr`)
is optimized: the iterations are printed, and `Fit.out` and the results of the final run with the fitted parameters are
written. With `--compare` the fit is compared with the `Fit.out` in the folder (exit code 4 when the objective function
is more than 5 % above the reference).

The CLI accepts a HYDRUS-1D project folder (the folder with `SELECTOR.IN`), the HYDRUS-1D 4.x project file `name.h1d`
(the folder `name` next to it is read), or a native `.h1dr` project. It writes `T_LEVEL.csv`, `A_LEVEL.csv`,
`NOD_INF.csv`, `OBS_NODE.csv`, `BALANCE.csv` and, when produced, `T_LEVEL1.csv`, `METEO.csv`, `Meteo.out` and `IRRIG.csv`.

The interface (`rusteau-gui [project]`) opens `.h1dr` projects and imports HYDRUS-1D projects (`.h1d` or project
folder), edits all inputs, runs the simulation and plots the results. *File → Save as HYDRUS-1D project* writes
`name.h1d` and the folder `name`; with *Save also writes a HYDRUS-1D project* every Save writes them next to the
`.h1dr`.

## Interface

- **Workflow:** the input pages are grouped in the order of a project: 1 Project (processes & units, time & output),
  2 Soil (hydraulic model, materials & parameters, soil profile), 3 Boundary conditions (water flow, time-variable
  data, meteorological data), 4 Uptake & transport (root water uptake, heat, solute pages; shown when the process is
  switched on), 5 Run (iteration criteria, further options, check & run), and Parameter estimation. A bar at the bottom leads to the
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
    With hysteresis, *Restart from these results* takes the heads and the hysteresis state of all nodes, see
    [Restart with hysteresis](#restart-with-hysteresis-hysteresisin).
  - Observation nodes and fixed points are added and removed by clicking (or by depth).
  - Discretization: profile depth, number of nodes and fixed points with the relative element size above and below
    each point (fixed points are always nodes; they can be placed at all material boundaries). Generating the nodes
    interpolates all nodal values onto the new nodes; observation nodes keep their depth. The fixed points are read
    from and kept with the project (header of `PROFILE.DAT`).
- **Soil hydraulic model** follows the *Soil Hydraulic Model* dialog of HYDRUS 4.17: single-porosity models (van
  Genuchten–Mualem with the option *air-entry value of −2 cm*, modified van Genuchten, Brooks–Corey, Kosugi),
  dual-porosity models (Durner; mobile–immobile with water-content or head mass transfer), dual permeability, look-up
  tables, and the hysteresis models with the initial branch.
- **Root water and solute uptake** follows the *Root Water and Solute Uptake Model* dialog of HYDRUS 4.17: water uptake
  reduction model (Feddes / S-shape) with its parameters (POpt per material), solute stress model (none, additive,
  multiplicative with threshold or S-shape) with the osmotic coefficients, critical stress index for water uptake,
  maximum concentration for passive uptake per solute, and active solute uptake (Michaelis-Menten) with the solute that
  is taken up actively, the potential uptake rate, the Michaelis-Menten constant, the minimum concentration, the
  critical stress index for active uptake and the reduction of the potential uptake with reduced water uptake.
- **Materials** are added (as a copy of the selected one, with all water, heat, solute and root parameters), removed
  and renamed on the page *Materials & parameters*. The hydraulic parameters can be loaded from the soil catalog
  (Carsel & Parrish 1988) or predicted with Rosetta (*Neural network prediction*, see
  [Pedotransfer functions](#pedotransfer-functions-rosetta)).
- **Results:** observation points, profiles (several print times together, or one at a time with a time slider
  and playback on fixed axes), boundary fluxes and cumulative quantities, soil hydraulic properties (θ(h), K(h), K(θ),
  C(h); linear or logarithmic axes; main wetting curve with hysteresis; several materials together), run-time
  information and mass balance. All axes carry their units.
- **Parameter estimation** (page *Inverse problem* under Parameter estimation, results page *Parameter estimation*):
  - Settings of `Fit.in`, grouped as in the *Inverse Solution* dialog of HYDRUS 4.17: the parameter groups (soil
    hydraulic; solute transport with the concentration used in the objective function, or heat transport: as in 4.17
    the two exclude each other), the hysteresis constraints, the weighting of the measurement sets, the maximum number
    of iterations and the iteration (Rusteau: frozen time levels and further starting points, the default; or exactly
    the iteration of HYDRUS 4.17, see [Parameter estimation](#parameter-estimation)).
  - Parameter tables per material and group as in the HYDRUS parameter dialogs (initial estimate, minimum, maximum,
    fitted; 0 = no constraint); initial estimates of the soil hydraulic parameters from the soil catalog (Carsel &
    Parrish 1988) or from Rosetta.
  - Observation table with all HYDRUS observation types (types 0-15; type 4 holds temperatures when heat parameters are
    fitted), number and weight of every measurement set; observations are added, removed or pasted from the
    clipboard (lines `x value type position weight` as in `Fit.in`).
  - `Fit.in` import and export; settings can be created from the current project.
  - The run shows the iteration table of the first starting point live, and the end of the others; it can be stopped.
    Results: the starting points with their final objective, fitted parameters with standard errors and 95 %
    confidence limits, goodness of fit (R², ME, MAE, RMSE, AIC, BIC, contribution of every measurement set),
    correlation matrix (|r| > 0.95 highlighted), fitted against observed values or observed and fitted values against
    time / pressure head, iteration table, `Fit.out`. The fitted parameters can be written into the project; the
    results of the final run fill the other results pages.
  - The examples *Parameter estimation: one-step outflow* and *Parameter estimation: soil temperature* (File →
    Examples) estimate α, n and Ks, and the heat parameters b1 and Cn, from synthetic data.
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

## Parameter estimation

The 2009 Fortran source contains no inverse code. The algorithm of HYDRUS 4.17 was reconstructed from the manual
(chapter 9, Table 9.1 and the `Fit.in` description), from the `Fit.out` files of the 12 inverse examples and from
4.17 check runs (weighting by mean ratio, heat parameters, perturbed initial values); every rule below reproduces
numbers printed by 4.17.

- **Model values:** the observation times are fixed times of the time step control, like print times (the final runs
  of 4.17 have time levels at all of them), and the values are taken at these time levels. Types 0-4 (cumulative and
  actual boundary fluxes, h, θ and concentrations at observation nodes, mean water content of the profile or of a
  sub-region for position 0 / −k, total solute mass in the profile for type 4 with position 0), 5 and 6 (θ(h), K(h)
  of a material), 7-11 (prior information on α, n, θr, θs, Ks) and 12-15 (h, θ, c and sorbed c at a depth at a print
  time) are evaluated.
- **Heat transport parameters** (`lTempF`, HYDRUS 4.17): `Fit.in` has `lTempF` as a fourth value after `lWatF lChemF
  NMat`, and after the soil hydraulic records one record per material with `Qn Qo Disper. B1 B2 B3 Cn Co Cw` (values,
  fit flags, minima, maxima). Temperatures at observation nodes are observations of type 4: 4.17 offers solute or heat
  parameters, not both, and its data list has no temperature type. `Fit.out` lists the heat parameters per material
  (*Heat transport properties*).
- **Objective function:** `SSQ = Σ (W_i (FO_i − F_i))²` with `W_i = w_i v_j`. The measurement sets are the
  observation types (all positions together); retention points (type 5) belong to the set of the water contents
  (type 2). Weighting of set `j`: none (`v_j` = 1); by mean ratio, `v_j` = smallest mean of the absolute values of all
  sets / mean of set `j`, also for a set of one value (EVAPOR and the check runs FITTEST2_W1, UPINFIL_W1,
  MULTSTEP_W1B); or by standard deviation, `v_j = 1 / (√n_j σ_j)` with the population standard deviation of the
  absolute values (Eq. 9.2 of the manual applied to the residual, not to its square), where a set of one value or
  without scatter gets `v_j` = 1. The products `W_i` are the "Weight" column of `Fit.out`.
- **Marquardt-Levenberg iteration:** forward-difference derivatives with an increment of 1 % of each parameter, normal
  equations scaled to a unit diagonal, Marquardt parameter 0.02 divided by 10 at the start of every iteration and
  multiplied by 10 after a failed step (a step that points within 30° of the gradient is halved instead). Parameters
  are clamped to their constraints (a constraint of 0 means none); a step that changes the sign of a parameter fails.
  The iteration stops when no parameter changes by more than 1 % (judged on the step before the constraints), after
  MIT iterations, or with "No further reduction in SSQ obtained". Example: in FITTEST5 4.17 sets ω to exactly 0.1 in
  iteration 2; that is the minimum constraint of ω in `Fit.in` (Rusteau does the same).
- **Rusteau iteration (the default; not in 4.17).** With the adaptive time steps of every run the objective function
  is not smooth: a small change of a parameter changes the time steps, and the objective jumps by up to a few per cent
  of its value (UPINFIL: ±3 % for changes of l of 0.2 %). The 1 % derivatives then carry large errors, and in a narrow
  curved valley the iteration of HYDRUS stops far from the minimum, at a point that depends on the last bits of the
  arithmetic (see *Path dependence* below). Rusteau therefore
  - runs the iteration in **stages on frozen time levels**: within a stage every run (derivatives and trial steps)
    uses the time levels of the run at the start of the stage, so the objective is a smooth function of the
    parameters, and the derivatives use increments of 0.1 % (a parameter without effect at 0.1 %, e.g. β next to the
    linear isotherm threshold |β − 1| ≤ 0.001 of HYDRUS, is retried with 1 % and 10 %). When the Marquardt iteration of
    a stage stops, the run with adaptive time steps at its end point gives the objective of that point and the time
    levels of the next stage; the stages go on while they lower that objective by more than 0.1 %, and the best point
    is the result (all reported objectives are those of runs with adaptive time steps);
  - starts the estimation **from further starting points** (8 by default, in parallel threads): a parameter whose
    constraints span at most two decades is placed anywhere between them (log-uniformly over more than a decade),
    any other one within a factor of four of its initial estimate, by a Halton sequence (deterministic). The result
    with the lowest objective is kept; `Fit.out` and the results page list all starting points. The lowest objective
    can lie at unusual values where a parameter has no constraint (HORIZINF: n = 13.5), so constraints that bound the
    physically sensible range matter more than with a single start;
  - stops a run that needs more than 10 times the iterations of the run at the initial estimates (at least 2000;
    extreme trial values otherwise take minutes); the trial step, or the starting point, fails.

  The iteration of HYDRUS 4.17 (adaptive time steps in every run, one starting point) is kept: `--hydrus-stop` (CLI),
  *Iteration: HYDRUS 4.17* (interface). The first iteration of this mode reproduces 4.17.
- **Statistics:** standard errors from the inverse of the normal matrix of the last successful iteration and
  `s² = SSQ / (n − p)`, 95 % limits with Student's t for `n − p` degrees of freedom, correlation matrix, R² of the
  weighted regression of fitted on observed values, ME = Σ W r / n, MAE = Σ |W r| / n, RMSE = √(Σ (W r)² / (n − 1)),
  AIC = n ln(SSQ₀ / n) + 2p and BIC = n ln(SSQ₀ / n) + p ln n with SSQ₀ the objective at the start of the last
  iteration (as 4.17 prints them).
- **`Fit.out`** is written in the layout of 4.17, including the hydraulic property tables of the fitted materials.

Comparison with HYDRUS 4.17 (`crates/rusteau-io/tests/inverse_test.rs`):

- With the parameters of 4.17 (first and last iteration) the objective function agrees within 0.2 %, FITHYST 0.44 %.
- The first iteration of the HYDRUS mode reproduces the parameters of 4.17 to the printed digits in DRAINAGE, EVAPOR,
  FITHYST, FITTEST4, FITTEST5 (with the constraint on DifW), HORIZINF (with the constraint on θr) and MULTSTEP.
- The full estimation ends with the same or a lower objective function than 4.17 in all 12 examples and in all check
  runs (Rusteau iteration; the HYDRUS iteration gives the values in brackets):

| Example | Fitted | 4.17 SSQ | Rusteau SSQ | Note |
|---|---|---|---|---|
| DRAINAGE | θr, α, n, Ks, l | 0.1373 | 0.1341 (0.1349) | MIT = 6 |
| EVAPOR | θr, θs, α, n, Ks | 0.01260 | 0.01256 (0.01260) | θr goes to 0 |
| FITHYST | α, n, αw (hysteresis) | 0.01348 | 0.00434 (0.00490) | 4.17 stops after 3 iterations (1 % rule) while the SSQ still falls |
| FITTEST2 | α, n, Ks of 2 materials | 2.410e-5 | 1.749e-5 (1.795e-5) | |
| FITTEST3 | Kd, μw′, μs′ (chain) | 8.389e-7 | 8.388e-7 (8.388e-7) | μw′ and μs′ are perfectly correlated (r = −1) |
| FITTEST4 | Kd, β | 0.1418 | 0.1409 (0.1409) | |
| FITTEST5 | Ks, f, Dw, Kd, ω | 2.603e-5 | 2.601e-5 (2.601e-5) | |
| HORINFDR | θr, θs, α, n, Ks, l, αw (hysteresis) | 0.07064 | 0.06487 (0.07032) | |
| HORIZINF | θr, θs, α, n, Ks, l | 0.1163 | 0.05975 (0.1342) | another minimum, found from a further starting point: n = 13.5 (4.17: 2.76), θs 0.375 (0.426); n has no upper constraint in `Fit.in` |
| MULTSTEP | θr, θs, α, n, Ks, l | 4.543e-3 | 4.538e-3 (4.540e-3) | |
| ONESTEP | θr, α, n | 1.409e-3 | 1.124e-3 (1.230e-3) | |
| UPINFIL | θr, θs, α, n, Ks, l | 1.914e-3 | 1.215e-5 (1.392e-3) | the data were generated with the loam parameters, see below |

Check runs (4.17 → Rusteau): UPINFIL_P 2.70e-4 → 1.215e-5, UPINFIL_W1 1.40e-3 → 5.04e-5, HORIZINF_P 0.1071 →
0.05975, FITTEST2_W1 2.76e-7 → 8.33e-8, MULTSTEP_W1B 4.992e-3 → 4.988e-3, TEST2_HEATFIT 1.05e-4 → 4.7e-9.

**Path dependence of the HYDRUS iteration.** In UPINFIL, HORIZINF and HORINFDR the objective function computed with
adaptive time steps jumps by up to a few per cent of its value when a parameter changes slightly, and has several
local minima along narrow, curved valleys. Which point the HYDRUS iteration reaches depends on the last bits of the
arithmetic: initial values changed by 1e-12 (relative) give UPINFIL objectives between 4e-5 and 4e-3 and HORIZINF
between 0.112 and 0.149, and another platform's math library has the same effect. 4.17 behaves the same: with θs
changed in the 6th digit (check runs UPINFIL_P, HORIZINF_P) it ends at 2.70e-4 instead of 1.91e-3 and at 0.1071
instead of 0.1163. In the UPINFIL family the first step differs systematically (4.17 lowers l, Rusteau raises it),
at the level of single against double precision of the model. The Rusteau iteration does not depend on such details:
UPINFIL and UPINFIL_P end at the same minimum (1.215e-5), as do HORIZINF and HORIZINF_P (0.05975). The data of the
UPINFIL family evidently come from a run with the loam parameters of Carsel & Parrish (1988) (θr 0.078, θs 0.43,
α 0.036 /cm, n 1.56, Ks 2.889e-4 cm/s, l 0.5): there the objective is 2.8e-5 (UPINFIL_W1 3.5e-5), the remainder is
the rounding of the data; 4.17 ends 70 times higher. With frozen time levels alone (one starting point) UPINFIL
reaches 1.2e-5 too; the further starting points are needed for HORIZINF (from the initial estimates alone the iteration ends
at 0.154) and let UPINFIL_W1 reach 5.0e-5 within MIT = 50 (8.6e-4 from the initial estimates alone).

**Heat transport parameters** (check runs TEST2_HEATFIT, TEST2_HEATFIT2: TEST2 with b1 and Cn of the top material, the
second run also with Ks, fitted to 36 temperatures of a direct 4.17 run with the original values): the weights and the
objective at the initial and the final 4.17 parameters agree within 0.07 %. 4.17 hardly moves b1 in its first
iteration (its step equals a step in Cn alone, as if the derivative with respect to b1 were zero) and stops after
2 iterations at 1.05e-4. At the original values the objective is 5e-9, and Rusteau returns to them (b1 1.5673e16,
Cn 1.4334e14) in 3 iterations, also with the HYDRUS stopping rule.

Known differences of the reports: the hysteresis examples carry the 4.17 offset after a drying reversal (FITHYST: up to
0.4 cm in h; the `Nod_Inf.out` of 4.17 even shows a negative conductivity at the reversal front, K = −0.0547 cm/d at
node 7, t = 2 d); the "Observed & Fitted" table of 4.17 lists observations at position 0 twice (Rusteau once); in
ONESTEP the table and the contributions of 4.17 come from a final run that differs from its last iteration (SSQ
1.466e-3 against 1.409e-3), Rusteau has one consistent run; the water balance error of the final run is that of the
last balance (4.17 prints e.g. 0.0032 % for DRAINAGE, Rusteau 0.0000 %).

## HYDRUS-1D project files

HYDRUS-1D 4.x keeps a project as `<workspace>/<name>.h1d` next to the folder `<workspace>/<name>` with the input files.
Rusteau reads such projects (the `.h1d` file, the folder or any file in it) and writes them (*File → Save as HYDRUS-1D
project*, the option *Save also writes a HYDRUS-1D project*, or `rusteau … --save-h1d FILE`):

- `SELECTOR.IN` (file version 4, the blocks and header lines of 4.17), `PROFILE.DAT`, `ATMOSPH.IN` (with time-variable
  boundary conditions), `METEO.IN` (meteorological data), `FIT.IN` (inverse settings), `MATER.IN` (tabular hydraulic
  properties), `Hysteresis.in` (the initial hysteresis state, see
  [Restart with hysteresis](#restart-with-hysteresis-hysteresisin)), `HYDRUS1D.DAT` (the project summary of the HYDRUS interface: processes, units, numbers of materials,
  nodes and print times) and `DESCRIPT.TXT`, with Windows line ends. The project file `<name>.h1d` holds only the window
  layout of the HYDRUS interface (an OLE compound file); Rusteau writes the same layout for every project.
- Input files of the folder that the project does not need any more (e.g. an old `ATMOSPH.IN`) are removed; output
  files are left alone.
- Settings HYDRUS-1D cannot store are listed as notes: the interface parameters of the dual-permeability model, the
  irrigation mode *full duration*, flux-averaged output concentrations, the trapezoidal water-content dependence of
  reactions (`MoistDep.in` is written but HYDRUS does not use it), the extra flux output, the analytic evaluation of
  the hydraulic functions (HYDRUS always uses tables), the Rusteau iteration of the parameter estimation and the
  [further options](#further-options-of-the-hydrus-code-optionsin). Material and solute names are not stored either.

A project written and read back equals the saved one and gives identical results (`tests/legacy_save.rs`: the
built-in examples and all HYDRUS-1D examples, direct and inverse).

## Restart with hysteresis (Hysteresis.in)

With hysteresis in the retention curve (and the conductivity) after Kool & Parker, the state of the soil is more than
its pressure heads: each node lies on a scanning curve that depends on its history. HYDRUS reads this state from the
file `Hysteresis.in` of the project folder (HysterIn of INPUT.FOR): two header lines, then `n  Theta  Kappa` for every
node, the bottom node first (the lines are assigned in the internal node order, `n` is not used); Kappa is the
direction of the last change of the water content (−1 drying, +1 wetting). Every node then starts on the scanning curve
of the opposite direction through its (h, θ), as after a reversal, so a run continues from the end of a previous one.

Rusteau reads and writes `Hysteresis.in` with HYDRUS projects and keeps the state in the project (*Soil hydraulic
model* shows it and clears it; it is dropped when the profile gets new nodes). *Soil profile → Initial h →
Restart from these results* takes the heads and the state of all nodes at a print time of the last run.

- In HYDRUS the conductivity at the reversal point (ConO, needed for the scanning curves of K) is still zero when
  `Hysteresis.in` is read, so the drying curves of K vanish. Rusteau stores the conductivities with the state of a
  Rusteau restart (not in `Hysteresis.in`) and otherwise uses the conductivity of the main curve of the given direction.
- The water contents at the start equal those of the file exactly (`tests/hysteresis_restart.rs`); a run restarted
  during the rain of the infiltration example ends within 0.007 (retention) and 0.004 (retention and conductivity) of
  the uninterrupted run, against 0.32 and 0.011 without the state.

## Further options of the HYDRUS code (Options.in)

The HYDRUS code contains options its interface does not offer (INPUT.FOR: "New Options - Not supported by GUI"). The
computational module opens a file `Options.in` for them, but never reads it; the format is not documented, so HYDRUS-1D
always runs with the defaults. Rusteau offers them on the page *Run → Further options* and stores them in the project
(not in HYDRUS project files):

- **Extrapolation of the heads** (lExtrap, default on): each time step starts from heads extrapolated linearly from
  the two previous steps.
- **Enhancement factor of the thermal vapor flux** (iEnhanc, default on; Cass, A., G. S. Campbell, and T. L. Jones
  (1984). Enhancement of thermal water vapor diffusion in soil. *Soil Science Society of America Journal* 48: 25–32.
  <https://doi.org/10.2136/sssaj1984.03615995004800010005x>); off: factor 1.
- **Non-equilibrium water flow** (TauW; Ross, P. J., and K. R. J. Smettem (2000). A simple treatment of physical
  nonequilibrium water flow in soils. *Soil Science Society of America Journal* 64: 1926–1930.
  <https://doi.org/10.2136/sssaj2000.6461926x>; the source cites it as 2001): the water content
  approaches the equilibrium water content of the pressure head, θ − θold = (θeq(h) − θold)(1 − e^(−Δt/τw)).
  The convergence test uses the change of the equilibrium water content; the source multiplies it by the rate, which
  accepts large head changes when the rate is small (the infiltration example diverges with τw = 0.05 d). The iteration
  still gets harder: a smaller time step no longer increases the storage term, so strong infiltration may not
  converge.
- **Centrifuge** (lCentrif, Radius): the gravity term becomes ω²(R + depth)/g with the rotation speed (rpm) and the
  distance R of the surface from the axis; with a time-variable bottom condition the hBot column gives the speed and
  the bottom head is 0, as in HYDRUS. Rusteau writes the centrifugal term in conservative form (the flux of each element
  with ω²(R + depth)/g of its midpoint); the source form leaks at equilibrium (8 % balance error after 200 h in the test
  column). A column draining to a bottom at h = 0 reaches the analytic equilibrium h = −ω²/(2g)((R + L)² − (R + d)²)
  (`tests/advanced_options.rs`).
- **Density-dependent flow** (lDensity): density and viscosity of the solution relative to water as cubic polynomials
  of the liquid concentration of solute 1 (HYDRUS hard-codes ρ/ρ0 = 1 + 0.75 c, μ/μ0 = 1; Rusteau starts with these
  coefficients). The conductivity scales with ρ/μ, the gravity term and the storage with ρ. This is the formulation of
  the source: it solves ρ ∂θ/∂t = ∂/∂z[K(∂h/∂z + ρ)] − S, while the mass balance sums ρθ, so the balance error equals
  the solution mass gained by changes of the density.

Non-equilibrium flow, centrifuge and density-dependent flow are not available with the dual-permeability model.

## Pedotransfer functions (Rosetta)

*Neural network prediction (Rosetta)* on the page *Materials & parameters* (and *Rosetta…* next to the soil catalog
of the parameter estimation, for initial estimates) predicts θr, θs, α, n and Ks from the soil texture, as the Rosetta
Lite dialog of HYDRUS-1D. Input: the textural class, or sand, silt and clay in % (SSC; sum 99–101 %), optionally the bulk density (BD,
0.5–2 g/cm³), the water content at 33 kPa (field capacity) and at 1500 kPa (wilting point). The dialog shows the
prediction with the standard deviation of the ensemble, the USDA texture class, and the unsaturated conductivity
parameters K0 and L of Schaap & Leij (2000); *Accept* writes θr, θs, α, n and Ks (converted to the project units) into
the material.

Models (all are ensembles of neural networks trained on bootstrap samples; the prediction is the ensemble mean):

- **Rosetta 3** (default): Zhang, Y., and M. G. Schaap (2017). Weighted recalibration of the Rosetta pedotransfer model
  with improved estimates of hydraulic parameter distributions and summary statistics (Rosetta3). *Journal of
  Hydrology* 547: 39–53. <https://doi.org/10.1016/j.jhydrol.2017.01.004>. One network for the retention parameters
  and Ks together, trained with weights from the uncertainty of the fitted retention curves (less bias), and 1000
  bootstrap members instead of 60 or 100, for better estimates of the parameter distributions and their uncertainty.
- **Rosetta 1**, the models of Rosetta Lite in HYDRUS-1D: Schaap, M. G., F. J. Leij, and M. Th. van Genuchten (2001).
  ROSETTA: a computer program for estimating soil hydraulic parameters with hierarchical pedotransfer functions.
  *Journal of Hydrology* 251: 163–176. <https://doi.org/10.1016/S0022-1694(01)00466-8>.
- **Rosetta 2**: Rosetta 1 with the retention parameters corrected by the linear functions of Schaap, M. G., A. Nemes,
  and M. Th. van Genuchten (2004). Comparison of models for indirect estimation of water retention and available
  water in surface soils. *Vadose Zone Journal* 3: 1455–1463. <https://doi.org/10.2136/vzj2004.1455>.
- K0 and L: Schaap, M. G., and F. J. Leij (2000). Improved prediction of unsaturated hydraulic conductivity with the
  Mualem-van Genuchten model. *Soil Science Society of America Journal* 64: 843–851.
  <https://doi.org/10.2136/sssaj2000.643843x>.

The network weights are those of the Python package rosetta-soil 0.3.2 of the USDA-ARS U.S. Salinity Laboratory
(<https://github.com/usda-ars-ussl/rosetta-soil>, CC0 1.0), converted with `tools/rosetta_export.py` into
`crates/rusteau-core/data/rosetta.bin` (single-precision weights). The unit tests reproduce the predictions of
rosetta-soil (Rosetta 3 with arithmetic, geometric and log means, Rosetta 1 and 2; values and standard deviations) within 2e-5
(relative).

The model *Textural class* gives the class averages of the 12 USDA textural classes in the Rosetta database (model H1
of Schaap et al. 2001), as the *Textural classes* of Rosetta Lite; the values are those HYDRUS 4.17 shows. The soil
catalog offers the class averages of Carsel & Parrish (1988).

## Testing

```sh
cargo test --release --workspace
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
table covers the 23 examples of `Examples/Direct` and the check runs EnBal2b_A, EnBal2b_B, 5SEASON_IRR, 2NOHYSTR_LN,
2HYSTER_RET, ROOTUPTK_ACT and ROOTUPTK_ACT2; any other folder (including the check runs 2NOHYSTR_DUR, 2NOHYSTR_AE and 1SCALING_BC) must match
exactly. Any other difference, or a known one that grows, fails the test. A run that stops on non-convergence counts as known only
when the HYDRUS reference run stopped as well (`Error.msg` in the folder). The report lists both kinds.

`inverse_test.rs` (folder `RUSTEAU_INVERSE_DIR`, default `...\Examples\Inverse`; skipped when it is missing) checks
the weights, the observation order and the objective function with the 4.17 parameters, the first Marquardt iteration
of the HYDRUS mode, the full estimation with the Rusteau iteration (objective at most 5 % above 4.17, in every example)
and the HYDRUS iteration (examples with stable paths within 5 % of 4.17; the others are reported). The inverse check
runs (FITTEST2_W1, UPINFIL_W1, MULTSTEP_W1B, UPINFIL_P, HORIZINF_P, TEST2_HEATFIT, TEST2_HEATFIT2) pass the same tests
when copied into the folder. Run it with `--release`; with nine starting points it takes a few minutes on a machine
with several cores. Inverse examples in `RUSTEAU_EXAMPLES_DIR` are skipped by the direct comparison (their output files
belong to the final run with the fitted parameters).

The check runs are separate folders (from the check-run packages); copy them into the examples folder to include them.
Their entries in `KNOWN` are unused otherwise.

`time_step_replay_of_hydrus_runs` (in `reference_comparison.rs`) runs TEST1 on the time levels of the 4.17
`T_Level.out` (prescribed time levels) and requires agreement with all reference files: its 12 differences in a normal
run come only from a different sequence of time steps.

`legacy_save.rs` writes the built-in examples and every HYDRUS-1D example (`RUSTEAU_EXAMPLES_DIR`,
`RUSTEAU_INVERSE_DIR`) as HYDRUS-1D projects, reads them back and compares the projects (with
`RUSTEAU_LEGACY_SAVE_RUN=1` also the results of the HYDRUS examples).

Four tests in `reference_comparison.rs` and `writer.rs` use `crates/rusteau-io/tests/data/Meteo_sample.out` (a
`Meteo.out` with 1217 time steps written by the Fortran code). The file is not part of the source package; without it
these tests are skipped with a note.

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
8. **Relative solute balance error:** the change of the mass of solute 1 (DeltC) enters the relative error of solute 1
   only; the other solutes are related to their cumulative absolute fluxes (the source uses the DeltC of solute 1 for
   all solutes; TEST3, ROOTUPTK_ACT2).
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

- **Active solute uptake of a chosen solute** (`iActSolU`, the 6th value of the active-uptake line in `SELECTOR.IN`;
  the 2009 source allows active uptake with a single solute only). With time-variable boundary conditions the
  potential uptake rate comes from the `cBot` column of `ATMOSPH.IN` (4.17 ROOTUPTK_ACT: identical). With several
  solutes Rusteau takes the column of the solute with active uptake; the 4.17 check run with two solutes
  (ROOTUPTK_ACT2) confirms the file layout and the passive uptake of both solutes, but had no potential rate, so this
  column is not confirmed yet.

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
- **Iteration of the parameter estimation** in stages on frozen time levels with further starting points (default;
  the HYDRUS iteration is available), see [Parameter estimation](#parameter-estimation).
- **Rosetta 3** besides the Rosetta 1 models of HYDRUS' Rosetta Lite, see
  [Pedotransfer functions](#pedotransfer-functions-rosetta).
- **Restart state with conductivities** and the **further options of the HYDRUS code**, see
  [Restart with hysteresis](#restart-with-hysteresis-hysteresisin) and
  [Further options](#further-options-of-the-hydrus-code-optionsin).

### Known differences to HYDRUS 4.17

These are the cases the test suite accepts (`Cause` in `reference_comparison.rs`). In each case Rusteau follows the
2009 source, or the cause is numerical.

| Cause | Examples | Size | Explanation |
|---|---|---|---|
| `RealPrecision` | TEST1, TEST2, TEST11, ROOTUPTK (and ROOTUPTK_ACT, ROOTUPTK_ACT2), VOLATILE | heads and fluxes ≤ 1.2 %, single values up to 4 % | Single precision in HYDRUS changes the iteration count near the TolTh / TolH limit, and with it the time steps. With 10× tighter tolerances TEST1 drops from 12 to 5 problems; on the time levels of 4.17 it agrees exactly (TEST11 5 → 2, ROOTUPTK 7 → 5; TEST2 and VOLATILE already take the same steps, their differences lie within single steps). |
| `NoFluxTopNoise` | 1DRAINAG | vTop where rTop = 0, early heads | HYDRUS 4.17 prints a top flux of up to 0.49 cm/h for a no-flux top, and its water balance error grows to 0.031 cm. With tighter tolerances it needs 47622 time steps. Rusteau keeps vTop = 0 (balance error 1e-13); the final outflow differs by 0.04 %. |
| `PondingChaos` | 1INFILTR | after t ≈ 0.86 h | With TolH = 1 cm the onset of ponding is chaotic. With tighter tolerances both programs stop on non-convergence at the same transition (0.329 / 0.331 h). |
| `HysteresisTopNode` | 2HYSTER | K of the top node 5 % (14 % with a 0.01 h first drying step) | After ponding, HYDRUS 4.17 starts the drying scanning curve of the top node from a lower conductivity at the reversal point than the 2009 source. The factor depends on the length of the first drying step and on the grid. The formulas in the manual (Eqs. 2.84–2.93) are those of the source. The rule 4.17 uses is unknown, so Rusteau keeps the source. Retention and all other nodes agree, and the run without ponding (2HYSTER_R2) is identical. |
| `LatentOscillation` | EnBal2b | 17 of 1623 `Meteo.out` latent heat values | At a dry surface at hCritA, vTop alternates every time step between evaporation and 0 in both programs. With dtMax = 0.002 the oscillation disappears and both agree exactly. |
| `IrrigationSign` | 5SEASON_IRR | `sum(Infil)`, `sum(Evap)` after t = 226 | vTop ≈ ±1e-6 is booked as infiltration or evaporation according to its sign. |
| `PondingEndTolerance` | 2NOHYSTR_LN, 2HYSTER_RET | heads near the surface in the first steps after ponding ends, ≤ 0.08 cm | Same time steps and iteration counts; the difference is below the iteration tolerance: with 100× tighter TolTh / TolH the Rust result itself moves by 0.15–0.44 cm. The compiled 2009 source agrees with Rust. |

## Not supported

- **HP1** (coupling with the geochemical code PHREEQC) and **UnsatChem** (major-ion chemistry): the decision was made
  to leave both out. They are separate codes beside the HYDRUS-1D core (HP1 couples HYDRUS with PHREEQC; UnsatChem is
  a different transport and chemistry module), and neither is part of the HYDRUS-1D source this port follows.

## Credits

The numerical methods are those of HYDRUS-1D:

Šimůnek, J., M. Šejna, H. Saito, M. Sakai, and M. Th. van Genuchten (2013). *The HYDRUS-1D Software Package for
Simulating the One-Dimensional Movement of Water, Heat, and Multiple Solutes in Variably-Saturated Media*, Version 4.17.
Department of Environmental Sciences, University of California Riverside.
<https://www.pc-progress.com/en/Default.aspx?hydrus-1d>

The Rosetta network weights come from rosetta-soil (USDA-ARS U.S. Salinity Laboratory, CC0 1.0); the models are
those of Schaap et al. (2001, 2004) and Zhang & Schaap (2017), see [Pedotransfer functions](#pedotransfer-functions-rosetta).

## License

Rusteau-1D is free software: you can redistribute it and/or modify it under the terms of the GNU General Public License
as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version.
It is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of
MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See [LICENSE](LICENSE) for details.
