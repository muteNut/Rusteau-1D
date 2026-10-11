# Measured data
keywords: measured data; observations; comparison; import data; CSV; text file; Fit.in data

Every results page can show measured data as points together with the simulated curves.

## Loading

Click *Load…* in the row *Measured data* of a results page and choose a text or CSV file with two columns:

| Page | Columns |
|---|---|
| Observation points | time and value (of the chosen variable at the chosen observation node) |
| Profiles | depth and value (at the first shown print time) |
| Boundary fluxes | time and value (of the chosen group) |
| Soil hydraulic properties | pressure head and θ, or pressure head and K |

Columns may be separated by blanks, tabs, commas or semicolons (with semicolons a decimal comma is accepted); lines without two numbers (headers, comments) are skipped. Each loaded set has a check box to show or hide it and ✖ to remove it.

## Data of Fit.in

The observations of a `Fit.in` in an imported HYDRUS-1D project folder (or imported with *File → Import optimization file*) are loaded automatically and shown on the matching pages.

Measured data belong to the project's units; they are discarded when a unit is changed.
