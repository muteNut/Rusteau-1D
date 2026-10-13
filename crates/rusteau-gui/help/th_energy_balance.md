# Surface energy balance
keywords: energy balance; surface energy balance; evaporation; sensible heat; latent heat; soil heat flux; surface temperature; aerodynamic resistance; soil surface resistance; albedo; emissivity

With the option *Evaporation and heat flux from the surface energy balance* (page [Meteorological data](meteo_page)) the evaporation is not prescribed but follows, together with the soil heat flux, from the energy balance at the soil surface [@saito2006; @vanBavelHillel1976]:

$$ R_{n} − H − L E − G = 0 $$

| Symbol | Meaning |
|---|---|
| R_{n} | net radiation [W/m²] |
| H | sensible heat flux [W/m²] |
| L E | latent heat flux [W/m²]: the evaporation E times the latent heat of vaporization L |
| G | soil heat flux [W/m²], the upper boundary condition of the [heat transport](th_heat) |

It requires heat transport and is usually combined with [vapor flow](th_vapor).

## Net radiation

$$
R_{n} = (1 − a) R_{s} + R_{L↓} − R_{L↑}
R_{L↑} = ε_{s} σ T_{s}^{4},   R_{L↓} = ε_{a} σ T_{a}^{4}
$$

- the surface albedo a depends on the water content θ of the surface [@vanBavelHillel1976]: 0.25 for θ ≤ 0.1, 0.35 − θ for 0.1 < θ ≤ 0.25, 0.1 for θ > 0.25;
- the soil emissivity ε_{s} = min(0.9 + 0.18 θ, 1) [@vanBavelHillel1976];
- the clear-sky emissivity of the atmosphere ε_{ac} = 1.24 (e_{a}/T_{a})^{1/7} [@brutsaert1975], corrected for the cloud cover c: ε_{a} = (1 − 0.84 c) ε_{ac} + 0.84 c [@campbellNorman1998];
- daily radiation is distributed over the day with the sine of the solar elevation.

## Sensible and latent heat

$$
H = C_{a} (T_{s} − T_{a}) / r_{h}
E = (ρ_{vs} − ρ_{va}) / (r_{v} + r_{s})
$$

C_{a} is the volumetric heat capacity of the air, T_{s} and T_{a} the surface and air temperatures, ρ_{vs} the water vapor density at the soil surface (the saturated vapor density times the relative humidity of the soil air, from the pressure head and temperature of the top node), ρ_{va} that of the air, r_{h} = r_{v} the aerodynamic resistances to heat and vapor transfer, and r_{s} the soil surface resistance to evaporation.

The aerodynamic resistance follows from the wind speed with stability corrections for unstable and stable atmospheric conditions (Monin–Obukhov similarity, iterated) [@campbellNorman1998; @saito2006]; with no wind, heat is transferred by diffusion only. The soil surface resistance increases as the surface dries [@vanDeGriendOwe1994]:

$$ r_{s} = 10 exp[35.63 (0.15 − θ)]   for θ < 0.15;   r_{s} = 10 s/m   otherwise $$

## Coupling

The energy balance is solved together with the [heat transport](th_heat) for the surface temperature; the evaporation becomes the flux condition of the water flow, limited by hCritA ([atmospheric boundary condition](th_bc_water#atmospheric)). With vapor flow, the latent heat carried through the surface by the soil vapor flux is returned to the top node. The time step is limited so that the surface temperature changes smoothly.
