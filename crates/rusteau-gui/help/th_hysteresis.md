# Hysteresis
keywords: hysteresis; scanning curves; main drying curve; main wetting curve; air entrapment; Kool Parker; Lenhard; pumping effect; reversal point

The relation between water content and pressure head depends on whether the soil is wetting or drying. The water content on a drying branch is higher than on a wetting branch at the same head (ink-bottle effect, contact angle, entrapped air).

## Kool and Parker model {#koolparker}

HYDRUS incorporates hysteresis with the empirical model of [@scott1983], as modified by [@koolParker1987] to account for air entrapment. The **main drying curve** has the parameters θ_{r}, θ_{s}, α, n (and θ_{m}); the **main wetting curve** has the same θ_{r} and n and its own θ_{s}^{w}, α^{w} (often α^{w} ≈ 2α). With hysteresis in the conductivity, the main wetting curve also has K_{s}^{w}.

**Scanning curves** start at a reversal point (h_{Δ}, θ_{Δ}), where the direction of the change of the water content changes. They are scaled from the main curves so that they pass through the reversal point:

- A **drying scanning curve** is the main drying curve scaled towards θ_{r}:

$$ θ(h) = θ_{r} + a^{d} [θ^{d}(h) − θ_{r}],   a^{d} = (θ_{Δ} − θ_{r}) / (θ^{d}(h_{Δ}) − θ_{r}) $$

- A **wetting scanning curve** is the main wetting curve scaled so that it passes through the reversal point and reaches, at saturation, the water content θ_{s}^{*} that remains after air entrapment:

$$
θ(h) = θ_{r}^{*} + a^{w} [θ^{w}(h) − θ_{r}],   θ_{r}^{*} = θ_{s}^{*} − a^{w} (θ_{s}^{w} − θ_{r})
θ_{s}^{*} = θ_{s} − (θ_{s} − θ_{Δ}) / [1 + R (θ_{s} − θ_{Δ})],   R = 1/(θ_{s} − θ_{s}^{w}) − 1/(θ_{s} − θ_{r})
$$

The trapped-air relation for θ_{s}^{*} follows [@land1968], as in [@koolParker1987]: wetting from a dry state ends at θ_{s}^{w}, wetting from saturation ends at θ_{s}. With hysteresis in the conductivity, the scanning curves of K are scaled in the same way between K at the reversal point and K_{s}^{*}.

A node changes its branch when its water content changes in the opposite direction by more than the water content tolerance. The initial condition lies on the main drying or the main wetting curve (*Initially drying / wetting curve*), or on the scanning curves of a previous run ([restart](hyd_model#restart)).

## The pumping effect and the model of Lenhard et al. {#lenhard}

While simple to implement, the Kool–Parker model can suffer from a pumping effect: under repeated small wetting and drying cycles the hysteresis loops can drift to physically unrealistic parts of the retention function. The model of [@lenhard1991] and [@lenhardParker1992] eliminates pumping by keeping track of the historical reversal points: a scanning curve that returns to an earlier reversal point continues on the curve of that earlier reversal (closed scanning loops). *Hysteresis in the retention curve (no pumping, Lenhard et al.)* selects it.

## Parameters

The hysteresis parameters are entered on the page [Materials & parameters](soil_params): θ_{m} (drying), θ_{s}, α and K_{s} of the wetting branch. For the [parameter estimation](inverse_page) the constraints θ_{m} = θ_{s}, α^{w} = 2α and K_{s}^{w} = K_{s} are available.
