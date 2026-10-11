# Pedotransfer functions (Rosetta)
keywords: pedotransfer functions; PTF; Rosetta; neural network; bootstrap; hierarchical; texture; bulk density; UNSODA

Pedotransfer functions (PTFs) predict soil hydraulic parameters from more easily measured soil properties. Rosetta [@schaap2001] implements hierarchical PTFs: five models with increasing input – textural class (H1), sand, silt and clay (H2), plus bulk density (H3), plus the water content at 33 kPa (H4), plus the water content at 1500 kPa (H5) – each predicting the van Genuchten parameters θ_{r}, θ_{s}, α, n and K_{s}.

## Method

Each model is an ensemble of artificial neural networks trained on bootstrap samples of a database of soils with measured retention curves and conductivities. The prediction is the mean of the ensemble; the spread of the ensemble gives the standard deviation, an estimate of the uncertainty of the prediction. Model H1 is a look-up table of the class averages.

| Version | Training and features |
|---|---|
| Rosetta 1 | 60 (retention) and 100 (K_{s}) bootstrap networks; the models of Rosetta Lite in HYDRUS-1D [@schaap2001] |
| Rosetta 2 | Rosetta 1 with corrected retention parameters [@schaap2004] |
| Rosetta 3 | one network for all parameters, weighted recalibration with the uncertainty of the fitted retention curves, 1000 bootstrap members [@zhangSchaap2017] |

α, n and K_{s} are predicted as logarithms; their ensemble means are either the mean of the back-transformed values (arithmetic) or 10 to the mean of the logarithms (geometric). Rosetta also predicts the parameters K_{0} and L of a conductivity function with a matching point [@schaapLeij2000]; Rusteau shows them for information.

The network weights are those of the Python package [@rosettaSoil]; the unit tests of Rusteau reproduce its predictions within 2 × 10^{−5}.

## Limitations

The predictions are only as good as the similarity of the soil to the training data (mostly temperate agricultural soils from North America and Europe). Measured data or a [parameter estimation](th_inverse) are preferable when available; Rosetta predictions are good initial estimates for it.

See also: [the Rosetta dialog](rosetta) · [soil catalog](soil_params#catalog) [@carselParrish1988]
