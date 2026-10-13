# Hargreaves equation
keywords: Hargreaves; Hargreaves-Samani; potential evapotranspiration; temperature-based; extraterrestrial radiation

When only air temperatures are available, the potential evapotranspiration can be estimated with the Hargreaves equation [@hargreavesSamani1985; @hargreaves1994]:

$$ ET_{p} = 0.0023 R_{a} (T_{m} + 17.8) √(T_{max} − T_{min}) $$

| Symbol | Meaning |
|---|---|
| ET_{p} | potential evapotranspiration [mm/d], converted to the project's units |
| R_{a} | extraterrestrial radiation, expressed as equivalent evaporation (R_{a} [MJ/m²/d] × 0.408) [mm/d] |
| T_{m} | mean daily air temperature, (T_{max} + T_{min})/2 [°C] |
| T_{max}, T_{min} | maximum and minimum daily air temperature [°C] |

R_{a} follows from the latitude of the site and the day of the year [@allen1998]. The temperature range √(T_{max} − T_{min}) stands for the cloudiness: clear days have a large range and high radiation. ET_{p} is partitioned into evaporation and transpiration with the soil cover fraction ([partitioning](th_atmosphere#lai)).

The Hargreaves equation is recommended by FAO-56 when solar radiation, humidity and wind data are missing [@allen1998]; it tends to overestimate under humid and underestimate under windy, dry conditions.
