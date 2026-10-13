# Workflow of a project
keywords: workflow; steps; setting up a model; new project; procedure

A simulation is set up in the order of the navigation panel. The workflow bar at the bottom of the input pages leads from one page to the next.

## Step 1 – Project

1. [Processes & units](processes): choose the length and time units and switch on the processes (water flow, vapor flow, root water uptake, root growth, heat transport, solute transport, meteorological ET, inverse problem). Pages of processes that are switched off are hidden.
2. [Time & output](time_output): initial and final time, time-step limits, print times.

## Step 2 – Soil

3. [Hydraulic model](hyd_model): the model of the soil hydraulic properties and hysteresis.
4. [Materials & parameters](soil_params): one material per soil layer type with its hydraulic parameters (from the catalog, from Rosetta or measured).
5. [Soil profile](profile_editor): depth and discretization, materials of the nodes, the initial condition, root distribution, observation nodes, scaling factors.

## Step 3 – Boundary conditions

6. [Water flow](water_bc): the upper and lower boundary conditions.
7. [Time-variable data](time_variable_bc): records of precipitation, evaporation, transpiration and boundary heads, fluxes, temperatures and concentrations.
8. [Meteorological data](meteo_page): data for the potential evapotranspiration or the energy balance.

## Step 4 – Uptake and transport

9. [Root water & solute uptake](root_uptake_page)
10. [Heat transport](heat_page)
11. Solute transport: [general](solute_general), [transport parameters](solute_params), [reactions](solute_reactions), [temperature and water-content dependence, colloids](solute_advanced), [boundary conditions](solute_bc)

## Step 5 – Run

12. [Iteration criteria](iteration): tolerances of the iterative solution.
13. [Further options](further_options): options of the HYDRUS code that HYDRUS-1D does not offer.
14. [Check & run](check_run): checks the input and runs the calculation.

## Parameter estimation

For an inverse problem, set up the parameters and observations on the page [Inverse problem (Fit.in)](inverse_page); *Run* then estimates the parameters ([results](res_inverse)).

## Tips

- Check the soil hydraulic functions on the page *Materials & parameters* before running: unrealistic parameters (e.g. n close to 1 with a large α) are the most common cause of convergence problems ([questions and problems](faq)).
- Start with a coarse output (few print times) and a short simulation time; refine when the model behaves as expected.
- Keep the [mass balance](res_balance) error below about 1 %; otherwise reduce the maximum time step or refine the profile near the surface ([discretization](th_discretization)).
