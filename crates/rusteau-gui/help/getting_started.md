# Getting started
keywords: tutorial; first simulation; quick start; example; step by step

This topic walks through a first simulation with the built-in example *Infiltration & evaporation with roots*: a 1 m profile of loam over sand, with rain, evaporation and root water uptake over 30 days. It is the project shown when the program starts.

## 1. Open the example

Start Rusteau-1D. The example is loaded already; otherwise choose *File → Examples → Infiltration & evaporation with roots*. The navigation panel on the left lists the input pages in five steps and the results pages below them ([main window](interface)).

## 2. Look at the project

- **Processes & units**: water flow and root water uptake are switched on; lengths in cm, times in days ([Processes & units](processes)).
- **Time & output**: 0 to 30 days, print times every 5 days ([Time & output](time_output)).
- **Soil → Hydraulic model**: van Genuchten–Mualem without hysteresis ([Soil hydraulic model](hyd_model)).
- **Soil → Materials & parameters**: material 1 *Loam* and material 2 *Sand* with the catalog values of [@carselParrish1988]; the retention curve θ(h) and the conductivity K(h) are drawn below the table ([Materials & parameters](soil_params)).
- **Soil → Soil profile**: 101 nodes over 100 cm; loam down to 50 cm, sand below; an initial pressure head of −200 cm in the loam and −80 cm in the sand; roots in the upper 30 cm; observation nodes at 20, 50 and 100 cm ([soil profile editor](profile_editor)).
- **Boundary conditions → Water flow**: atmospheric boundary condition with surface runoff at the top, free drainage at the bottom ([Water flow boundary conditions](water_bc)).
- **Boundary conditions → Time-variable data**: six records of precipitation, potential evaporation and potential transpiration ([Time-variable boundary data](time_variable_bc)).
- **Uptake & transport → Root water & solute uptake**: the Feddes stress function [@feddes1978] ([Root water uptake](root_uptake_page)).

Rest the mouse on any input to see what it means; press **F1** for the help of the page.

## 3. Run the calculation

Press **F5**, click **▶ Run** at the top right, or go to *Run → Check & run*. The page *Check & run* lists problems of the input (e.g. a material with θs ≤ θr) before anything is computed ([Check & run](check_run)). During the run the status bar shows the progress; **■ Stop** cancels it and keeps the partial results.

## 4. Look at the results

When the run has finished, the page *Profiles* opens:

- **Profiles**: pressure head, water content, fluxes, root water uptake … against depth at the print times ([Profiles](res_profiles)).
- **Observation points**: the same quantities against time at the observation nodes ([Observation points](res_obs)).
- **Boundary fluxes & heads**: actual and potential surface fluxes, bottom flux, transpiration, cumulative values ([Boundary fluxes](res_fluxes)).
- **Mass balance**: the water balance error of the run ([Mass balance](res_balance)).

*File → Export results (CSV)…* writes all tables as CSV files for further analysis ([Output files](output_files)).

## 5. Change something and compare

For example, set the potential transpiration of the third record to 1 cm/d on the page *Time-variable data*, run again (F5) and look at the root water uptake on the page *Boundary fluxes & heads*: the actual transpiration stays below the potential one when the soil dries (water stress).

## Next steps

- Start a new project with *File → New project* (Ctrl+N) and follow the [workflow](workflow).
- Import a HYDRUS-1D project with *File → Open project* or by dragging its folder onto the window ([Projects](projects)).
- Try the other [examples](examples): solute transport and two parameter-estimation problems.
