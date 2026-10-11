# Dual-permeability model
keywords: dual permeability; fractures; macropores; matrix; preferential flow; Gerke van Genuchten; water transfer; solute transfer

The dual-permeability model of Gerke and van Genuchten [@gerkeVG1993a] assumes that the porous medium consists of two overlapping domains, both of which conduct water: the **fracture** (macropore, inter-aggregate) domain and the **matrix** (micropore, intra-aggregate) domain. Each domain has its own Richards equation, coupled by a water transfer term.

## Governing equations

$$
∂θ_{f}/∂t = ∂/∂x [ K_{f}(h_{f}) (∂h_{f}/∂x + cos α) ] − S_{f} − Γ_{w}/w
∂θ_{m}/∂t = ∂/∂x [ K_{m}(h_{m}) (∂h_{m}/∂x + cos α) ] − S_{m} + Γ_{w}/(1 − w)
θ = w θ_{f} + (1 − w) θ_{m}
$$

The subscripts f and m refer to the fracture and the matrix domain, and w is the ratio of the volume of the fracture domain to the total soil volume. Each domain has its own van Genuchten–Mualem functions: in the parameter table of [Materials & parameters](soil_params) the primary columns describe the fractures and the extra columns θ_{r,m}, θ_{s,m}, α_{m}, n_{m}, K_{s,m} the matrix.

## Water transfer

The water transfer is proportional to the difference of the pressure heads [@gerkeVG1993a; @gerkeVG1993b]:

$$
Γ_{w} = α_{w} (h_{f} − h_{m})
α_{w} = (β / a^{2}) γ_{w} K_{a},   K_{a} = [K_{a}(h_{f}) + K_{a}(h_{m})] / 2,   K_{a}(h) = K_{a,s} K_{r,m}(h)
$$

with the shape factor β of the matrix blocks (3 for rectangular slabs, 15 for spheres [@gerkeVG1996]), the effective diffusion path length a (the half width of the blocks), the empirical scaling coefficient γ_{w} = 0.4 [@gerkeVG1993b], and the conductivity K_{a} of the fracture–matrix interface (K_{a,s} = 0 uses the matrix K_{s}). A positive Γ_{w} moves water from the fractures into the matrix.

## Boundary fluxes

A prescribed flux at a boundary is distributed between the two domains, in proportion to their volumes (f = w) or with a given fraction entering the fractures ([soil hydraulic model](hyd_model#dualperm)).

## Dual porosity inside the matrix

Optionally the matrix itself has a mobile and an immobile region, with a water-content or pressure-head driven transfer as in the [dual-porosity models](th_dual_porosity).

## Solute transport

The advection–dispersion equation is solved in both domains, coupled by the solute transfer

$$ Γ_{s} = α_{s} (1 − w) θ_{m} (c_{f} − c_{m}) + Γ_{w} c^{*} $$

with the solute transfer coefficient α_{s} [1/T] and the concentration c^{*} of the transferred water (c_{f} for flow into the matrix, c_{m} otherwise) [@gerkeVG1993a; @simunekVG2008].

> The interface parameters of the dual-permeability model cannot be stored in HYDRUS-1D project files. Non-equilibrium water flow, the centrifuge and density-dependent flow are not available with this model.
