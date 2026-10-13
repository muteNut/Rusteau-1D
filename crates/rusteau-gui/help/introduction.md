# Introduction to Rusteau-1D
keywords: introduction; capabilities; features; HYDRUS-1D; vadose zone; overview

Rusteau-1D is a program for simulating the one-dimensional movement of water, heat and multiple solutes in variably saturated porous media, such as the unsaturated (vadose) zone of a soil profile, laboratory columns or lysimeters. It solves the same equations with the same numerical methods as HYDRUS-1D [@simunek2009; @simunek2008; @simunek2016]: the numerical engine is a port of the HYDRUS-1D Fortran source (version 4.08) to Rust, and where HYDRUS-1D 4.17 behaves differently from that source, Rusteau follows 4.17 [@simunek2013].

## What the program computes

- **Variably saturated water flow** with the Richards equation, including a sink term for root water uptake ([theory](th_richards)). The soil hydraulic properties can be described by the models of van Genuchten–Mualem [@vanGenuchten1980; @mualem1976], modified van Genuchten [@vogelCislerova1988], [@brooksCorey1964], [@kosugi1996] and [@durner1994], or by look-up tables ([hydraulic models](th_hydraulic_models)).
- **Hysteresis** in the retention curve and the hydraulic conductivity [@koolParker1987], or without pumping [@lenhard1991] ([hysteresis](th_hysteresis)).
- **Non-equilibrium (preferential) flow** with dual-porosity and dual-permeability models [@simunek2003; @gerkeVG1993a] ([dual porosity](th_dual_porosity), [dual permeability](th_dual_permeability)).
- **Root water and solute uptake** with water and osmotic stress, compensation and active uptake [@feddes1978; @vanGenuchten1987; @simunekHopmans2009] ([root uptake](th_root_uptake)).
- **Atmospheric boundary conditions** from precipitation and potential evapotranspiration, or from meteorological data with the Penman–Monteith or Hargreaves equations [@allen1998; @hargreavesSamani1985], snow, interception and the surface energy balance [@saito2006] ([atmosphere](th_atmosphere), [energy balance](th_energy_balance)).
- **Heat transport** by conduction and convection, coupled with **water vapor flow** [@saito2006] ([heat](th_heat), [vapor](th_vapor)).
- **Solute transport** of several solutes with linear or non-linear sorption, decay chains, gas-phase diffusion, physical and chemical non-equilibrium, and colloid / virus transport [@simunekVG2008; @schijvenSimunek2002] ([solute transport](th_solute)).
- **Parameter estimation** (inverse problem) by the Marquardt–Levenberg method from measured data [@marquardt1963; @simunekHopmans2002] ([theory](th_inverse)).
- **Pedotransfer functions**: soil hydraulic parameters from the texture with Rosetta [@schaap2001; @zhangSchaap2017] ([Rosetta](rosetta)).

## Verification

All 23 examples of the HYDRUS-1D distribution and its 12 parameter-estimation examples, recalculated with HYDRUS 4.17, are part of the test suite of the program. Most examples agree with HYDRUS within the print precision of the output files; the remaining differences are understood and documented ([relation to HYDRUS-1D](hydrus_relation)). End-of-run water and solute totals agree within 0.07 %.

## The interface

The interface leads through a project in the order of the input: processes and units, soil, boundary conditions, uptake and transport, run ([workflow](workflow)). Projects are stored as Rusteau projects (`.h1dr`) and can be read from and written to HYDRUS-1D 4.x project folders, so a project made in Rusteau opens in HYDRUS-1D ([projects](projects)). Every input has a tooltip with its meaning, unit and typical values; F1 opens the help of the current page.

## Not included

HP1 (the coupling of HYDRUS with the geochemical code PHREEQC) and the UNSATCHEM major-ion chemistry module are separate codes beside the HYDRUS-1D core and are not part of Rusteau-1D. The carbon dioxide transport of UNSATCHEM is not included either.
