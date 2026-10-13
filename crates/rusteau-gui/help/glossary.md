# Symbols and glossary
keywords: symbols; glossary; notation; definitions; variables; HYDRUS variable names

## Symbols

| Symbol | Meaning | Unit |
|---|---|---|
| h | pressure head (negative when unsaturated) | L |
| θ | volumetric water content | L³/L³ |
| θ_{r}, θ_{s} | residual and saturated water content | – |
| S_{e} | effective saturation (θ − θ_{r})/(θ_{s} − θ_{r}) | – |
| α, n, m | parameters of the van Genuchten function, m = 1 − 1/n | 1/L, –, – |
| K, K_{s}, K_{r} | unsaturated, saturated and relative hydraulic conductivity | L/T, L/T, – |
| l | pore-connectivity parameter | – |
| C | water capacity dθ/dh | 1/L |
| q | volumetric water flux (positive upward) | L/T |
| S | sink term (root water uptake) | 1/T |
| T_{p}, T_{a} | potential and actual transpiration | L/T |
| E_{p}, ET_{p} | potential evaporation, potential evapotranspiration | L/T |
| α(h), ω, ω_{c} | water stress response function, water stress index, critical stress index | – |
| b(x) | normalized root distribution | 1/L |
| LAI, SCF | leaf area index, soil cover fraction | – |
| T | temperature | °C |
| λ, λ_{0} | apparent thermal conductivity, conductivity without flow | M L T⁻³ K⁻¹ |
| C_{p}, C_{n}, C_{o}, C_{w} | volumetric heat capacities of soil, solid phase, organic matter, water | M L⁻¹ T⁻² K⁻¹ |
| c, s, g | concentrations in the liquid, solid and gas phases | M/L³, M/M, M/L³ |
| ρ | bulk density | M/L³ |
| D_{L} | longitudinal dispersivity | L |
| D_{w}, D_{g} | molecular diffusion coefficients in water and air | L²/T |
| K_{s} (Kd), η, β | sorption coefficients (Freundlich, Langmuir) | L³/M, L³/M, – |
| k_{g} | Henry's constant | – |
| μ, γ, μ' | first-order decay, first-order (chain) production, zero-order production | 1/T, 1/T, M/L³/T |
| f | fraction of equilibrium sorption sites | – |
| θ_{im} | immobile water content | – |
| ω (α) | first-order mass-transfer coefficient | 1/T |
| Pe, Cr | grid Peclet and Courant numbers | – |
| Φ, SSQ | objective function of the parameter estimation | – |

## Glossary

| Term | Meaning |
|---|---|
| Atmospheric boundary condition | surface condition that switches between flux and head conditions according to precipitation, evaporation and the soil state ([theory](th_bc_water#atmospheric)) |
| Breakthrough curve | concentration against time at a depth or in the outflow |
| Dual porosity | two pore regions, one of them immobile ([theory](th_dual_porosity)) |
| Dual permeability | two pore domains that both conduct water ([theory](th_dual_permeability)) |
| Fixed point | depth that is always a node of the mesh ([soil profile](profile_editor#discretization)) |
| hCritA | minimum pressure head allowed at the surface (dry limit of evaporation) |
| hCritS | maximum ponding depth at the surface before runoff |
| Hysteresis | dependence of θ(h) on the wetting and drying history ([theory](th_hysteresis)) |
| Material | soil layer type with its own parameters |
| Node | point of the finite element mesh |
| Observation node | node whose values are recorded at every time step |
| Print time | time at which profiles and balances are stored |
| Pumping effect | drift of hysteresis loops in models without memory of reversal points |
| Retention curve | relation θ(h), also water retention characteristic or soil water characteristic |
| Sub-region | part of the profile with its own mass balance |
| Time level | the end of a time step |
| Two-site sorption | sorption on equilibrium and kinetic sites ([theory](th_nonequilibrium#twosite)) |

## HYDRUS variable names

The tooltips name the variables of the HYDRUS-1D input files (e.g. `hCritA`, `TolTh`, `dtMin`, `iKappa`, `lFluxOut`, `TauW`), so that the HYDRUS-1D manual [@simunek2009; @simunek2013] can be used alongside this help.
