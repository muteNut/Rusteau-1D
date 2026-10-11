# Numerical solution
keywords: numerical solution; finite elements; Galerkin; Picard iteration; mass conservative; time step control; interpolation tables; Crank-Nicolson; upstream weighting; artificial dispersion; Thomas algorithm

Rusteau uses the numerical methods of HYDRUS-1D [@simunek2009]: Galerkin-type linear finite elements in space and finite differences in time.

## Water flow {#water}

The Richards equation is discretized with linear finite elements on the nodes of the profile, with a lumped (diagonal) mass matrix, and with a fully implicit finite difference scheme in time [@vogel1987]. The non-linearity is resolved by the mass-conservative modified Picard iteration of Celia et al. [@celia1990]: the time derivative is written in terms of the water content,

$$ (θ_{i}^{j+1,k+1} − θ_{i}^{j}) / Δt = C_{i}^{j+1,k} (h_{i}^{j+1,k+1} − h_{i}^{j+1,k}) / Δt + (θ_{i}^{j+1,k} − θ_{i}^{j}) / Δt $$

(j time level, k iteration), which conserves mass much better than the head form. Each iteration solves a tridiagonal system (Thomas algorithm). The iteration ends when the changes of θ (unsaturated nodes) and h (saturated nodes) are below the tolerances ([iteration criteria](iteration)). The first estimate of the heads of a new time step is extrapolated from the two previous time levels.

## Time step control {#timestep}

Three time discretizations are used: one for the numerical solution, one for the boundary conditions (the records) and one for the output (the print times). The time step of the solution adapts automatically:

1. it always ends at the times of the boundary records and the print times;
2. it stays between Δt_{min} and Δt_{max};
3. after a step that converged in at most ItMin iterations (3) it is multiplied by dMul (1.3), after a step with at least ItMax iterations (7) by dMul2 (0.7);
4. when the iteration does not converge within MaxIt iterations, the step is repeated with Δt/3; below Δt_{min} the run stops.

The solute and heat transport can further limit the step (Courant number, see [discretization](th_discretization)). As in HYDRUS, the time step is handled in single precision so that the time levels agree with HYDRUS.

## Interpolation tables {#tables}

To save time, θ(h), K(h) and C(h) of every material are tabulated at 100 logarithmically spaced heads between hTab1 and hTabN and interpolated linearly in log \|h\| [@vogel1987]; outside this interval the functions are evaluated directly ([iteration criteria](iteration#tables)).

## Solute and heat transport {#solute}

The advection–dispersion and heat transport equations are solved after the water flow of each time step with Galerkin linear finite elements and a weighted time scheme (ε = 0.5: Crank–Nicolson; 1: implicit) [@huyakornPinder1983]. With non-linear sorption the solution is iterated. To limit oscillations at sharp fronts, upstream weighting [@christie1976] or artificial dispersion can be used: the artificial dispersion is added where the product of the Peclet and Courant numbers exceeds the stability criterion [@perrochetBerod1993]

$$ Pe · Cr ≤ ω_{s} = 2 $$

## Mass balance {#balance}

At every print time the water and solute balance of the whole profile and of the sub-regions is computed from the nodal values and the cumulative boundary fluxes ([mass balance](res_balance)); its error measures the accuracy of the numerical solution.
