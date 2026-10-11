# Root solute uptake
keywords: root solute uptake; passive uptake; active uptake; Michaelis-Menten; nutrient uptake; compensation

Root solute (nutrient) uptake follows Šimůnek and Hopmans [@simunekHopmans2009] as the sum of a passive and an active part.

## Passive uptake {#passive}

Solutes taken up passively move with the water into the roots, up to a maximum concentration c_{max} (cRoot):

$$ P_{a}(x, t) = S(x, t) min[c(x, t), c_{max}] $$

with the root water uptake S. c_{max} = 0 means no passive uptake; a large c_{max} means that the solute is taken up with the water at its full concentration.

## Active uptake {#active}

The potential active uptake A_{p} is the part of the potential (demand) uptake rate that the passive uptake does not cover:

$$ A_{p}(t) = max[ S_{pot} ω_{w} − ∫ P_{a} dx, 0 ] $$

where S_{pot} is the potential solute uptake rate [M/L²/T] (with time-variable boundary conditions from the cBot column of the solute) and ω_{w} the water stress index when *reduced potential solute uptake due to reduced water uptake* is on (otherwise 1). It is distributed over the root zone with Michaelis–Menten kinetics:

$$ A_{a}(x, t) = b(x) A_{p}(t) (c − c_{min}) / (K_{m} + c − c_{min}) $$

with the Michaelis–Menten constant K_{m} and the minimum concentration c_{min} below which no active uptake occurs. As for water, the active uptake can be compensated: with the solute stress index ω_{s} = ∫ A_{a} dx / A_{p} and the critical index ω_{s,c}, the uptake is divided by max(ω_{s}, ω_{s,c}).

The total uptake P_{a} + A_{a} is a sink term of the [solute transport](th_solute) equation of the solute. HYDRUS-1D 2009 allows active uptake of a single solute only; HYDRUS 4.17 and Rusteau let you choose the solute [@simunek2016].
