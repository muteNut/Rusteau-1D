# Boundary conditions of water flow
keywords: boundary conditions; Dirichlet; Neumann; atmospheric boundary condition; hCritA; hCritS; ponding; runoff; free drainage; deep drainage; seepage face; drains; Hooghoudt; Ernst

The Richards equation needs a condition at the soil surface (x = L) and at the bottom (x = 0) [@simunek2009].

## Pressure head and flux conditions

$$
h(x, t) = h_{0}(t)   at x = 0 or x = L   (first type, Dirichlet)
−K (∂h/∂x + 1) = q_{0}(t)   at x = 0 or x = L   (second type, Neumann)
$$

with the prescribed head h_{0} and flux q_{0} (positive upward).

## Atmospheric boundary condition {#atmospheric}

The potential fluid flux across the soil surface is determined by the external conditions (precipitation, potential evaporation), but the actual flux depends on the moisture conditions in the soil. The atmospheric boundary condition of [@neuman1974] limits the absolute flux and the pressure head at the surface:

$$
\|−K (∂h/∂x + 1)\| ≤ E   at x = L
h_{A} ≤ h ≤ h_{S}   at x = L
$$

E is the maximum potential rate of infiltration or evaporation, h_{A} the minimum pressure head allowed at the surface (−hCritA, the dry limit) and h_{S} the maximum (hCritS, the ponding depth; 0 without a surface layer). When one of the head limits is reached, the boundary switches from a flux to a head condition: during evaporation from a dry surface the surface stays at h_{A} and the actual evaporation falls below the potential one; during rain on a saturated surface the water ponds up to h_{S} and the excess runs off. When the actual flux again falls within the limits, the boundary switches back.

With a **surface layer**, ponded water accumulates and its height h changes with the balance of precipitation, infiltration and evaporation:

$$ −K (∂h/∂x + 1) = q_{0}(t) − dh/dt   at x = L $$

## Free drainage {#free}

A unit vertical hydraulic gradient at the bottom (gravity flow):

$$ ∂h/∂x = 0,   q = −K(h)   at x = 0 $$

suitable where the water table is far below the domain.

## Deep drainage {#deep}

The bottom flux depends on the position of the groundwater level [@hopmansStricker1989]:

$$ q(h) = −A_{qh} exp(B_{qh} \|h − GWL_{0L}\|) $$

with the empirical parameters A_{qh} [L/T] and B_{qh} [1/L] and the reference position of the groundwater level GWL_{0L}.

## Seepage face {#seepage}

The bottom of a lysimeter or a column with a free outlet: as long as the bottom is unsaturated the flux is zero; once the pressure head reaches h_{Seep} (usually 0) the head is fixed at h_{Seep} and water flows out.

## Horizontal drains {#drains}

The bottom flux equals the discharge to parallel drains or ditches, computed from the groundwater level with the formulas of [@hooghoudt1940] (homogeneous profiles, equivalent depth) and [@ernst1962] (layered profiles with vertical, horizontal and radial resistances), as implemented in SWAP [@vanDam1997], with the drain spacing, the entrance resistance and the conductivities of the layers.

## Time-variable conditions

Heads and fluxes at both boundaries, precipitation, evaporation and the dry limit can change with time ([time-variable data](time_variable_bc)). Time steps end at the times of the records.
