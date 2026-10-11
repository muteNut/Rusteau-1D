# Solute transport: temperature and water-content dependence, colloids
keywords: temperature dependence; activation energy; Arrhenius; water content dependence; Walker; MoistDep.in; colloid attachment; virus; bacteria; filtration theory; blocking; straining

The page appears when the temperature dependence, the water-content dependence of reactions or the colloid model is switched on (page [Solute transport: general](solute_general)). It corresponds to the dialogs *Temperature Dependence of Reaction Parameters*, *Water Content Dependence of Reaction Parameters* and the attachment parameters of the *Virus, Colloid, Bacteria Transport Model* of HYDRUS-1D.

## Activation energies

Per solute, the activation energy Ea [J/mol] of every transport and reaction parameter (Dw, Dg, Ks, η, β, Henry, μ, γ, μ0, ω). A parameter a at temperature T is a(T) = a(T_{ref}) exp[Ea (T − T_{ref}) / (R T T_{ref})] with the reference temperature 20 °C [@stummMorgan1981]; 0 = no temperature dependence. Typical activation energies of degradation are 40–80 kJ/mol ([theory](th_reaction_dependence#temperature)).

## Water-content dependence

**Walker's formula** [@walker1974]: rate = rate_{ref} (θ/θ_{ref})^{B} for θ < θ_{ref}, with the exponent B (≈ 0.7) and the reference pressure head h_{ref} whose water content is θ_{ref}, for each of the nine rate coefficients.

**Trapezoidal tables** (`MoistDep.in`): per material, solute and reaction, the rate factor is ReacMin0 below θ0, rises linearly to 1 at θ1, stays 1 up to θ2 and falls to ReacMin1 at θ3 (θ2 = 0: no reduction). HYDRUS-1D writes but does not use `MoistDep.in`; Rusteau uses it ([theory](th_reaction_dependence#water)).

## Colloid attachment parameters

| Parameter | Description |
|---|---|
| rKa1, rKa2 | attachment coefficients of the two sites [1/T], or sticking efficiencies with filtration theory |
| rKd1, rKd2 | detachment coefficients [1/T] |
| SMax1, SMax2 | maximum solid-phase concentrations (blocking) |
| iPsi1, iPsi2 | blocking function: 0 none, 1 Langmuir, 2 ripening, 3 random sequential adsorption, 4 straining, 5 straining and blocking |
| Dc, Dp | diameters of the sand grains and the colloids (filtration theory, straining) [L] |
| ω immobile (matrix) | dual permeability: exchange of colloids with the immobile matrix water [1/T] |

See [colloid, virus and bacteria transport](th_colloids).
