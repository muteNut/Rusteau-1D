# Units
keywords: units; unit conversion; length unit; time unit; mass unit; dimensions

All inputs and results use the units chosen on the page [Processes & units](processes): a length unit (mm, cm, m), a time unit (s, min, h, d, yr) and a mass unit, which is only a label. As in HYDRUS-1D, the dimension of a quantity is given in the help and the tooltips with L (length), T (time) and M (mass), e.g. Ks [L/T], α [1/L], concentrations [M/L³].

## Unit conversion {#conversion}

When the length or the time unit is changed with *Convert all values when a unit changes* switched on, every input is rescaled:

$$ value_{new} = value_{old} · f_{L}^{a} · f_{T}^{b} $$

with f_{L} and f_{T} the ratios of the units and a, b the dimension of the quantity. Converted are heads, depths and lengths, conductivities and fluxes, α (1/L), times and print times, atmospheric records, root uptake parameters, heat parameters, transport parameters (dispersivities, diffusion coefficients, rate constants, Freundlich / Langmuir coefficients Ks and η with the exponent β), concentrations (M/L³), the initial profile and the tables of tabular soil properties.

Not converted are the mass unit (only a label), dimensionless quantities, temperatures, activation energies and the meteorological inputs that HYDRUS reads in fixed units (radiation in MJ/m²/d, wind speed in km/d, sunshine hours, crop height and measurement heights in cm, latitude, altitude in m, the interception constant in mm). The time and rooting-depth columns of the meteorological data are converted.

With the option switched off only the unit label changes and all numbers stay as they are – useful when the values were entered in the wrong unit label.

Changing a unit discards the results and the measured data of the project (they are in the old units).

## Units of the heat parameters {#heat}

The thermal parameters are given in the project's length and time units with the mass in kg, as in HYDRUS-1D: the coefficients b1, b2, b3 of the thermal conductivity in kg L T⁻³ K⁻¹ (W/m/K = kg m s⁻³ K⁻¹ in SI units) and the volumetric heat capacities Cn, Co, Cw in kg L⁻¹ T⁻² K⁻¹ (J/m³/K). For cm and days:

| SI value | in cm and days |
|---|---|
| 1 W/m/K | 100 · 86400³ = 6.45 × 10^{16} kg cm d⁻³ K⁻¹ |
| 1 J/m³/K | 86400² / 100 = 7.46 × 10^{7} kg cm⁻¹ d⁻² K⁻¹ |

The buttons on the page [Heat transport](heat_page) enter typical values already converted to the project's units.

> Vapor flow depends slightly on the length unit, as in HYDRUS: the capacity term of the equivalent vapor content contains the unit conversion factor in a way that is exact only for metres. Rusteau keeps the HYDRUS expression so that its results match HYDRUS 4.17 ([vapor flow](th_vapor)).
