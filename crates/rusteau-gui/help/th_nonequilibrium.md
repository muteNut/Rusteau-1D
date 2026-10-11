# Non-equilibrium solute transport
keywords: non-equilibrium transport; chemical non-equilibrium; physical non-equilibrium; two-site sorption; kinetic sorption; mobile immobile; two-region; dual porosity; dual non-equilibrium; mass transfer coefficient

Non-equilibrium transport describes early breakthrough and long tailing that the equilibrium advection–dispersion equation cannot reproduce [@simunekVG2008]. Rusteau offers the models of HYDRUS-1D.

## Two-site sorption (chemical non-equilibrium) {#twosite}

The sorption sites are divided into two fractions [@selim1976; @vanGenuchtenWagenet1989]: on type-1 sites (fraction f) sorption is instantaneous, on type-2 sites it is kinetic (first order):

$$
s^{e} = f K_{s} c^{β} / (1 + η c^{β})
∂s^{k}/∂t = α [ (1 − f) K_{s} c^{β}/(1 + η c^{β}) − s^{k} ] − φ_{k}
$$

with the first-order rate coefficient α [1/T] and the degradation φ_{k} on the kinetic sites. It is selected with f < 1 ([transport parameters](solute_params)). f = 0 gives a one-site kinetic sorption model.

## Mobile–immobile water (physical non-equilibrium) {#mim}

The liquid phase is divided into a mobile (flowing) region θ_{mo} and an immobile (stagnant) region θ_{im} [@vanGenuchtenWierenga1976]. Solutes are exchanged by diffusion at the rate α [1/T]:

$$ ∂(θ_{im} c_{im})/∂t + ∂((1 − f) ρ s_{im})/∂t = α (c − c_{im}) − φ_{im} $$

where f is the fraction of sorption sites in contact with the mobile water. It is selected with θ_{im} > 0. With the [dual-porosity flow models](th_dual_porosity) the immobile water content is transient and the water transfer carries solute as well.

## Dual non-equilibrium {#dual}

Physical and chemical non-equilibrium combined: mobile–immobile water and, in the mobile region, equilibrium (fraction f_{em}) and kinetic sorption sites with the rate ω_{S} [@simunekVG2008]. In the reaction table, μ_{g} holds f_{em} and γ_{g} holds ω_{S}, as in HYDRUS-1D.

## Dual permeability {#dualperm}

Advection–dispersion in both the fracture and the matrix domain, coupled by the solute transfer Γ_{s} = α_{s}(1 − w)θ_{m}(c_{f} − c_{m}) + Γ_{w}c^{*} ([dual permeability](th_dual_permeability)) [@gerkeVG1993a].

## Initial condition

With *Non-equilibrium phase initially in equilibrium* the initial kinetic or immobile concentrations are set in equilibrium with the liquid concentrations; otherwise they are given on the page [Soil profile](profile_editor) (*Non-eq. conc.*).

## Typical behaviour

- small α: the non-equilibrium phase hardly takes part (early breakthrough, as with less sorption);
- large α: approaches equilibrium transport;
- intermediate α: asymmetric breakthrough curves with tailing.
