# Temperature and water-content dependence of reactions
keywords: temperature dependence; Arrhenius; activation energy; water content dependence; Walker; moisture dependence; degradation; MoistDep

## Temperature dependence {#temperature}

The transport and reaction parameters (diffusion coefficients, sorption coefficients, Henry's constant, decay and production rates, mass-transfer coefficients) can depend on the temperature after the Arrhenius equation [@stummMorgan1981]:

$$ a_{T} = a_{r} exp[ E_{a} (T^{A} − T_{r}^{A}) / (R_{u} T^{A} T_{r}^{A}) ] $$

with the parameter value a_{r} at the reference absolute temperature T_{r}^{A} = 293.15 K (20 °C), the absolute temperature T^{A} [K], the universal gas constant R_{u} = 8.314 J/mol/K and the activation energy E_{a} [J/mol] of the parameter. The temperature comes from the [heat transport](th_heat), or is the constant initial temperature of each node.

## Water-content dependence {#water}

Degradation is often slower in dry soil. Following Walker [@walker1974], the reaction rates can be reduced below a reference water content:

$$ a(θ) = a_{r} (θ/θ_{ref})^{B}   for θ < θ_{ref};   a(θ) = a_{r}   otherwise $$

with the exponent B (≈ 0.7) and the reference water content θ_{ref} = θ(h_{ref}) at the reference pressure head h_{ref}, defined separately for each rate.

Alternatively, a **trapezoidal function** per reaction (file `MoistDep.in`) multiplies the rate: ReacMin0 below θ_{0}, a linear rise to 1 at θ_{1}, 1 up to θ_{2}, and a linear decrease to ReacMin1 at θ_{3} (oxygen limitation in wet soil). HYDRUS-1D writes the file but does not use it; Rusteau does.
