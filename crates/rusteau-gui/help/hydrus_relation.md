# Relation to HYDRUS-1D
keywords: HYDRUS-1D; differences; compatibility; verification; precision; HYDRUS 4.17; port

Rusteau-1D ports the numerical engine of HYDRUS-1D (Fortran source version 4.08, 2009) [@simunek2009] to Rust. Its results are compared with HYDRUS-1D 4.17 [@simunek2013] for all examples of the HYDRUS-1D distribution. Where HYDRUS 4.17 behaves differently from the 2009 source, Rusteau follows 4.17.

## Compatibility

- HYDRUS-1D 4.x projects (`name.h1d` and the folder `name` with `SELECTOR.IN`, `PROFILE.DAT`, `ATMOSPH.IN`, `METEO.IN`, `FIT.IN`, `MATER.IN`, `Hysteresis.in`) are read and written ([projects](projects)).
- The input pages follow the dialogs of HYDRUS-1D 4.17; the names of the HYDRUS variables (e.g. `hCritA`, `TolTh`, `iKappa`) are given in the tooltips and the help so that the HYDRUS manual can be used alongside.
- Settings HYDRUS-1D cannot store are listed when a HYDRUS-1D project is written.

## Verification against HYDRUS 4.17 {#verification}

All columns of `T_Level.out`, `A_Level.out`, `Nod_Inf.out`, `Obs_Node.out`, `Balance.out`, `Solute<i>.out`, `Meteo.out` and `Irrig.out` are compared with the output of HYDRUS 4.17, with the print precision of the reference files as tolerance.

| Result | Examples |
|---|---|
| identical within print precision | 1SCALING, 2NOHYSTR, 3LAIJURI, 3SELIM, 4HEAT, 5SEASON, DRAINAGE, TEST3, TEST4, TEST5, TEST9a, Test9, Test10, UPINFIL |
| known differences | 1DRAINAG, 1INFILTR, 2HYSTER, EnBal2b, ROOTUPTK, TEST1, TEST2, TEST11, VOLATILE |

End-of-run water and solute totals agree with HYDRUS 4.17 within 0.07 % in all examples. The known differences come from the single precision of HYDRUS (iteration counts near the tolerance limits change the time steps), from chaotic situations (onset of ponding with a coarse head tolerance, a dry surface oscillating between evaporation and zero flux), and from the conductivity of the top node at a hysteresis reversal after ponding, where 4.17 differs from its own manual and source. The README of the project lists them in detail.

## Precision

HYDRUS-1D stores pressure heads, water contents, conductivities and temperatures in single precision; Rusteau computes in double precision throughout. The time-step control emulates the single-precision time step of HYDRUS, so that the time levels match. The hydraulic properties are interpolated from internal tables as in HYDRUS ([numerical solution](th_numerics#tables)).

## Parameter estimation

The source code of the inverse solution is not part of the 2009 Fortran source. Its algorithm was reconstructed from the manual and from the `Fit.out` files of HYDRUS 4.17; the objective function, the weighting, the statistics and the first Marquardt iteration reproduce 4.17. By default Rusteau iterates in stages on frozen time levels from several starting points, which reaches the same or a lower objective than 4.17 in all examples; the HYDRUS iteration is available ([theory](th_inverse)).

## Additions of Rusteau

- The [further options](further_options) of the HYDRUS code that HYDRUS-1D does not offer (non-equilibrium water flow, centrifuge, density-dependent flow, enhancement factor, extrapolation).
- Rosetta 3 besides the Rosetta 1 models of Rosetta Lite ([Rosetta](rosetta)).
- A restart state with conductivities for hysteresis ([soil hydraulic model](hyd_model#restart)).
- Irrigation for the full duration ([water flow boundary conditions](water_bc#irrigation)).
- The native project format `.h1dr`, CSV output, measured data shown with the results.

## Not supported

HP1 (coupling with PHREEQC), UNSATCHEM (major-ion chemistry) and the carbon dioxide transport are separate modules beside the HYDRUS-1D core and are not included.
