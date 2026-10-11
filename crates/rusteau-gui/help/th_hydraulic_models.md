# Soil hydraulic property models
keywords: van Genuchten; Mualem; Brooks Corey; Kosugi; Durner; modified van Genuchten; air entry value; retention curve; conductivity function; effective saturation; temperature dependence

The unsaturated soil hydraulic properties θ(h) and K(h) are described with closed-form equations [@simunek2009]. In all models the effective saturation is

$$ S_{e} = (θ − θ_{r}) / (θ_{s} − θ_{r}) $$

with the residual and saturated water contents θ_{r} and θ_{s}.

## van Genuchten – Mualem {#vg}

van Genuchten [@vanGenuchten1980] used the statistical pore-size distribution model of Mualem [@mualem1976] to obtain a predictive equation for the unsaturated conductivity from the retention function:

$$
θ(h) = θ_{r} + (θ_{s} − θ_{r}) / [1 + \|αh\|^{n}]^{m}   for h < 0;   θ(h) = θ_{s}   for h ≥ 0
K(h) = K_{s} S_{e}^{l} [1 − (1 − S_{e}^{1/m})^{m}]^{2},   m = 1 − 1/n,   n > 1
$$

α [1/L], n and l are empirical parameters; l is the pore-connectivity parameter, estimated as 0.5 by Mualem [@mualem1976] ([the parameter l](soil_params#l)).

## van Genuchten with an air-entry value of −2 cm {#airentry}

For fine-textured soils with n close to 1 the van Genuchten–Mualem conductivity drops steeply just below saturation, which makes the numerical solution unstable and the predictions poor. Vogel et al. [@vogel2001] and Schaap and van Genuchten [@schaapVG2006] introduced a small minimum capillary height h_{s} = −2 cm: the retention curve is the van Genuchten function with the fictitious saturated water content θ_{m} chosen so that θ(h_{s}) = θ_{s},

$$ θ_{m} = θ_{r} + (θ_{s} − θ_{r}) (1 + \|α h_{s}\|^{n})^{m} $$

and the conductivity follows the modified model below with θ_{a} = θ_{r}, θ_{k} = θ_{s}, K_{k} = K_{s}.

## Modified van Genuchten (Vogel and Císlerová) {#modified}

Vogel and Císlerová [@vogelCislerova1988] added flexibility near saturation with a non-zero air-entry value h_{s}:

$$
θ(h) = θ_{a} + (θ_{m} − θ_{a}) / (1 + \|αh\|^{n})^{m}   for h < h_{s};   θ(h) = θ_{s}   for h ≥ h_{s}
K(h) = K_{s} K_{r}(h)   for h ≤ h_{k};   K(h) = K_{k} + (h − h_{k})(K_{s} − K_{k})/(h_{s} − h_{k})   for h_{k} < h < h_{s};   K(h) = K_{s}   for h ≥ h_{s}
K_{r} = (K_{k}/K_{s}) (S_{e}/S_{ek})^{l} [ (F(θ_{r}) − F(θ)) / (F(θ_{r}) − F(θ_{k})) ]^{2}
F(θ) = [1 − ((θ − θ_{a})/(θ_{m} − θ_{a}))^{1/m}]^{m},   S_{ek} = (θ_{k} − θ_{r})/(θ_{s} − θ_{r})
$$

θ_{m} ≥ θ_{s} and θ_{a} ≤ θ_{r} are fictitious extrapolations of the retention curve; K_{k} is a measured conductivity at the water content θ_{k} (h_{k} = h(θ_{k})). With θ_{a} = θ_{r}, θ_{m} = θ_{k} = θ_{s} and K_{k} = K_{s} the original van Genuchten–Mualem model results.

## Brooks and Corey {#bc}

$$
S_{e} = \|αh\|^{−n}   for h < −1/α;   S_{e} = 1   for h ≥ −1/α
K = K_{s} S_{e}^{2/n + l + 2}
$$

[@brooksCorey1964]; here n is the pore-size distribution index (λ) and 1/α the air-entry (bubbling) pressure head.

## Kosugi (log-normal) {#kosugi}

Kosugi [@kosugi1996] assumed a log-normal pore-size distribution; with Mualem's model:

$$
S_{e} = ½ erfc[ ln(h/h_{0}) / (√2 σ) ]   for h < 0
K = K_{s} S_{e}^{l} { ½ erfc[ ln(h/h_{0}) / (√2 σ) + σ/√2 ] }^{2}
$$

In the parameter table α holds h_{0} (the median pressure head) and n holds σ (the standard deviation of ln \|h\|), as in HYDRUS.

## Durner (dual porosity) {#durner}

Durner [@durner1994] divided the pore space into two overlapping regions, each with a van Genuchten–Mualem function; their linear superposition describes multimodal pore systems:

$$
S_{e} = w_{1} [1 + (α_{1} h)^{n_{1}}]^{−m_{1}} + w_{2} [1 + (α_{2} h)^{n_{2}}]^{−m_{2}}
K(S_{e}) = K_{s} (w_{1} S_{e1} + w_{2} S_{e2})^{l} ( w_{1} α_{1} [1 − (1 − S_{e1}^{1/m_{1}})^{m_{1}}] + w_{2} α_{2} [1 − (1 − S_{e2}^{1/m_{2}})^{m_{2}}] )^{2} / (w_{1} α_{1} + w_{2} α_{2})^{2}
$$

with the weights w_{1} = 1 − w_{2} and m_{i} = 1 − 1/n_{i}.

## Look-up tables {#tables}

Instead of a model, θ(h), K(h) and C(h) can be given as tables (file `Mater.in` of HYDRUS-1D); values between the tabulated heads are interpolated.

## Temperature dependence {#temperature}

With temperature-dependent hydraulic properties, the pressure head is scaled with the surface tension σ and the conductivity with the dynamic viscosity μ and the density ρ of water at the soil temperature T, relative to a reference temperature [@philipDeVries1957; @constantz1982]:

$$
h(T) = h(T_{ref}) σ(T) / σ(T_{ref})
K(T) = K(T_{ref}) [μ(T_{ref}) ρ(T)] / [μ(T) ρ(T_{ref})]
$$

The conductivity of water-saturated soil increases by about 2 % per °C, mostly through the viscosity.

## Typical values

The [soil catalog](soil_params#catalog) of Carsel and Parrish [@carselParrish1988] gives average parameters of the textural classes; [Rosetta](rosetta) predicts them from the texture [@schaap2001; @zhangSchaap2017].
