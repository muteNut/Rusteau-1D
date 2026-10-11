# Atmospheric inputs: partitioning, daily variation, interception, snow
keywords: potential evapotranspiration; partitioning; leaf area index; LAI; soil cover fraction; daily variation; interception; snow; snow melt; degree-day

## Partitioning of the potential evapotranspiration {#lai}

When only the potential evapotranspiration ET_{p} is known, it is divided into potential transpiration T_{p} and potential evaporation E_{p} with the leaf area index LAI [@ritchie1972]:

$$
SCF = 1 − exp(−k · LAI)
T_{p} = ET_{p} · SCF,   E_{p} = ET_{p} (1 − SCF)
$$

SCF is the soil cover fraction and k the extinction coefficient of the radiation in the canopy (HYDRUS default 0.463). The same partitioning is used with [meteorological data](th_penman).

## Daily variation of evaporation and transpiration {#daily}

Daily values of evaporation and transpiration can be distributed over the day with the functions of Fayer [@fayer2000], as in HYDRUS: during the night (t < 0.264 d and t > 0.736 d) the rate is 24 % of the daily mean, during the day it follows a sine wave:

$$ T_{p}(t) = 2.75 T_{p} sin(2π t − π/2)   for 0.264 d ≤ t ≤ 0.736 d;   T_{p}(t) = 0.24 T_{p}   otherwise $$

with t the time of the day in days; the integral over the day equals the daily rate. The same applies to the evaporation.

## Canopy interception {#interception}

Part of the precipitation is intercepted by the leaves and evaporates. The interception follows von Hoyningen-Hüne [@vonHoyningenHune1983] and Braden [@braden1985]:

$$ I = a · LAI [ 1 − 1 / (1 + SCF · P / (a · LAI)) ] $$

with the precipitation P, the interception constant a [L] (the interception capacity is a·LAI) and the soil cover fraction SCF. The intercepted water evaporates first and reduces the potential transpiration; the rest reaches the soil.

## Snow {#snow}

With snow accumulation, precipitation falls as snow at air temperatures below −2 °C, as rain above +2 °C, and as a mixture in between (the snow fraction decreases linearly from 1 to 0 between −2 and +2 °C). Once a snow layer exists, all precipitation is added to it. The snow melts at air temperatures above 0 °C with a degree-day factor m_{f} [L/T/°C]:

$$ M = m_{f} · T_{a}   for T_{a} > 0 °C $$

(a temperature-index approach, see Hock [@hock2003]); the HYDRUS default is 0.43 cm/d/°C. Evaporation is taken from the snow layer first. Solutes in the precipitation are stored in the snow and released with the melt water.

## Sinusoidal precipitation

With *sinusoidal precipitation*, the precipitation of each record follows a sine wave over the record's interval instead of a constant rate, with the same total.
