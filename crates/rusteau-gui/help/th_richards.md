# Variably saturated water flow
keywords: Richards equation; governing equation; water flow; unsaturated flow; sink term; pressure head; water content; hydraulic conductivity; inclination

One-dimensional uniform (equilibrium) water flow in a partially saturated rigid porous medium is described by a modified form of the Richards equation, assuming that the air phase plays an insignificant role in the liquid flow process and that water flow due to thermal gradients can be neglected [@simunek2009]:

$$ ∂θ/∂t = ∂/∂x [ K (∂h/∂x + cos α) ] − S $$

| Symbol | Meaning |
|---|---|
| h | pressure head [L] (negative in unsaturated soil) |
| θ | volumetric water content [L³/L³] |
| t | time [T] |
| x | spatial coordinate [L], positive upward (the interface uses the depth below the surface, positive downward) |
| S | sink term [L³/L³/T], the [root water uptake](th_root_uptake) |
| α | angle between the flow direction and the vertical axis (α = 0° for vertical flow, 90° for horizontal flow) |
| K | unsaturated hydraulic conductivity [L/T] |

The unsaturated hydraulic conductivity is the product of the saturated conductivity and the relative conductivity, scaled locally ([scaling](th_scaling)):

$$ K(h, x) = K_{s}(x) K_{r}(h, x) $$

The functions θ(h) and K(h) – the soil hydraulic properties – are given by one of the [soil hydraulic models](th_hydraulic_models). The water capacity C(h) = dθ/dh appears when the equation is written in terms of h; the numerical solution uses the mass-conservative mixed form with θ and h [@celia1990] ([numerical solution](th_numerics)).

## Extensions

- **Hysteresis**: θ(h) and K(h) depend on the wetting and drying history ([hysteresis](th_hysteresis)).
- **Non-equilibrium flow**: dual-porosity and dual-permeability formulations with exchange terms between two regions ([dual porosity](th_dual_porosity), [dual permeability](th_dual_permeability)), or a delayed approach of θ to equilibrium ([further options](th_advanced_options#nonequilibrium)).
- **Vapor flow and temperature**: with vapor flow the equation includes the isothermal and thermal vapor fluxes and the thermal liquid flux [@saito2006] ([vapor flow](th_vapor)); with temperature-dependent hydraulic properties h and K are scaled with the temperature ([hydraulic models](th_hydraulic_models#temperature)).
- **Centrifuge and density-dependent flow**: the gravity term cos α is replaced ([further options](th_advanced_options)).

## Boundary and initial conditions

The initial condition is the pressure head (or water content) of every node, set on the page [Soil profile](profile_editor). The boundary conditions at the surface and the bottom are described in [boundary conditions](th_bc_water).
