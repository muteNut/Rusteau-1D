# Heat transport
keywords: heat transport; thermal conductivity; heat capacity; convection; conduction; Chung Horton; Campbell; de Vries; thermal dispersivity; temperature

Neglecting the effects of water vapor diffusion, one-dimensional heat transport is described by [@sophocleous1979; @simunek2009]:

$$ C_{p}(θ) ∂T/∂t = ∂/∂x [ λ(θ) ∂T/∂x ] − C_{w} q ∂T/∂x − C_{w} S T $$

| Symbol | Meaning |
|---|---|
| T | temperature [K or °C] |
| C_{p}, C_{w} | volumetric heat capacities of the porous medium and of the liquid phase [M L⁻¹ T⁻² K⁻¹] |
| λ(θ) | apparent thermal conductivity of the soil [M L T⁻³ K⁻¹] |
| q | water flux [L/T] |
| S | sink term of root water uptake |

The first term on the right is conduction, the second convection by flowing water, the third the heat removed with the root water uptake. With [vapor flow](th_vapor) the equation contains the latent heat of the vapor flux as well [@saito2006].

## Heat capacity {#capacity}

Following [@deVries1963]:

$$ C_{p}(θ) = C_{n} θ_{n} + C_{o} θ_{o} + C_{w} θ + C_{v} a_{v} $$

with the volume fractions θ_{n} (solid phase), θ_{o} (organic matter), θ (water) and a_{v} (air), and the corresponding volumetric heat capacities (C_{n} = 1.92, C_{o} = 2.51, C_{w} = 4.18 MJ/m³/K; the air term is small).

## Thermal conductivity {#conductivity}

The apparent thermal conductivity combines the thermal conductivity of the soil without flow, λ_{0}(θ), with the macrodispersivity, a linear function of the water flux [@deMarsily1986]:

$$ λ(θ) = λ_{0}(θ) + β_{t} C_{w} \|q\| $$

with the longitudinal thermal dispersivity β_{t} (λL). λ_{0} follows [@chungHorton1987]:

$$ λ_{0}(θ) = b_{1} + b_{2} θ + b_{3} θ^{0.5} $$

with the empirical coefficients b_{1}, b_{2}, b_{3}; for sand 0.228, −2.406, 4.909, for loam 0.243, 0.393, 1.534 and for clay −0.197, −0.962, 2.521 W/m/K. Alternatively the function of [@campbell1985] computes λ_{0} from the volume fractions of quartz, other minerals and clay.

## Boundary conditions

At the surface and the bottom, the temperature (first type) or the heat flux (third type) is prescribed; the bottom can also have a zero gradient. The heat flux condition carries the heat of the water crossing the boundary:

$$ −λ ∂T/∂x + C_{w} q T = C_{w} q T_{0} $$

A sinusoidal daily surface temperature T(t) = T̄ + A sin(2π t/p − 7π/12) can be prescribed (maximum at 13:00), or the surface temperature follows from the [surface energy balance](th_energy_balance).

## Numerical solution

The heat transport equation is solved with Galerkin finite elements and the Crank–Nicolson scheme, like the [solute transport](th_numerics#solute), after the water flow of each time step.
