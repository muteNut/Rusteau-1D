# Projects and files
keywords: project; open; save; h1dr; h1d; HYDRUS-1D project; import; export; Fit.in; file menu

## Rusteau projects (.h1dr) {#h1dr}

A Rusteau project is one file `name.h1dr` (JSON text) with all inputs. *File → Save* (Ctrl+S) writes it, *Save as…* (Ctrl+Shift+S) asks for a new name, *Open project* (Ctrl+O) reads it. A project with unsaved changes shows `*` after the title; *New project*, *Open project* and *Quit* ask before discarding changes.

## HYDRUS-1D projects (.h1d) {#hydrus}

HYDRUS-1D 4.x keeps a project as `<workspace>/<name>.h1d` next to the folder `<workspace>/<name>` with the input files.

**Reading:** *File → Open project* with the `.h1d` file, *File → Import HYDRUS-1D project folder…*, or dropping the `.h1d` file, the folder or any file in it onto the window. The imported project has no file name yet; *Save* asks for a `.h1dr`. If the folder has a `Fit.in`, its observations are shown with the results as [measured data](measured_data).

**Writing:** *File → Save as HYDRUS-1D project (.h1d + folder)…* writes `name.h1d` and the folder `name` with

| File | Content |
|---|---|
| `SELECTOR.IN` | the main input (file version 4, the blocks of HYDRUS 4.17) |
| `PROFILE.DAT` | nodes, initial condition, materials, root distribution, scaling factors, observation nodes, fixed points |
| `ATMOSPH.IN` | time-variable boundary conditions |
| `METEO.IN` | meteorological data |
| `FIT.IN` | settings and observations of the parameter estimation |
| `MATER.IN` | look-up tables of the hydraulic properties |
| `Hysteresis.in` | initial hysteresis state |
| `HYDRUS1D.DAT`, `DESCRIPT.TXT` | project summary and description of the HYDRUS interface |

Input files of the folder that the project does not need any more are removed; output files are left alone. With the option *Save also writes a HYDRUS-1D project* every *Save* also writes `name.h1d` and the folder `name` next to the `.h1dr`.

Settings that HYDRUS-1D cannot store are listed on the page *Check & run* after writing: the interface parameters of the dual-permeability model, irrigation for the full duration, flux-averaged output concentrations, the trapezoidal water-content dependence of reactions, the extended flux output, the Rusteau iteration of the parameter estimation, the [further options](further_options), and the names of materials and solutes.

## Fit.in {#fitin}

*File → Import optimization file (Fit.in)…* reads the settings and observations of a HYDRUS-1D parameter estimation into the project, switches the inverse problem on and opens the page [Inverse problem](inverse_page). The page has buttons to import and export `Fit.in` as well.

## Results {#results}

Results stay in memory with the project until it is changed in a way that invalidates them (e.g. a unit change). *File → Export results (CSV)…* writes them into a folder ([output files](output_files)).

## Examples {#examples}

*File → Examples* loads one of the built-in [example projects](examples).
