# Coupled water, vapor and heat flow
keywords: vapor flow; water vapor; thermal vapor flux; isothermal vapor flux; enhancement factor; non-isothermal flow; Philip de Vries; coupled transport

With *Vapor flow* switched on, liquid water and water vapor move under gradients of the pressure head and of the temperature [@philipDeVries1957; @saito2006]. The water flow equation becomes

$$ ∂θ_{T}/∂t = ∂/∂x [ K_{Lh} (∂h/∂x + cos α) + K_{LT} ∂T/∂x + K_{vh} ∂h/∂x + K_{vT} ∂T/∂x ] − S $$

| Symbol | Meaning |
|---|---|
| θ_{T} | total water content, θ_{T} = θ_{L} + θ_{v}, the liquid water content plus the water vapor content expressed as an equivalent water content |
| K_{Lh} | isothermal hydraulic conductivity of the liquid phase [L/T] (the conductivity K(h) of the [hydraulic model](th_hydraulic_models)) |
| K_{LT} | thermal hydraulic conductivity of the liquid phase [L²/K/T] (temperature dependence of the surface tension) |
| K_{vh} | isothermal vapor hydraulic conductivity [L/T] |
| K_{vT} | thermal vapor hydraulic conductivity [L²/K/T] |

The vapor conductivities follow from the vapor diffusivity in the soil air, the saturated vapor density and the relative humidity h_{r} = exp(h M g / (R T)) of the soil air (M molecular weight of water, g gravitational acceleration, R gas constant). The thermal vapor flux is multiplied by the enhancement factor η(θ) of Cass et al. [@cass1984], which accounts for the higher thermal gradients in the air-filled pores and liquid islands (*Further options*: switchable).

The heat transport equation includes the transport of latent heat by the vapor flux and the convection of sensible heat by liquid water and vapor ([heat transport](th_heat)); water flow and heat transport are solved sequentially in every time step. The surface boundary condition is usually the [surface energy balance](th_energy_balance).

> As in HYDRUS, the capacity term of the vapor content contains the unit conversion in a form that is exact for metres; with cm or mm the results differ slightly (e.g. 3.4 % in the cumulative surface flux of the HYDRUS example EnBal2b when changing from cm/d to m/h). Rusteau keeps the HYDRUS expression so that its results match HYDRUS 4.17.
