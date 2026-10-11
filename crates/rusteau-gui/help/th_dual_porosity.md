# Dual-porosity models
keywords: dual porosity; mobile immobile; non-equilibrium flow; preferential flow; mass transfer; immobile water; structured soils

Dual-porosity models assume that water flows only in one part of the pore space (inter-aggregate pores, fractures, the **mobile** region), while water in the other part (intra-aggregate pores, the rock matrix, the **immobile** region) does not move but exchanges water and solutes with the mobile region [@simunek2003; @simunekVG2008]. They describe non-equilibrium (preferential) flow in structured soils.

## Governing equations

$$
∂θ_{mo}/∂t = ∂/∂x [ K(h) (∂h/∂x + cos α) ] − S_{mo} − Γ_{w}
∂θ_{im}/∂t = −S_{im} + Γ_{w}
θ = θ_{mo} + θ_{im}
$$

θ_{mo} and θ_{im} are the water contents of the mobile and immobile regions, S_{mo} and S_{im} their sink terms and Γ_{w} the transfer rate of water from the mobile to the immobile region [1/T]. The hydraulic functions of the mobile region are van Genuchten–Mualem functions; the immobile region has its own retention parameters θ_{r,im}, θ_{s,im} (and α_{im}, n_{im}).

## Mass transfer driven by the water content {#water}

The transfer rate is proportional to the difference of the effective saturations of the two regions [@simunek2001; @simunek2003]:

$$ Γ_{w} = ω [S_{e}^{mo} − S_{e}^{im}] $$

with the first-order rate coefficient ω [1/T]. The transfer within a time step is limited so that the immobile region does not exceed saturation or fall below its residual water content.

## Mass transfer driven by the pressure head {#head}

The transfer is driven by the difference of the pressure heads of the two regions, with a conductivity of the interface evaluated with the van Genuchten parameters of the immobile region (α_{im}, n_{im}; ω takes the role of the saturated conductivity of the interface):

$$ Γ_{w} = ½ [K_{a}(h_{mo}) + K_{a}(h_{im})] (h_{mo} − h_{im}) $$

where h_{im} is the head of the immobile region at its current water content.

## Durner's model

The model of Durner [@durner1994] also has two pore regions, but in equilibrium: one Richards equation with a bimodal retention function ([Durner](th_hydraulic_models#durner)).

## Solute transport

With a dual-porosity flow model, solute transport uses the mobile–immobile formulation with an exchange proportional to the concentration difference plus the solute carried by the water transfer ([non-equilibrium transport](th_nonequilibrium#mim)).

See also: [dual-permeability model](th_dual_permeability)
