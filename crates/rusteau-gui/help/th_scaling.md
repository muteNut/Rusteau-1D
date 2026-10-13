# Scaling of the hydraulic properties
keywords: scaling factors; similar media; spatial variability; alpha h; alpha K; alpha theta; Miller Miller

Spatial variability of the soil hydraulic properties within a material is described by a set of linear scaling transformations that relate the individual functions θ(h) and K(h) of a node to reference functions θ^{*}(h^{*}) and K^{*}(h^{*}) of its material [@vogel1991; @simunek2009]:

$$
θ(h) = θ_{r} + α_{θ} [θ^{*}(h^{*}) − θ_{r}]
K(h) = α_{K} K^{*}(h^{*})
h = α_{h} h^{*}
$$

with the three independent scaling factors α_{θ}, α_{K} and α_{h} of the node (the quantities *Scaling αθ, αK, αh* of the [soil profile](profile_editor)). The approach is a generalization of the similar-media scaling of [@millerMiller1956], in which α_{h} and α_{K} follow from a single characteristic length λ (α_{h} = 1/λ, α_{K} = λ², α_{θ} = 1).

All factors equal to 1 mean no scaling. With hysteresis the scaling applies to both branches. Scaling factors are used, for example, to describe stochastic fields of hydraulic properties or gradual changes within a layer.
