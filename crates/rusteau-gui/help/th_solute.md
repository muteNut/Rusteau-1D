# Solute transport equations
keywords: advection dispersion equation; convection dispersion; sorption; Freundlich; Langmuir; linear sorption; volatilization; Henry; decay chain; first-order decay; zero-order production; dispersion coefficient; tortuosity; retardation

## Governing equation

The partial differential equations governing one-dimensional advective–dispersive transport of solutes under transient flow in a variably saturated rigid porous medium are, for a chain of decaying solutes [@simunek2009; @bear1972]:

$$
∂(θc_{k})/∂t + ∂(ρs_{k})/∂t + ∂(a_{v}g_{k})/∂t = ∂/∂x (θ D_{k}^{w} ∂c_{k}/∂x) + ∂/∂x (a_{v} D_{k}^{g} ∂g_{k}/∂x) − ∂(q c_{k})/∂x − φ_{k} − S c_{r,k}
$$

| Symbol | Meaning |
|---|---|
| c, s, g | concentrations in the liquid [M/L³], solid [M/M] and gas [M/L³] phases |
| ρ | bulk density [M/L³] |
| a_{v} | air content [L³/L³] |
| q | volumetric water flux [L/T] |
| D^{w}, D^{g} | dispersion coefficient in the liquid phase and diffusion coefficient in the gas phase [L²/T] |
| φ | reactions: decay and production [M/L³/T] |
| S c_{r} | removal by root uptake ([root solute uptake](th_root_solute)) |
| k | number of the solute in the chain |

The reactions of solute k are first-order decay with the constants μ_{w}, μ_{s}, μ_{g} in the three phases, first-order production from the parent solute k−1 with γ_{w}, γ_{s}, γ_{g}, and zero-order production with μ'_{w}, μ'_{s}, μ'_{g}:

$$ φ_{k} = θ(μ_{w}c_{k} − γ_{w}c_{k−1} − μ'_{w}) + ρ(μ_{s}s_{k} − γ_{s}s_{k−1} − μ'_{s}) + a_{v}(μ_{g}g_{k} − γ_{g}g_{k−1} − μ'_{g}) $$

## Sorption {#sorption}

The adsorption isotherm relating s and c is described by a generalized non-linear equation:

$$ s_{k} = K_{s,k} c_{k}^{β_{k}} / (1 + η_{k} c_{k}^{β_{k}}) $$

with the empirical coefficients K_{s} [L³/M], β [–] and η [L³/M]. It simplifies to the Langmuir isotherm (β = 1), the Freundlich isotherm (η = 0) and the linear isotherm (β = 1, η = 0) with the distribution coefficient K_{d} = K_{s}. For linear sorption the retardation factor is R = 1 + ρK_{d}/θ.

## Gas phase {#gas}

The concentrations in the liquid and gas phases are related by Henry's law [@jury1983]:

$$ g_{k} = k_{g,k} c_{k} $$

## Dispersion {#dispersion}

The dispersion coefficient in the liquid phase is

$$ θ D^{w} = D_{L} \|q\| + θ D_{w} τ_{w} $$

with the longitudinal dispersivity D_{L} [L], the molecular diffusion coefficient in free water D_{w} and the tortuosity factor τ_{w}. The tortuosity factors of the liquid and gas phases follow [@millingtonQuirk1961]:

$$ τ_{w} = θ^{7/3} / θ_{s}^{2},   τ_{g} = a_{v}^{7/3} / θ_{s}^{2} $$

or [@moldrup1997; @moldrup2000].

## Boundary conditions {#bc}

$$
c(x, t) = c_{0}(t)   (first type, concentration)
−θ D ∂c/∂x + q c = q_{0} c_{0}(t)   (third type, concentration flux)
∂c/∂x = 0   (zero gradient, bottom only)
$$

The third-type condition at the surface makes the solute enter only with infiltrating water. With **volatilization** the surface flux additionally contains the diffusion of the gas through a stagnant boundary layer of thickness d [@jury1983]:

$$ −θ D ∂c/∂x + q c = q_{0} c_{0} − (D_{g}/d) (k_{g} c − g_{atm}) $$

## Non-equilibrium transport

Two-site sorption, mobile–immobile water, dual non-equilibrium and dual-permeability transport are described in [non-equilibrium transport](th_nonequilibrium); colloids in [colloid transport](th_colloids).

## Numerical solution

Galerkin finite elements with time weighting ε (Crank–Nicolson by default), optional upstream weighting and artificial dispersion; the time step is limited by the Courant number ([numerical solution](th_numerics#solute), [discretization](th_discretization)).
