# Notes on spatial and temporal discretization
keywords: discretization; mesh; element size; node spacing; Peclet number; Courant number; numerical oscillations; numerical dispersion; wetting front; solute front

The accuracy of the numerical solution depends on the size of the finite elements and the time steps. Following the HYDRUS-1D help [@simunek2009]:

## Water flow {#water}

- Use small elements where large hydraulic gradients occur: near the soil surface (infiltration, evaporation), at the interfaces of materials with very different properties, and around the water table. Elements of 0.1–1 cm near the surface of a soil with infiltration or evaporation are typical [@vanDamFeddes2000].
- Elements may grow with depth where the gradients are small; neighbouring elements should not differ in size by more than a factor of about 1.5. Use fixed points with relative element sizes on the page [Soil profile](profile_editor#discretization).
- Infiltration into very dry, fine-textured soil forms a sharp wetting front that needs fine elements and small time steps. Soils with n close to 1 benefit from the [air-entry value of −2 cm](th_hydraulic_models#airentry).
- Check the [mass balance](res_balance): relative errors above 1 % call for a finer mesh or smaller time steps.

## Solute and heat transport {#transport}

Two dimensionless numbers characterize the discretization of the advection–dispersion equation:

$$
Pe = q Δx / (θ D)   (grid Peclet number)
Cr = q Δt / (θ R Δx)   (Courant number)
$$

with the flux q, the element size Δx, the dispersion coefficient D, the retardation factor R and the time step Δt. Numerical oscillations are small when Pe ≤ 5 and Cr ≤ 1; oscillations can be virtually removed with Pe·Cr ≤ 2 [@perrochetBerod1993]. Rusteau, as HYDRUS, limits the time step so that the criterion Pe·Cr ≤ Pe_{crit} is met, and adds artificial dispersion or uses upstream weighting where it is not ([solute transport: general](solute_general)). Both reduce oscillations at the cost of numerical dispersion: the front is smeared more than with the real dispersivity.

Practical rules:

- choose Δx at most about twice the dispersivity (Pe ≤ 2);
- the maximum Peclet and Courant numbers of a run are shown on the page [Run-time information](res_runtime);
- with very small dispersivities (sharp fronts) refine the mesh rather than relying on artificial dispersion.

## Moving fronts

Wetting fronts and solute fronts move through the profile; a mesh that is fine only near the surface may be too coarse once the front reaches deeper layers. Uniform fine meshes are safe for short profiles; for deep profiles refine where the front is expected during the period of interest.
