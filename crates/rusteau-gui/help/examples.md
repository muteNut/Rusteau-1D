# Examples
keywords: examples; example projects; demonstration; tutorial projects

*File → Examples* loads one of four built-in projects. They are small and run in seconds.

## Infiltration & evaporation with roots

A 100 cm profile (101 nodes) of loam (0–50 cm) over sand, with the van Genuchten parameters of [@carselParrish1988]. Atmospheric boundary condition with runoff at the surface, free drainage at the bottom, six records of rain (3 cm/d in the first 2 days, 8 cm/d between days 8 and 12), potential evaporation and transpiration over 30 days. Roots in the upper 30 cm with the Feddes stress function [@feddes1978]. Observation nodes at 20, 50 and 100 cm. This project is loaded at start ([getting started](getting_started)).

## Solute pulse with sorption and decay

Steady infiltration of 2 cm/d into a 100 cm loam column (201 nodes). A tracer with a concentration of 10 is applied for 10 days with a concentration-flux boundary condition; linear sorption (Kd = 0.2 cm³/g, bulk density 1.3 g/cm³), first-order decay in the liquid and the solid phase (0.01 1/d), dispersivity 2 cm. Shows the solute pages and the breakthrough at the observation nodes ([solute transport](th_solute)).

## Parameter estimation: one-step outflow

A 10 cm soil core drains against a suction of 100 cm applied at the bottom (time unit hours). α, n and Ks are estimated from the cumulative outflow at 11 times and one retention point. The data were produced with θr 0.08, θs 0.40, α 0.025 1/cm, n 1.7, Ks 3 cm/h, l 0.5 and rounded; the initial estimates are off by 20–70 %. *Run* starts the estimation ([parameter estimation](th_inverse), [results](res_inverse)).

## Parameter estimation: soil temperature

Heat transport in a 1 m loam profile wetted by an infiltration of 2 cm/d; the surface temperature oscillates daily by ±8 °C around 15 °C. The coefficient b1 of the thermal conductivity and the heat capacity Cn of the solid phase are estimated from temperatures at 5, 10 and 20 cm over 5 days. The data were produced with the loam values of the HYDRUS examples and rounded to 0.001 °C; the initial estimates are off by +25 % and −20 % ([heat transport](th_heat)).

## HYDRUS-1D examples

The examples of the HYDRUS-1D distribution (`Examples/Direct` and `Examples/Inverse` of a HYDRUS-1D installation) can be opened directly ([HYDRUS-1D projects](projects#hydrus)); they are part of the test suite of Rusteau ([relation to HYDRUS-1D](hydrus_relation#verification)).
