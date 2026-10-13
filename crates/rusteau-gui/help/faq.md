# Questions and problems
keywords: FAQ; troubleshooting; convergence; non-convergence; mass balance error; oscillations; slow simulation; problems; errors

## The calculation stops: the iteration does not converge {#convergence}

The Richards equation is strongly non-linear; convergence problems usually have one of these causes:

- **Unrealistic or extreme hydraulic parameters**, e.g. n close to 1 with a large α, or very contrasting layers. Check θ(h) and K(h) on the page [Soil hydraulic properties](res_soilprops). For fine soils try the van Genuchten model with an [air-entry value of −2 cm](th_hydraulic_models#airentry).
- **Infiltration into very dry soil** or **sudden ponding**: use a smaller initial time step and minimum time step, a finer mesh near the surface ([discretization](th_discretization)), or more iterations ([iteration criteria](iteration)).
- **Inconsistent initial and boundary conditions**, e.g. a constant head at the surface very different from the initial head of the top node.
- **Too large hCritA** with a coarse mesh at the surface during strong evaporation.

Raising the maximum number of iterations (e.g. to 20) and lowering dtMin often helps; check the time steps on the page [Run-time information](res_runtime).

## The mass balance error is large

Look at the relative error WatBalR on the page [Mass balance](res_balance). Reduce dtMax, tighten the water content tolerance (e.g. 0.0001), and refine the mesh where the gradients are large. Errors of a few per cent in the first time steps of a run that ends with a small error are usually harmless.

## Concentrations oscillate or become negative

The grid Peclet or Courant number is too large ([discretization](th_discretization#transport)). Refine the mesh, reduce dtMax, switch on artificial dispersion or upstream weighting, or use ε = 1 (implicit) ([Solute transport: general](solute_general)).

## The results differ from HYDRUS-1D

For the HYDRUS-1D examples the differences are documented ([relation to HYDRUS-1D](hydrus_relation)). Small differences come from the single precision of HYDRUS near the iteration tolerances; tightening the tolerances in both programs makes them agree better.

## The actual transpiration is smaller than the potential one

Water or osmotic stress reduces the uptake ([root water uptake](th_root_uptake)); compensated uptake (ωc < 1) reduces the effect. Also check that the root distribution is not zero and that the roots reach the moist soil.

## The run is slow

Long simulations with small time steps (frequent rain, dry soil, fine mesh) take longer. Reduce the number of print times and use *Short output*; thin the time-level output (*Store time-level info every n-th step*). Parameter estimation runs the model many times; fewer starting points speed it up.

## The program stopped unexpectedly {#crash}

An internal error while drawing a page or a figure shows a message in place of the page (*Try again* draws it again); an internal error of a calculation ends the run with a message in the status bar. Every such error is written with its details to the crash log `crash.log` in the folder `Rusteau-1D` of the user data (Windows: `%LOCALAPPDATA%\Rusteau-1D`, Linux: `~/.local/share/Rusteau-1D`, macOS: `~/Library/Application Support/Rusteau-1D`); please add it to a report.

A project with unsaved changes is copied to `recovery.h1dr` in the same folder once a minute. If the program did not close normally, the next start offers to open this copy (*Open the copy*) or to discard it. Save the recovered project under a name of your choice.

## Where are the results stored?

In memory with the project. Use *File → Export results (CSV)…* to write them ([output files](output_files)).

## How do I make a figure for a paper or a presentation?

Open the [figure editor](figures) (*Results → 📈 Figures*), start from a template, set the size to the column width of the journal (e.g. 85 or 170 mm), add axis titles with units (markup such as `cm^{3} cm^{−3}`), and export a PNG at 300 dpi or an SVG. To compare scenarios, keep the results of each run ([comparison of runs](run_comparison)).

## How do I cite the methods?

The help topics cite the publications behind every calculation; the topic [References](references) lists all of them, each with what Rusteau bases on it. For the numerical methods cite the HYDRUS-1D manual [@simunek2009; @simunek2013].
