# Output files
keywords: output; CSV; export; T_LEVEL; NOD_INF; OBS_NODE; BALANCE; A_LEVEL; Meteo.out; results files

*File → Export results (CSV)…* (and the [command line program](cli)) write the results as comma-separated files with a header line. The files correspond to the output files of HYDRUS-1D [@simunek2009]. All values are in the project's units; signs follow HYDRUS (fluxes positive upward, i.e. negative values enter the soil at the surface and leave the profile at the bottom).

## T_LEVEL.csv – time-level information {#tlevel}

One line per time step (or every n-th step, [Time & output](time_output)).

| Column | Meaning |
|---|---|
| `time`, `dt`, `tlevel` | time, time step, number of the time level |
| `iter_w`, `it_cum` | iterations of the water flow in this step, cumulative iterations |
| `kod_top`, `kod_bot` | boundary condition code at the top and bottom in this step (positive: head, negative: flux) |
| `rTop`, `rRoot` | potential surface flux and potential transpiration [L/T] |
| `vTop`, `vRoot`, `vBot` | actual surface flux, actual transpiration, actual bottom flux [L/T] |
| `sum_rTop` … `sum_vBot` | cumulative values of the fluxes [L] |
| `hTop`, `hRoot`, `hBot` | pressure head at the surface, mean head of the root zone, head at the bottom [L] |
| `runoff`, `sum_runoff` | surface runoff [L/T] and its cumulative value [L] |
| `volume` | water volume in the profile per unit area [L] |
| `sum_infil`, `sum_evap` | cumulative infiltration and evaporation [L] |
| `precip`, `snow_layer` | precipitation rate, thickness of the snow layer (water equivalent) |
| per solute | `cvTop`, `cvBot` (solute fluxes), `sum_cvTop`, `sum_cvBot`, `sum_cvCh0`, `sum_cvCh1` (cumulative zero- and first-order reactions), `cTop`, `cRoot`, `cBot`, `cvRoot`, `sum_cvRoot`, `sum_cvNEql`, `cGWL`, `cRunOff`, `sum_cRunOff` |
| `temp_top`, `temp_bot` | temperatures at the top and the bottom [°C] |
| `peclet`, `courant` | maximum Peclet and Courant numbers of the solute transport |
| `cum_w_transf` | cumulative water transfer between the domains (dual porosity / permeability) |

## NOD_INF.csv – profiles at the print times {#nodinf}

| Column | Meaning |
|---|---|
| `time`, `node`, `depth` | print time, node number (from the top), depth [L] |
| `head`, `theta` | pressure head [L], water content [–] |
| `K`, `C` | hydraulic conductivity [L/T], water capacity [1/L] |
| `flux`, `sink` | water flux [L/T], root water uptake [1/T] |
| `kappa` | branch of the hysteresis model (−1 drying, +1 wetting) |
| `v_over_Ks` | flux relative to Ks of the top node |
| `temp` | temperature [°C] |
| `h_matrix`, `th_matrix` | head and water content of the matrix or the immobile region (dual porosity / permeability) |
| per solute | `conc` (liquid), `sorb` (sorbed or immobile), `conc_matrix`, `flux_conc` |

## OBS_NODE.csv – observation nodes {#obsnode}

`time`, `node`, `head`, `theta`, `temp`, `flux`, `h_matrix`, `th_matrix` and per solute `conc`, `conc_matrix`, `flux_conc`, one line per time step and observation node.

## BALANCE.csv – mass balance {#balance}

`time`, `area` (length of the region), `volume`, `inflow`, `h_mean`, `top_flux`, `bot_flux`, `WatBalT` (absolute error [L]) and `WatBalR_percent` (relative error [%]) at the print times ([mass balance](res_balance)).

## A_LEVEL.csv – atmospheric records {#alevel}

Cumulative fluxes and boundary heads at the end of every atmospheric record: `time`, `sum_rTop`, `sum_rRoot`, `sum_vTop`, `sum_vRoot`, `sum_vBot`, `hTop`, `hRoot`, `hBot`.

## T_LEVEL1.csv – extended fluxes {#tlevel1}

Written with the option *Write extended flux table T_Level1*: potential and actual precipitation, evaporation, surface flux, infiltration, transpiration, bottom flux, runoff, their cumulative values, the storage, the snow layer and the liquid and vapor parts of the surface flux.

## METEO.csv and Meteo.out {#meteo}

With meteorological data: the potential evapotranspiration and its components, the radiation terms and, with the energy balance, the surface energy fluxes. `Meteo.out` has the fixed-width layout of HYDRUS-1D.

## IRRIG.csv {#irrig}

With triggered irrigation: one line per irrigation event (`event`, `t_trigger`, `t_start`, `t_end`, `amount`).

## Fit.out

The results of a parameter estimation are written with *Save Fit.out…* on the page [Parameter estimation results](res_inverse), in the layout of HYDRUS-1D 4.17.
