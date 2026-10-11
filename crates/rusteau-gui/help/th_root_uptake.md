# Root water uptake
keywords: root water uptake; sink term; Feddes; S-shape; water stress; osmotic stress; salinity; compensated uptake; root distribution; root growth; logistic function; transpiration

## Sink term

The sink term S of the [Richards equation](th_richards) is the volume of water removed per unit time from a unit volume of soil by plant roots. Feddes et al. [@feddes1978] defined it as

$$ S(h) = α(h) S_{p} $$

where the water stress response function α(h) (0 ≤ α ≤ 1) reduces the potential uptake S_{p}. The potential uptake is the potential transpiration T_{p} distributed over the root zone with the normalized root distribution b(x):

$$
S_{p} = b(x) T_{p},   b(x) = b'(x) / ∫_{L_{R}} b'(x) dx
S(h, h_{φ}, x) = α(h, h_{φ}, x) b(x) T_{p}
$$

b'(x) is the root distribution of the [soil profile](profile_editor) (only relative values matter) and L_{R} the root zone. The actual transpiration is the integral of S over the root zone, T_{a} = ∫ S dx.

## Water stress response functions {#stress}

**Feddes et al. (1978)** [@feddes1978]: piecewise linear. Water uptake is zero close to saturation (h > h_{1} = P0, oxygen deficiency) and below the wilting point (h < h_{4} = P3); it is optimal between h_{2} = POpt and h_{3}, and varies linearly in between. h_{3} depends on the potential transpiration: h_{3} = P2H for T_{p} ≥ r2H, P2L for T_{p} ≤ r2L, and is interpolated linearly in between.

**S-shape (van Genuchten 1987)** [@vanGenuchten1987]:

$$ α(h) = 1 / [1 + (h/h_{50})^{p}] $$

with the pressure head h_{50} at which the uptake is halved and the exponent p (≈ 3). It does not account for oxygen stress.

## Osmotic (salinity) stress {#osmotic}

The osmotic head of the soil solution is h_{φ} = Σ a_{j} c_{j} with the osmotic coefficients a_{j} of the solutes. Following van Genuchten [@vanGenuchten1987]:

- **additive**: the osmotic head adds to the pressure head, α = α(h + h_{φ});
- **multiplicative**: α = α(h) α_{φ}(h_{φ}), with an S-shaped α_{φ} = 1/[1 + (h_{φ}/h_{φ50})^{p}] or the threshold–slope function of Maas and Hoffman [@maasHoffman1977; @maas1990]: no reduction up to a threshold, then a linear decrease with the slope.

## Compensated root water uptake {#compensation}

Roots may take up more water from less stressed parts of the root zone to compensate for stressed parts. With the water stress index ω (the ratio of actual to potential transpiration without compensation) and the critical stress index ω_{c} [@jarvis1989; @simunekHopmans2009]:

$$
ω = ∫_{L_{R}} α(h, h_{φ}, x) b(x) dx
S = α(h, h_{φ}, x) b(x) T_{p} / max(ω, ω_{c})
$$

For ω ≥ ω_{c} the uptake is fully compensated (T_{a} = T_{p}); below, T_{a} = T_{p} ω/ω_{c}. ω_{c} = 1 means no compensation.

## Root distribution {#distribution}

The root distribution b'(x) is set on the page [Soil profile](profile_editor), constant, linear, or with the functions of Hoffman and van Genuchten [@hoffmanVG1983] (constant in the upper 20 % of the rooting depth L_{r}, then linear to zero at L_{r}) and Vrugt et al. [@vrugt2001]:

$$ b(z) = (1 − z/z_{m}) exp[ −(p_{z}/z_{m}) \|z^{*} − z\| ] $$

with the maximum rooting depth z_{m}, the depth of maximum root density z^{*} and the shape parameter p_{z} (above z^{*}; p = 1 below).

## Root growth {#growth}

With root growth the root distribution follows the Hoffman–van Genuchten function of the current rooting depth L_{R}(t), which is read from the time-variable data, interpolated from a table, or given by the Verhulst–Pearl logistic growth function [@simunekSuarez1993]:

$$
L_{R}(t) = L_{m} L_{0} / [L_{0} + (L_{m} − L_{0}) e^{−r t}]
r = −1/(t_{m} − t_{0}) · ln[ L_{0}(L_{m} − L_{med}) / (L_{med}(L_{m} − L_{0})) ]
$$

with the initial rooting depth L_{0} at the sowing time t_{0}, the maximum rooting depth L_{m} and the known rooting depth L_{med} at the time t_{m}; t is the time since sowing. Before sowing and after harvest there are no roots; a period repeats the cycle.

See also: [root solute uptake](th_root_solute) · [page Root water and solute uptake](root_uptake_page)
