# Penman–Monteith combination equation
keywords: Penman-Monteith; potential evapotranspiration; FAO-56; net radiation; aerodynamic resistance; canopy resistance; vapor pressure deficit; psychrometric constant; Angstrom; long-wave radiation

With meteorological data, the potential evapotranspiration is calculated with the Penman–Monteith combination equation [@monteith1981; @monteithUnsworth1990] in the form of FAO Irrigation and Drainage Paper 56 [@allen1998]:

$$ ET_{p} = (1/λ) [ Δ (R_{n} − G) + ρ_{a} c_{p} (e_{s} − e_{a}) / r_{a} ] / [ Δ + γ (1 + r_{c}/r_{a}) ] $$

| Symbol | Meaning |
|---|---|
| ET_{p} | potential evapotranspiration [mm/d], converted to the project's units |
| λ | latent heat of vaporization, λ = 2.501 − 0.002361 T [MJ/kg] |
| Δ | slope of the saturation vapor pressure curve [kPa/°C] |
| R_{n} | net radiation at the surface [MJ/m²/d] |
| G | soil heat flux [MJ/m²/d], neglected for daily values |
| ρ_{a} c_{p} | volumetric heat capacity of the air |
| e_{s} − e_{a} | vapor pressure deficit [kPa] |
| r_{a}, r_{c} | aerodynamic and canopy resistance [s/m] |
| γ | psychrometric constant [kPa/°C], γ = 0.0016286 P/λ, with the atmospheric pressure P = 101.3 [(293 − 0.0065 z)/293]^{5.253} at the altitude z [m] |

## Vapor pressure

The saturation vapor pressure is the mean of e°(T_{max}) and e°(T_{min}), with e°(T) = 0.6108 exp[17.27 T/(T + 237.3)], and Δ the mean of the slopes 4098 e°(T)/(T + 237.3)² at T_{max} and T_{min}. The actual vapor pressure e_{a} follows from the relative humidity or is given directly [@allen1998].

## Aerodynamic and canopy resistance

$$
r_{a} = ln[(z_{m} − d)/z_{om}] ln[(z_{h} − d)/z_{oh}] / (k² u_{z})
r_{c} = r_{l} / LAI_{active} = 200 / LAI
$$

with the measurement heights of wind z_{m} and humidity z_{h}, the zero-plane displacement d = 0.667 h_{c}, the roughness lengths z_{om} = 0.123 h_{c} and z_{oh} = 0.1 z_{om} of a crop of height h_{c}, the von Kármán constant k = 0.41 and the wind speed u_{z}; r_{l} = 100 s/m and LAI_{active} = 0.5 LAI [@allen1998]. The LAI comes from the crop data or from the crop height (clipped grass: 0.24 h_{c} with h_{c} in cm; alfalfa: 5.5 + 1.5 ln h_{c}).

## Radiation {#radiation}

The net radiation is the difference of the net short-wave and the net long-wave radiation:

$$
R_{n} = (1 − a) R_{s} − R_{nl}
R_{s} = (a_{s} + b_{s} n/N) R_{a}
R_{nl} = σ [(T_{max,K}^{4} + T_{min,K}^{4})/2] (a_{l} + b_{l} √e_{a}) f
$$

a is the albedo, R_{s} the solar radiation (measured, or from the relative sunshine duration n/N after [@angstrom1924]; a_{s} = 0.25, b_{s} = 0.5), R_{a} the extraterrestrial radiation from the latitude and the day of the year, N the day length, σ the Stefan–Boltzmann constant, a_{l} + b_{l} √e_{a} the net emissivity (0.34, −0.14 [@allen1998]) and f the cloudiness factor: f = a_{1} n/N + b_{1} from the sunshine duration (0.9, 0.1 [@doorenbosPruitt1977]), or f = a_{c} R_{s}/R_{so} + b_{c} from the ratio of the measured to the clear-sky radiation (1.35, −0.35 [@allen1998]).

## Partitioning

ET_{p} is divided into potential transpiration and evaporation with the soil cover fraction SCF = 1 − exp(−k LAI) ([partitioning](th_atmosphere#lai)) [@ritchie1972]; the canopy interception is subtracted from the transpiration ([interception](th_atmosphere#interception)).

See also: [Hargreaves equation](th_hargreaves) · [surface energy balance](th_energy_balance) · [page Meteorological data](meteo_page)
