# Parameter estimation
keywords: inverse problem; parameter estimation; objective function; Marquardt-Levenberg; weighting; confidence intervals; correlation; AIC; BIC; starting points; frozen time levels; Fit.in

The inverse problem estimates model parameters by minimizing the differences between observed and simulated values [@kool1987; @hopmans2002; @simunekHopmans2002]. The algorithm of HYDRUS 4.17 [@simunek2013] was reconstructed from its manual and output; the objective function, the weighting, the statistics and the first iteration reproduce HYDRUS 4.17.

## Objective function {#objective}

$$ Φ(b) = Σ_{i=1}^{n} ( W_{i} [ F_{i}^{*} − F_{i}(b) ] )^{2},   W_{i} = w_{i} v_{j} $$

with the vector b of the estimated parameters, the observations F_{i}^{*} (types 0–15, see [observations](inverse_page#observations)), the model values F_{i}(b), the weight w_{i} of the observation and the weight v_{j} of its measurement set j. The measurement sets are the observation types (retention points belong to the water contents). The set weights are:

- no internal weighting: v_{j} = 1;
- weighting by mean ratio: v_{j} = the smallest mean of the absolute values of all sets divided by the mean of set j;
- weighting by standard deviation: v_{j} = 1/(√n_{j} σ_{j}) with the standard deviation σ_{j} of set j.

Prior information on parameters (types 7–11) enters the same sum. The model values are taken at the observation times, which are fixed time levels of the simulation.

## Marquardt–Levenberg iteration {#marquardt}

Φ is minimized with the Marquardt–Levenberg method [@marquardt1963]: from the sensitivity (Jacobian) matrix J of the model values with respect to the parameters, computed with forward differences (1 % of each parameter), each iteration solves

$$ (J^{T} W J + λ D) Δb = J^{T} W r $$

for the change Δb of the parameters, with the residuals r, the diagonal D of J^{T}WJ and the Marquardt parameter λ (0.02 at the start, divided by 10 at the start of every iteration and multiplied by 10 after a failed step). Parameters are kept within their constraints. The iteration ends when no parameter changes by more than 1 %, after MIT iterations, or when Φ cannot be reduced further.

## Rusteau iteration {#rusteau}

With adaptive time steps in every run, Φ is not a smooth function of the parameters: a small change of a parameter changes the time steps, and Φ jumps by up to a few per cent. The derivatives then carry large errors, and the HYDRUS iteration can stop far from the minimum at a point that depends on the last bits of the arithmetic. Rusteau therefore (by default)

- iterates in **stages on frozen time levels**: within a stage all runs use the time levels of the run at the start of the stage, so Φ is smooth and the derivatives use increments of 0.1 %; a new stage starts from the end point of the previous one while that lowers Φ (computed with adaptive time steps) by more than 0.1 %;
- starts from **further starting points** (8 by default, in parallel threads), placed deterministically with a Halton sequence [@halton1960] within the constraints (log-uniformly over more than a decade) or within a factor of four of the initial estimates, and keeps the lowest Φ.

The HYDRUS 4.17 iteration (adaptive time steps in every run, one starting point) is available for comparisons. In all twelve inverse examples of HYDRUS-1D the Rusteau iteration ends at the same or a lower Φ than HYDRUS 4.17.

## Statistics {#statistics}

From the last iteration [@simunekHopmans2002]:

- the **covariance matrix** C = s² (J^{T}WJ)^{−1} with the mean square error s² = Φ/(n − p) (n observations, p parameters);
- the **standard errors** SE_{k} = √C_{kk} and the **95 % confidence intervals** b_{k} ± t_{0.975, n−p} SE_{k} (Student's t);
- the **correlation matrix** a_{kl} = C_{kl}/√(C_{kk}C_{ll}); \|a\| near 1 means that the two parameters cannot be estimated independently from the data;
- **R²** of the regression of fitted on observed values;
- ME = Σ W r / n, MAE = Σ \|W r\| / n, RMSE = √(Σ (W r)² / (n − 1));
- the information criteria **AIC** = n ln(Φ_{0}/n) + 2p [@akaike1974] and **BIC** = n ln(Φ_{0}/n) + p ln n [@schwarz1978] with the objective function Φ_{0} at the start of the last iteration (as HYDRUS 4.17 prints them).

## Good practice

- Estimate as few parameters as possible; fix those known from measurements.
- Combine different kinds of data (e.g. outflow and retention points, or pressure heads and water contents).
- Give physically sensible constraints, especially with several starting points.
- Check the correlation matrix and the confidence intervals ([results](res_inverse)).
