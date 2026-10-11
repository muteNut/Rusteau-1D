# Non-equilibrium water flow, centrifuge and density-dependent flow
keywords: non-equilibrium water flow; TauW; Ross Smettem; centrifuge; centrifugal acceleration; density-dependent flow; viscosity; Options.in

These options of the HYDRUS code are not offered by the HYDRUS-1D interface ([further options](further_options)).

## Non-equilibrium water flow {#nonequilibrium}

Following Ross and Smettem [@rossSmettem2000], the water content does not follow the equilibrium retention curve θ_{eq}(h) instantly but approaches it with the time constant τ_{w}:

$$ ∂θ/∂t = (θ_{eq}(h) − θ) / τ_{w} $$

which over a time step gives

$$ θ − θ_{old} = (θ_{eq}(h) − θ_{old}) (1 − e^{−Δt/τ_{w}}) $$

τ_{w} = 0 is equilibrium flow. The convergence test of the iteration uses the change of the equilibrium water content. The iteration gets harder: a smaller time step no longer increases the storage term, so strong infiltration may not converge.

## Centrifuge {#centrifuge}

In a centrifuge the gravity term cos α of the [Richards equation](th_richards) is replaced by the centrifugal acceleration relative to gravity:

$$ ω² (R + d) / g $$

with the angular velocity ω = 2π rpm/60, the distance R of the soil surface from the axis of rotation, the depth d below the surface and the gravitational acceleration g. Rusteau writes the centrifugal term in conservative form (the flux of each element with the acceleration at its midpoint). A column draining to a bottom at h = 0 reaches the analytical equilibrium

$$ h(d) = −ω²/(2g) [ (R + L)² − (R + d)² ] $$

(L the length of the column).

## Density-dependent flow {#density}

The density ρ and the dynamic viscosity μ of the solution depend on the liquid concentration c of solute 1:

$$ ρ/ρ_{0} = a_{0} + a_{1} c + a_{2} c² + a_{3} c³,   μ/μ_{0} = b_{0} + b_{1} c + b_{2} c² + b_{3} c³ $$

The conductivity scales with ρ/μ, the gravity term and the storage with ρ; HYDRUS solves ρ ∂θ/∂t = ∂/∂z[K(∂h/∂z + ρ)] − S while its mass balance sums ρθ, so the balance error equals the solution mass gained by changes of the density.
