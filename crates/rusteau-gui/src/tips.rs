//! Tooltips: a description of every input of the interface, shown when the mouse rests on its label, its value or
//! its column heading. The keys are the texts of the labels; [`tip`] also finds a label with a unit in brackets
//! ("Dispersivity Dl (cm)") under the key without it ("Dispersivity Dl").
//!
//! Units in square brackets follow the HYDRUS convention: L length, T time, M mass, – dimensionless.

use egui::{Response, RichText, Ui};
use std::collections::HashMap;
use std::sync::OnceLock;

const TIPS: &[(&str, &str)] = &[
    // ------------------------------------------------------------------ Processes & units
    ("Project title", "Name of the project; shown in the title bar and written to the HYDRUS-1D files (Selector.in, DESCRIPT.TXT)."),
    ("Length unit", "Length unit of all inputs and results (mm, cm or m). Heads, depths and fluxes are given in this unit; α is in 1/L, Ks in L/T."),
    ("Time unit", "Time unit of all inputs and results (seconds … years). Times, time steps, rates and conductivities use it."),
    ("Mass unit", "Mass unit of the concentrations [M/L³]. Only a label: the program does not convert masses."),
    ("Convert all values when a unit changes (otherwise only the unit label changes)", "On: changing the length or time unit rescales every input (heads, fluxes, α, Ks, times, dispersivities, rate constants …). Off: only the unit label changes and the numbers stay as they are."),
    ("Water flow (uncheck for transport under steady flow from the initial condition)", "Solve the Richards equation for variably saturated water flow. Off: the water contents and fluxes of the initial condition stay constant (steady flow), e.g. for solute or heat transport only."),
    ("Vapor flow (coupled non-isothermal liquid & water vapor flow)", "Includes isothermal and thermal water vapor flow (Philip & de Vries) and the coupling of water flow and heat transport (Saito et al., 2006). Requires heat transport."),
    ("Root water uptake", "Water uptake by plant roots as a sink term S(h) of the Richards equation (Feddes or S-shape stress function). Enables the page \"Root water & solute uptake\" and the root distribution in the soil profile."),
    ("Root growth", "The rooting depth changes in time (from the atmospheric data, a table or the logistic growth function); otherwise the root distribution of the profile is fixed."),
    ("Heat transport", "Solves the heat transport equation (conduction and convection). Temperatures are needed for vapor flow, temperature-dependent hydraulic properties or solute parameters, and the surface energy balance."),
    ("Solute transport", "Solves the advection–dispersion equation for one or more solutes with sorption, decay, production and non-equilibrium transport."),
    ("Number of solutes:", "Number of solutes (1–11). Solute k can be produced from solute k−1 by first-order chain reactions (γ coefficients on the reaction page)."),
    ("Short output: write time-level information only at print times", "lShort: the time-level tables (T_Level, Run_Inf, Solute) get a line only at the print times instead of every time step. Smaller output for long simulations."),
    ("Meteorological ET (Penman-Monteith / Hargreaves)", "Computes the potential evaporation and transpiration from meteorological data (Meteo.in) with the Penman–Monteith or the Hargreaves equation, or drives the surface energy balance. Enables the page \"Meteorological data\"."),
    ("Inverse problem: estimate parameters (see Parameter estimation)", "lInverse: the calculation estimates parameters from measured data (Fit.in) instead of a single direct run. Settings on the page \"Inverse problem (Fit.in)\"."),
    ("Inverse problem: estimate parameters when the calculation is run", "lInverse: Run starts the parameter estimation (Marquardt–Levenberg) instead of a single direct run."),
    // ------------------------------------------------------------------ Time & output
    ("Initial time step Δt", "dt: first time step [T]. The time step then adapts to the number of iterations. Recommended: small, e.g. 0.001–0.01 of the time unit for infiltration problems."),
    ("Minimum time step", "dtMin [T]: the time step never falls below this value. If the iteration does not converge with dtMin the run stops."),
    ("Maximum time step", "dtMax [T]: upper limit of the time step. Smaller values give smoother output; the solute transport may limit the step further (Courant number)."),
    ("Multiplication factor (few iterations)", "dMul (> 1, default 1.3): the time step is multiplied by this factor when the water flow converged in fewer iterations than the lower optimal iteration range."),
    ("Multiplication factor (many iterations)", "dMul2 (< 1, default 0.7): the time step is multiplied by this factor when the water flow needed more iterations than the upper optimal iteration range."),
    ("Lower optimal iteration range", "ItMin (default 3): if the water flow converges in at most this many iterations, the next time step is increased by dMul."),
    ("Upper optimal iteration range", "ItMax (default 7): if the water flow needs at least this many iterations, the next time step is decreased by dMul2."),
    ("Initial time", "tInit [T]: time at the start of the simulation (time of the initial condition)."),
    ("Final time", "tMax [T]: time at the end of the simulation."),
    ("Print times", "Times [T] at which profiles (Nod_Inf), the mass balance (Balance) and the results pages store values. Time steps end exactly at the print times."),
    ("Generate equidistant print times", "Replaces the print times by 10 equidistant times between the initial and the final time."),
    ("Store time-level info every n-th step", "nPrStep: the time-level tables (T_Level, Run_Inf, Solute, Obs_Node) get a line every n-th time step."),
    ("Write extended flux table T_Level1 (potential/actual fluxes and cumulative values, lFluxOut)", "lFluxOut: writes the table T_Level1 with potential and actual surface fluxes and further cumulative values. Option of the HYDRUS code that the HYDRUS-1D interface does not offer."),
    ("Additionally print at regular intervals", "lPrintD: besides the print times, the time-level information is also written at regular time intervals."),
    ("Interval", "tPrintInt [T]: length of the regular print interval."),
    // ------------------------------------------------------------------ Iteration
    ("Maximum number of iterations", "MaxIt (default 10): maximum number of Picard iterations of the water flow in a time step. When it is exceeded, the time step is reduced to Δt/3 and the step is repeated."),
    ("Water content tolerance", "TolTh (default 0.001): the iteration of an unsaturated node has converged when its water content changes by less than this between two iterations."),
    ("Pressure head tolerance", "TolH [L] (default 1 cm): the iteration of a saturated node has converged when its pressure head changes by less than this between two iterations."),
    ("Lower limit of the tension interval (table)", "hTab1 [L] (positive, default 1e-4 cm … 1e-6 m): lower absolute pressure head of the internal interpolation tables of θ(h), K(h) and C(h). Outside the interval the functions are evaluated directly."),
    ("Upper limit of the tension interval (table)", "hTabN [L] (positive, default 1e4–1e5 cm): upper absolute pressure head of the internal interpolation tables."),
    // ------------------------------------------------------------------ Further options
    ("Start each time step from heads extrapolated from the two previous steps (lExtrap)", "On (HYDRUS default): the first estimate of the heads of a new time step is extrapolated linearly from the two previous steps; off: the heads of the previous step are used."),
    ("Non-equilibrium water flow (Ross & Smettem, 2000; TauW)", "The water content follows the equilibrium retention curve with a delay: θ − θold = (θeq(h) − θold)(1 − exp(−Δt/τw)). Not available with dual permeability."),
    ("Time constant τw", "τw [T]: time constant of the non-equilibrium water flow; 0 = equilibrium (no delay)."),
    ("Flow in a centrifuge (lCentrif)", "The gravity term becomes the centrifugal acceleration ω²(R + depth)/g, e.g. for centrifuge experiments."),
    ("Rotation speed", "Rotation speed of the centrifuge [rpm]. With a time-variable bottom boundary condition the hBot column gives the speed."),
    ("Distance of the soil surface from the axis", "R [L]: distance between the axis of rotation and the soil surface."),
    ("Density and viscosity depend on the concentration of solute 1 (lDensity)", "Density-dependent flow: the density and the viscosity of the solution are cubic polynomials of the liquid concentration of solute 1; K scales with ρ/μ, gravity and storage with ρ."),
    ("ρ(c)/ρ(0)", "Coefficients a0 … a3 of the relative density of the solution, ρ/ρ0 = a0 + a1 c + a2 c² + a3 c³ (HYDRUS: 1 + 0.75 c)."),
    ("μ(c)/μ(0)", "Coefficients a0 … a3 of the relative dynamic viscosity, μ/μ0 = a0 + a1 c + a2 c² + a3 c³ (HYDRUS: 1)."),
    ("Enhancement factor of the thermal vapor flux (Cass et al., 1984; iEnhanc)", "On (default): the thermal vapor flux is multiplied by the enhancement factor η(θ) of Cass et al. (1984); off: η = 1."),
    ("HYDRUS defaults", "Resets all further options to the defaults of the HYDRUS code."),
    // ------------------------------------------------------------------ Soil hydraulic model
    ("van Genuchten – Mualem", "θ(h) = θr + (θs − θr)/[1 + |αh|ⁿ]^m with m = 1 − 1/n and the Mualem conductivity K = Ks Se^l [1 − (1 − Se^(1/m))^m]² (van Genuchten, 1980). The most common model."),
    ("With air-entry value of −2 cm", "Modified van Genuchten function with a minimum capillary height hs = −2 cm (Vogel et al., 2001): improves the description of fine-textured soils with n close to 1 (SELECTOR.IN model 3)."),
    ("Modified van Genuchten (Vogel & Císlerová, 1988)", "van Genuchten function with extra flexibility near saturation: the retention curve reaches θs at a non-zero air-entry value; parameters θm, θa, θk and Kk."),
    ("Brooks – Corey", "Se = |αh|^(−n) below the air-entry head −1/α, K = Ks Se^(2/n + l + 2) (Brooks & Corey, 1964); n is the pore-size distribution index λ."),
    ("Kosugi (log-normal)", "Log-normal pore-size distribution (Kosugi, 1996): Se = Q[ln(h/hm)/σ]; α holds the median head hm, n the standard deviation σ."),
    ("Dual-porosity (Durner, dual van Genuchten – Mualem)", "Bimodal retention curve: weighted sum of two van Genuchten curves (Durner, 1994) with w2, α2, n2 for the second pore system; one Richards equation."),
    ("Dual-porosity (mobile–immobile, water-content mass transfer)", "Water flows in the mobile region; the immobile region exchanges water at the rate ω(Se,m − Se,im) (Šimůnek et al., 2003)."),
    ("Dual-porosity (mobile–immobile, head mass transfer)", "Water flows in the mobile region; the immobile region exchanges water driven by the difference of the pressure heads, with its own van Genuchten parameters."),
    ("Dual-permeability (Gerke & van Genuchten, 1993)", "Two overlapping domains (fractures / macropores and matrix) with a Richards equation each and a first-order water transfer between them."),
    ("Look-up tables of θ(h), K(h) and C(h) (Mater.in)", "The hydraulic properties are read from tables (Mater.in of a HYDRUS-1D project) instead of an analytical model."),
    ("No hysteresis", "One (drying) retention curve and conductivity function."),
    ("Hysteresis in the retention curve", "Kool & Parker (1987) hysteresis: main drying and main wetting curves with scanning curves; K(h) without hysteresis. Parameters θm, θs and α of the wetting branch."),
    ("Hysteresis in the retention curve and the conductivity", "Kool & Parker (1987) hysteresis in the retention curve and in K(h); the wetting branch also has its own Ks."),
    ("Hysteresis in the retention curve (no pumping, Lenhard et al.)", "Hysteresis model of Lenhard et al. (1991) and Lenhard & Parker (1992) that remembers the reversal points, so scanning loops cannot drift (\"pumping\")."),
    ("Initially drying curve", "The initial condition lies on the main drying curve (Kappa = −1)."),
    ("Initially wetting curve", "The initial condition lies on the main wetting curve (Kappa = +1)."),
    ("Temperature dependence of the hydraulic properties (surface tension, viscosity)", "The pressure head is scaled with the surface tension and the conductivity with the viscosity and density of water at the soil temperature (requires heat transport)."),
    ("Clear", "Removes the initial hysteresis state; the run starts on the main curve chosen above."),
    // ------------------------------------------------------------------ dual permeability
    ("Fracture volume fraction w (same for all layers):", "w [–]: volume of the fracture (macropore) domain relative to the total soil volume, 0 < w < 1."),
    ("Distribute boundary fluxes in proportion to the volume fractions (f = w)", "On: a prescribed boundary flux is split between the domains in proportion to their volumes. Off: give the fraction that enters the fractures."),
    ("Fraction of the boundary flux into the fractures:", "Fraction of a prescribed boundary flux that enters the fracture domain (0 = all into the matrix, 1 = all into the fractures)."),
    ("Dual porosity inside the matrix: transient immobile matrix water", "The matrix has a mobile and an immobile region; water and solutes exchange between them (dual porosity within dual permeability)."),
    ("water-content driven transfer (ωw)", "Mass transfer between mobile and immobile matrix water proportional to the difference of the effective saturations, rate ωw."),
    ("pressure-head driven transfer (van Genuchten immobile region)", "Mass transfer between mobile and immobile matrix water driven by the pressure-head difference; the immobile region has its own α and n."),
    ("β (3 = slabs)", "β [–]: shape factor of the matrix blocks (3 for rectangular slabs, 15 for spheres; Gerke & van Genuchten, 1996)."),
    ("γw (0.4)", "γw [–]: empirical scaling coefficient of the water transfer (0.4; Gerke & van Genuchten, 1993b)."),
    ("a (path length)", "a [L]: effective diffusion path length, e.g. the half width of the matrix blocks."),
    ("Kas (0 = matrix Ks)", "Ka,s [L/T]: saturated conductivity of the fracture–matrix interface; 0 uses the saturated conductivity of the matrix."),
    ("l (matrix)", "l [–]: pore-connectivity parameter of the matrix domain."),
    ("θr immobile (matrix)", "Residual water content of the immobile region of the matrix."),
    ("θs immobile (matrix)", "Saturated water content of the immobile region of the matrix."),
    ("α immobile", "α [1/L] of the immobile matrix region (pressure-head driven transfer)."),
    ("n immobile", "n [–] of the immobile matrix region (pressure-head driven transfer)."),
    ("Ks interface", "Saturated conductivity [L/T] of the interface between mobile and immobile matrix water (pressure-head driven transfer)."),
    ("ωw (water transfer)", "ωw [1/T]: rate coefficient of the water transfer between mobile and immobile matrix water."),
    // ------------------------------------------------------------------ Soil hydraulic parameters
    ("θr – residual water content", "θr (Qr) [–]: residual water content, the water content approached at very negative pressure heads. Typical 0–0.1."),
    ("θs – saturated water content", "θs (Qs) [–]: saturated water content, close to the porosity. Typical 0.3–0.55."),
    ("α", "α (Alpha) [1/L]: parameter of the retention function, roughly the inverse of the air-entry pressure head. Sand ≈ 0.145 1/cm, loam 0.036, clay 0.008."),
    ("n", "n [–] (> 1): shape parameter of the retention function, related to the width of the pore-size distribution. Sand ≈ 2.7, loam 1.56, clay 1.09."),
    ("α – inverse of air-entry value", "α [1/L] of Brooks & Corey: the inverse of the air-entry (bubbling) pressure head; Se = 1 for |h| < 1/α."),
    ("n – pore-size distribution index (λ)", "λ [–] of Brooks & Corey (here in the column n): pore-size distribution index; Se = |αh|^(−λ)."),
    ("hm – median pressure head (as α)", "Kosugi model: the median pressure head hm [L] (in the column α), the head at Se = 0.5."),
    ("σ – log-standard deviation (as n)", "Kosugi model: the standard deviation σ [–] of ln|h| of the pore-size distribution (in the column n)."),
    ("Ks – saturated hydraulic conductivity", "Ks [L/T]: saturated hydraulic conductivity. Sand ≈ 713 cm/d, loam 25 cm/d, clay 4.8 cm/d."),
    ("l – pore connectivity", "l [–]: pore-connectivity (tortuosity) parameter of the Mualem conductivity model; 0.5 after Mualem (1976). Rosetta often gives negative values."),
    ("θm", "θm [–]: fictitious extrapolated water content at h = 0 of the modified van Genuchten function (θm ≥ θs); θm = θs gives the original model."),
    ("θa", "θa [–]: fictitious water content used instead of θr in the modified van Genuchten function (θa ≤ θr)."),
    ("θk", "θk [–]: water content at which the conductivity Kk is known (modified van Genuchten; θk ≤ θs)."),
    ("Kk", "Kk [L/T]: measured conductivity at the water content θk (modified van Genuchten; Kk ≤ Ks)."),
    ("w2", "w2 [–]: weight of the second pore system of the Durner model (0 < w2 < 1)."),
    ("α2", "α2 [1/L]: α of the second pore system of the Durner model."),
    ("n2", "n2 [–]: n of the second pore system of the Durner model."),
    ("θr,im", "θr,im [–]: residual water content of the immobile region (dual porosity)."),
    ("θs,im", "θs,im [–]: saturated water content of the immobile region (dual porosity)."),
    ("ω", "ω [1/T]: first-order mass transfer coefficient between the mobile and the immobile region (dual porosity)."),
    ("α_im", "α [1/L] of the immobile region (dual porosity with pressure-head driven transfer)."),
    ("n_im", "n [–] of the immobile region (dual porosity with pressure-head driven transfer)."),
    ("θr,m", "θr of the matrix domain (dual permeability; the primary columns describe the fractures)."),
    ("θs,m", "θs of the matrix domain (dual permeability)."),
    ("α_m", "α [1/L] of the matrix domain (dual permeability)."),
    ("n_m", "n [–] of the matrix domain (dual permeability)."),
    ("Ks,m", "Ks [L/T] of the matrix domain (dual permeability)."),
    ("θm (drying)", "θm [–] of the main drying curve with hysteresis (θm ≥ θs; usually θm = θs)."),
    ("θs – wetting branch", "θs,w [–]: saturated water content of the main wetting curve (≤ θs, the difference is the entrapped air)."),
    ("α – wetting branch", "αw [1/L]: α of the main wetting curve; often about 2 α."),
    ("Ks – wetting branch", "Ks,w [L/T]: saturated conductivity of the main wetting curve (hysteresis in the conductivity)."),
    ("Name", "Name of the material (shown in the profile and the plots). HYDRUS-1D project files do not store it."),
    ("➕ Add material", "Adds a copy of the selected material with all its water, heat, solute and root parameters."),
    ("Load from catalog (Carsel & Parrish 1988):", "Average van Genuchten parameters of the USDA textural classes (Carsel & Parrish, 1988), converted to the project's units. Only rough averages of each class."),
    ("Neural network prediction (Rosetta)…", "Predicts θr, θs, α, n and Ks from the texture, the bulk density and the water contents at 33 and 1500 kPa with the Rosetta pedotransfer functions."),
    // ------------------------------------------------------------------ Water flow boundary conditions
    ("Upper boundary", "Boundary condition at the soil surface."),
    ("Lower boundary", "Boundary condition at the bottom of the profile."),
    ("Top flux (negative = infiltration)", "rTop [L/T]: constant flux across the surface; negative values enter the soil (infiltration), positive leave it (evaporation)."),
    ("Bottom flux (positive = upward)", "rBot [L/T]: constant flux across the bottom; positive values enter the profile from below, negative values drain it."),
    ("Constant potential transpiration", "rRoot [L/T]: potential transpiration with constant boundary conditions (without atmospheric data)."),
    ("hCritS – max. surface layer height before runoff", "hCritS [L]: maximum allowed pressure head (ponding depth) at the surface. Above it the water runs off; 0 = no ponding."),
    ("Triggered irrigation", "HYDRUS 4.17 option: irrigation starts when the pressure head at an observation node falls below a threshold (atmospheric boundary condition only)."),
    ("Observation node triggering irrigation (number in the observation-node list)", "Number of the observation node (1 = first in the list on the page Soil profile) whose pressure head triggers the irrigation."),
    ("Pressure head triggering irrigation", "hTrigger [L]: irrigation starts when the pressure head at the observation node falls below this (negative) value."),
    ("Irrigation rate", "Irrigation rate [L/T] (positive)."),
    ("Irrigation duration", "Duration of one irrigation [T]."),
    ("Lag time", "Time [T] between the trigger and the start of the irrigation."),
    ("Behaviour", "How the irrigation combines with the atmospheric records: as HYDRUS 4.17 (reproduces its results) or for the full duration on top of the precipitation."),
    ("Reference groundwater level GWL0", "GWL0L [L]: reference position of the groundwater table (usually the bottom of the profile) in the deep-drainage function."),
    ("Aqh", "Aqh [L/T]: coefficient A of the deep drainage flux q(h) = −Aqh exp(Bqh |h − GWL0L|) (Hopmans & Stricker, 1989)."),
    ("Bqh", "Bqh [1/L]: coefficient B of the deep drainage flux q(h) = −Aqh exp(Bqh |h − GWL0L|)."),
    ("Pressure head at the seepage face", "hSeep [L]: pressure head at which the seepage face starts to drain (usually 0). Below it the bottom is a no-flux boundary."),
    ("Drain geometry / configuration", "Position of the drains relative to the impervious layer and the soil layers, as in SWAP / HYDRUS (homogeneous profile on or above the impervious layer, layered profile with the drain at the interface, in the bottom or in the top layer)."),
    ("Elevation of the drain bottom (negative)", "zBotDr [L]: z coordinate of the drain bottom (negative below the surface)."),
    ("Drain spacing", "L [L]: distance between the parallel drains."),
    ("Entrance resistance", "Entrance resistance of the drains [T]."),
    ("Elevation of the impervious layer (negative)", "z coordinate [L] of the impervious layer below the drains."),
    ("Wet perimeter", "Wet perimeter of the drain [L]."),
    ("Kh above drain", "Horizontal saturated conductivity above the drain [L/T]."),
    ("Kh below drain", "Horizontal saturated conductivity below the drain [L/T]."),
    ("Kv top", "Vertical saturated conductivity of the top layer [L/T]."),
    ("Kv bottom", "Vertical saturated conductivity of the bottom layer [L/T]."),
    ("Depth of layer interface", "z coordinate [L] of the interface between the top and the bottom layer of the drain configuration."),
    ("Geometry factor", "Geometry factor of the Ernst equation (drain in the top layer)."),
    // ------------------------------------------------------------------ Time-variable boundary data
    ("Daily sinusoidal variation of evaporation/transpiration", "The daily potential evaporation and transpiration are distributed over the day with a sinusoidal function (night: 24 % of the mean rate; Fayer, 2000). Requires daily records."),
    ("Sinusoidal precipitation within each interval", "The precipitation of each record is distributed sinusoidally within its interval instead of a constant rate."),
    ("Snow accumulation and melt", "Precipitation falls as snow below −2 °C (mixed between −2 and +2 °C), accumulates and melts with a degree-day factor. Needs the air temperature (Ttop column or meteo data)."),
    ("Snow melt factor:", "Degree-day factor [L/T/°C] of the snow melt (HYDRUS default 0.43 cm/d/°C)."),
    ("Canopy interception", "Part of the precipitation is intercepted by the canopy (depends on the LAI) and evaporates; the transpiration is reduced by the evaporated interception."),
    ("Interception a:", "Interception constant a [L] of the interception capacity a · LAI."),
    ("Partition the potential ET by LAI (the transpiration column then holds the LAI)", "The Evap column holds the potential evapotranspiration ETp and the Transp column the LAI; Tp = ETp (1 − exp(−k LAI)), Ep = ETp − Tp (Ritchie, 1972)."),
    ("Extinction coefficient:", "k [–]: extinction coefficient of the radiation in the canopy (HYDRUS default 0.463)."),
    ("Repeat the atmospheric records cyclically after the last record", "After the last record the table starts again (e.g. one year of data repeated for many years)."),
    ("➕ Add record", "Adds a record (a copy of the last one, one time unit later)."),
    ("Import from text file…", "Reads records from a text or CSV file with the columns of the table (at least 8 columns: t, Prec, Evap, Transp, hCritA, rBot, hBot, hTop, then optional heat, solute and root columns)."),
    ("t", "End time [T] of the record. Each record holds from the end of the previous one (or the initial time) to this time."),
    ("Prec", "Precipitation (and irrigation) rate [L/T], positive."),
    ("Evap", "Potential evaporation rate [L/T], positive (with LAI partitioning: potential evapotranspiration). For variable head/flux conditions: the code KodTop of the record."),
    ("Transp", "Potential transpiration rate [L/T], positive (with LAI partitioning: the leaf area index)."),
    ("hCritA", "hCritA [L]: absolute value of the minimum pressure head allowed at the surface (dry limit of evaporation), e.g. 10000–100000 cm."),
    ("rBot", "Flux [L/T] across the bottom (time-variable flux condition); positive upward."),
    ("hBot", "Pressure head [L] at the bottom (time-variable head condition)."),
    ("hTop", "Pressure head [L] at the surface (time-variable head condition)."),
    ("Ttop", "Temperature [°C] at the surface (air temperature for the snow model)."),
    ("Tbot", "Temperature [°C] at the bottom."),
    ("Ampl", "Amplitude [°C] of the daily sinusoidal surface temperature."),
    ("cTop", "Concentration [M/L³] of the solute in the water entering at the surface (precipitation, irrigation)."),
    ("cBot", "Concentration [M/L³] at the bottom (or the potential solute uptake rate of the actively taken-up solute)."),
    ("xRoot", "Rooting depth [L] (root growth from the atmospheric data)."),
    // ------------------------------------------------------------------ Meteorological data
    ("Potential Radiation (Ra computed)", "No radiation data: the solar radiation is estimated from the potential radiation Ra and the relative sunshine duration (Ångström)."),
    ("Solar Radiation (measured Rs)", "The records give the measured global (short-wave) radiation Rs [MJ/m²/d]."),
    ("Net Radiation (measured Rn)", "The records give the measured net radiation Rn [MJ/m²/d]."),
    ("Sunshine Hours", "The last column gives the hours of sunshine n; n/N with the day length N."),
    ("Cloudiness Index", "The last column gives a cloudiness index (converted to n/N with the long-wave cloud factors a1, b1)."),
    ("Transmission Coeff. / Cloud Cover", "The last column gives the transmission coefficient of the atmosphere."),
    ("Derived from Solar Radiation", "The cloudiness factor is derived from the measured solar radiation, f = ac Rs/Rso + bc."),
    ("Relative Humidity (%)", "The humidity column gives the mean relative humidity [%]."),
    ("Vapor Pressure (kPa)", "The humidity column gives the actual vapor pressure [kPa]."),
    ("Use Hargreaves equation instead of Penman-Monteith", "ETp = 0.0023 Ra (Tmean + 17.8) √(Tmax − Tmin) (Hargreaves & Samani, 1985): needs only temperatures."),
    ("Evaporation and heat flux from the surface energy balance (requires heat transport)", "The evaporation and the surface heat flux follow from the surface energy balance Rn − H − λE − G = 0 with the surface temperature (Saito et al., 2006); requires heat transport and usually vapor flow."),
    ("Generate daily variations of temperature and humidity from the daily records", "lMetDaily: from daily records (Tmax, Tmin, …) hourly air temperatures and humidities are generated; the time step is limited to one hour."),
    ("Latitude (° N+, S-)", "Latitude of the site [decimal degrees], north positive, south negative (potential radiation, day length)."),
    ("Altitude (m)", "Altitude above sea level [m] (atmospheric pressure, psychrometric constant)."),
    ("Ångström a (shortwave)", "as [–]: fraction of the extraterrestrial radiation reaching the ground on overcast days, Rs = (as + bs n/N) Ra (FAO-56: 0.25)."),
    ("Ångström b (shortwave)", "bs [–]: Rs = (as + bs n/N) Ra; as + bs is the fraction on clear days (FAO-56: 0.50)."),
    ("Longwave cloud factor a1", "a1 [–]: cloudiness factor of the net long-wave radiation, f = a1 n/N + b1 (Doorenbos & Pruitt, 1977: 0.9)."),
    ("Longwave cloud factor b1", "b1 [–]: cloudiness factor of the net long-wave radiation, f = a1 n/N + b1 (0.1)."),
    ("Longwave emissivity al", "al [–]: net emissivity ε = al + bl √ea (FAO-56: 0.34)."),
    ("Longwave emissivity bl", "bl [kPa^-0.5]: net emissivity ε = al + bl √ea (FAO-56: −0.14)."),
    ("Cloudiness from Solar ac", "ac [–]: cloudiness factor from the solar radiation, f = ac Rs/Rso + bc (FAO-56: 1.35)."),
    ("Cloudiness from Solar bc", "bc [–]: f = ac Rs/Rso + bc (FAO-56: −0.35)."),
    ("Wind measurement height (cm)", "Height [cm] of the wind speed measurement (aerodynamic resistance); must exceed the zero-plane displacement 0.67 × crop height."),
    ("Temp. measurement height (cm)", "Height [cm] of the temperature and humidity measurement."),
    ("Bare Soil", "No crop: only the evaporation from the soil is computed."),
    ("Constant crop", "Crop height, albedo and LAI are constant (values below)."),
    ("Crop growth table", "Crop height, albedo, LAI and root depth are interpolated from a table of times."),
    ("Daily crop data in the records", "Each meteorological record carries its own crop height, albedo, LAI and root depth."),
    ("From Height (Clipped Grass)", "LAI from the crop height for clipped grass (LAI = 0.24 h, h in cm)."),
    ("From Height (Alfalfa)", "LAI from the crop height for alfalfa (LAI = 5.5 + 1.5 ln h)."),
    ("Direct Input / Surface Fraction", "The LAI is given directly; a value < 1 is read as the soil cover fraction SCF, LAI = −ln(1 − SCF)/k."),
    ("Crop height (cm)", "Height of the crop [cm] (roughness, zero-plane displacement, LAI)."),
    ("Albedo", "Albedo of the crop surface [–] (FAO-56 reference grass: 0.23)."),
    ("Input LAI / Surface Fraction", "Leaf area index [–], or the soil cover fraction if < 1."),
    ("Radiation Extinction Coeff.", "k [–]: extinction coefficient for the partitioning of the radiation between canopy and soil, SCF = 1 − exp(−k LAI)."),
    ("Interception constant a (mm):", "Interception constant a [mm] of the interception capacity a · LAI (von Hoyningen-Hüne, 1983; Braden, 1985)."),
    ("Root depth (0 = take it from the root-growth settings):", "Rooting depth [L] of the constant crop; 0 uses the root growth settings."),
    ("Rad (MJ/m²)", "Radiation of the record [MJ/m²/d]: global (solar) or net radiation, as selected above."),
    ("T Max (°C)", "Maximum air temperature of the record [°C]."),
    ("T Min (°C)", "Minimum air temperature of the record [°C]."),
    ("RH Mean (%)", "Mean relative humidity [%]."),
    ("Vapor (kPa)", "Actual vapor pressure [kPa]."),
    ("Wind (km/d)", "Wind speed [km/d] at the measurement height."),
    ("Sun Hours (h)", "Hours of bright sunshine of the day."),
    ("Cloudiness", "Cloudiness index of the record."),
    ("Transm. Coeff", "Transmission coefficient of the atmosphere."),
    ("Solar Factor", "Not used with \"Derived from Solar Radiation\" (the cloudiness follows from the radiation)."),
    ("Height (cm)", "Crop height [cm]."),
    ("LAI", "Leaf area index [m²/m²]."),
    ("Root depth", "Rooting depth [L]."),
    // ------------------------------------------------------------------ Root uptake
    ("Feddes et al. (1978)", "Piecewise-linear stress response α(h): zero above P0 and below P3, one between POpt and P2, linear in between."),
    ("S-shape (van Genuchten 1987)", "α(h) = 1/(1 + (h/P50)^p): smooth reduction around the head P50 of 50 % uptake."),
    ("No solute stress", "The salinity (osmotic head) does not reduce the water uptake."),
    ("Additive", "The osmotic head a·c is added to the pressure head: α(h + hφ) (van Genuchten, 1987)."),
    ("Multiplicative", "The water stress and the osmotic stress multiply: α(h)·α(hφ)."),
    ("threshold", "Osmotic stress after Maas & Hoffman (1977): no reduction above the threshold, then a linear decrease (slope)."),
    ("S-shape", "S-shaped osmotic stress α(hφ) = 1/(1 + (hφ/c50)^p)."),
    ("Critical stress index for water uptake ωc (1 = no compensation)", "ωc [–]: compensated uptake (Jarvis, 1989; Šimůnek & Hopmans, 2009): if the water stress index ω = ∫ α b dz is above ωc, unstressed roots compensate the stressed ones. 1 = no compensation."),
    ("P0", "P0 [L]: above this pressure head roots take up no water (oxygen deficiency, anaerobiosis). Typical −10 cm."),
    ("POpt", "POpt [L]: below this head the uptake is optimal (may differ between materials). Typical −25 cm."),
    ("P2H", "P2H [L]: head below which the uptake decreases, for a high potential transpiration (≥ r2H). Typical −200 to −400 cm."),
    ("P2L", "P2L [L]: head below which the uptake decreases, for a low potential transpiration (≤ r2L). Typical −800 to −1000 cm."),
    ("P3", "P3 [L]: wilting point – below this head no uptake. Typical −8000 cm."),
    ("r2H", "r2H [L/T]: potential transpiration rate belonging to P2H (typical 0.5 cm/d)."),
    ("r2L", "r2L [L/T]: potential transpiration rate belonging to P2L (typical 0.1 cm/d)."),
    ("P50 – pressure head of 50 % uptake", "P50 [L]: pressure head at which the uptake is halved (S-shape function)."),
    ("p – exponent", "p [–]: exponent of the S-shape function (≈ 3)."),
    ("c50 – osmotic head of 50 % uptake", "c50 [L]: osmotic head at which the uptake is halved (S-shaped multiplicative osmotic stress)."),
    ("Threshold osmotic head", "Osmotic head [L] below which (in absolute value) the uptake is not reduced (threshold–slope function)."),
    ("Slope (relative reduction per unit osmotic head)", "Relative reduction of the uptake per unit osmotic head beyond the threshold."),
    ("Osmotic coefficients (osmotic head per unit concentration):", "aOsm [L⁴/M]: osmotic head per unit concentration of each solute, hφ = Σ aOsm,j cj."),
    ("cRoot", "Maximum concentration [M/L³] of passive root solute uptake: the passive uptake is S·min(c, cRoot). 0 = no passive uptake; a large value = uptake with the water."),
    ("Active solute uptake (Michaelis-Menten)", "The roots take up one solute actively with Michaelis–Menten kinetics, compensating the passive uptake (Šimůnek & Hopmans, 2009)."),
    ("Solute with active uptake", "Solute taken up actively (HYDRUS 4.17)."),
    ("Potential solute uptake rate", "Potential (demand) rate of the solute uptake [M/L²/T]; with time-variable boundary conditions taken from the cBot column of the solute."),
    ("Michaelis-Menten constant", "Km [M/L³]: concentration of half the maximum active uptake rate."),
    ("Minimum concentration for uptake", "cMin [M/L³]: below this concentration no active uptake."),
    ("Critical stress index for active solute uptake", "Critical stress index of the active solute uptake (compensation as for water; 1 = none)."),
    ("Reduced potential solute uptake due to reduced water uptake", "The potential active uptake is reduced in proportion to the water stress."),
    ("Rooting depth from atmospheric data", "The rooting depth is read from the xRoot column of the time-variable data."),
    ("Table (time, depth)", "The rooting depth is interpolated from a table of times and depths."),
    ("Logistic (Verhulst–Pearl)", "Rooting depth from the logistic growth function (Šimůnek & Suarez, 1993) between sowing and harvest."),
    ("Sowing time", "Time [T] when the growth starts (initial root depth)."),
    ("Time of known root depth", "Time [T] at which the root depth is known (determines the growth rate)."),
    ("Harvest time", "Time [T] of harvest: the roots disappear."),
    ("Initial root depth", "Rooting depth [L] at sowing."),
    ("Root depth at the known time", "Rooting depth [L] at the time of known root depth."),
    ("Maximum root depth", "Maximum rooting depth [L]."),
    ("Period (1e30 = no cycling)", "Length [T] of the growth cycle; the growth repeats with this period (e.g. 365 d). 1e30 = one cycle."),
    // ------------------------------------------------------------------ Heat
    ("Material", "Number of the material (soil layer type)."),
    ("θn", "θn [–]: volume fraction of the solid (mineral) phase, ≈ 1 − porosity."),
    ("θo", "θo [–]: volume fraction of organic matter."),
    ("λL", "λL [L]: longitudinal thermal dispersivity (thermal conductivity increases with the water flux)."),
    ("b1", "b1: coefficient of the thermal conductivity λ0(θ) = b1 + b2 θ + b3 θ^0.5 (Chung & Horton, 1987) [M L T⁻³ K⁻¹]; with Campbell: quartz fraction."),
    ("b2", "b2: coefficient of λ0(θ) = b1 + b2 θ + b3 θ^0.5; with Campbell: fraction of other minerals."),
    ("b3", "b3: coefficient of λ0(θ) = b1 + b2 θ + b3 θ^0.5; with Campbell: clay fraction."),
    ("Cn", "Cn: volumetric heat capacity of the solid phase [M L⁻¹ T⁻² K⁻¹] (1.92 MJ/m³/K)."),
    ("Co", "Co: volumetric heat capacity of organic matter (2.51 MJ/m³/K)."),
    ("Cw", "Cw: volumetric heat capacity of water (4.18 MJ/m³/K)."),
    ("Campbell (1985) thermal conductivity instead of Chung & Horton (1987): b1/b2/b3 = quartz, other minerals, clay fractions", "The thermal conductivity follows Campbell (1985) from the volume fractions of quartz, other minerals and clay (given in b1, b2, b3)."),
    ("Default heat capacities", "Cn = 1.92, Co = 2.51, Cw = 4.18 MJ/m³/K (solid phase, organic matter, water) for all materials, converted to the project's units."),
    ("Temperature", "First-type (Dirichlet) condition: the temperature is prescribed."),
    ("Heat flux", "Third-type (Cauchy) condition: the heat flux of the entering water with the given temperature."),
    ("Zero gradient", "Zero temperature (or concentration) gradient at the bottom: heat (solute) leaves only by convection."),
    ("Upper temperature", "Temperature [°C] at the surface (or of the infiltrating water); overridden by the Ttop column of time-variable data."),
    ("Lower temperature", "Temperature [°C] at the bottom; overridden by the Tbot column."),
    ("Amplitude of the sinusoidal top temperature", "Amplitude [°C] of the sinusoidal daily variation of the surface temperature."),
    ("Period", "Period [T] of the sinusoidal surface temperature (1 day)."),
    // ------------------------------------------------------------------ Solute general
    ("Time weighting ε (0 explicit, 0.5 Crank–Nicolson, 1 implicit)", "Epsi: temporal weighting of the solute transport equation; 0.5 (Crank–Nicolson) is second-order accurate, 1 (fully implicit) is the most stable."),
    ("Max. iterations (non-linear)", "Maximum number of iterations of the solute transport per time step with non-linear sorption."),
    ("Absolute concentration tolerance", "cTolA [M/L³]: absolute tolerance of the iteration for non-linear sorption (0 = linear problems)."),
    ("Relative concentration tolerance", "cTolR [–]: relative tolerance of the iteration for non-linear sorption."),
    ("Stability criterion Pe_crit", "PeCr: stability criterion Pe·Cr ≤ PeCr (Perrochet & Berod, 1993); the time step is limited and artificial dispersion is added if needed. Default 2."),
    ("Upstream weighting finite elements (else Galerkin)", "Upstream weighting reduces oscillations at sharp fronts (numerical dispersion is added)."),
    ("Add artificial dispersion to stabilise (Pe > Pe_crit)", "lArtD: artificial dispersion is added where the Peclet–Courant criterion is not met."),
    ("Tortuosity factor", "The molecular diffusion is reduced by the tortuosity factor of the liquid and gas phases."),
    ("Millington & Quirk", "τw = θ^(7/3)/θs² (Millington & Quirk, 1961)."),
    ("Moldrup & Olesen", "Tortuosity of Moldrup et al. (1997, 2000)."),
    ("Non-equilibrium phase initially in equilibrium with the liquid phase", "lEquil: the initial kinetically sorbed (or immobile) concentrations are set in equilibrium with the liquid concentration."),
    ("Initial conditions given as total solute mass (per volume of soil)", "The initial concentrations of the profile are total masses per volume of soil (liquid + sorbed + gas) instead of liquid concentrations."),
    ("Volume averaged (resident)", "Concentrations are reported as resident (volume averaged) concentrations."),
    ("Flux averaged", "Concentrations are reported as flux-averaged concentrations (as measured in an effluent)."),
    ("Dual non-equilibrium: mobile-immobile model plus kinetic sorption sites in the mobile zone (needs θim > 0)", "Physical non-equilibrium (mobile–immobile) plus chemical non-equilibrium (kinetic sites) in the mobile zone."),
    ("Decay products of non-equilibrium sorbed mass go to the non-equilibrium phase (lNEquil)", "Daughter products of the kinetically sorbed mass enter the kinetic sites of the next solute instead of the liquid phase."),
    ("Colloid / virus / bacteria transport (attachment-detachment, two attachment sites)", "Colloid transport with two kinetic attachment–detachment sites, blocking and straining (Schijven & Šimůnek, 2002; Bradford et al., 2003)."),
    ("Compute attachment rates from filtration theory (Dc, Dp, sticking efficiencies)", "The attachment coefficients follow from filtration theory with the collector and particle diameters and sticking efficiencies."),
    ("Temperature dependence of transport and reaction parameters (activation energies)", "Diffusion, sorption and rate coefficients follow the Arrhenius equation with activation energies (page T / θ dependence)."),
    ("Water content dependent reaction rates", "Degradation rates depend on the water content (Walker, 1974, or trapezoidal tables)."),
    ("Walker: (θ/θref)^B below θref", "Rate = rate_ref (θ/θref)^B for θ < θref (Walker, 1974)."),
    ("Trapezoidal tables per reaction", "Rate factor as a trapezoidal function of the water content (MoistDep.in)."),
    // ------------------------------------------------------------------ Solute parameters
    ("Bulk density ρ", "ρ [M/L³]: bulk density of the soil (e.g. 1.5 g/cm³ — in the project's mass and length units)."),
    ("Dispersivity Dl", "DisperL [L]: longitudinal dispersivity; the dispersion coefficient is D = Dl |q|/θ + Dw τ. Typical 1/10 of the travel distance in the field, mm–cm in columns."),
    ("Fraction f (equilibrium sites)", "Frac [–]: fraction of the sorption sites in equilibrium with the liquid (two-site model); 1 = equilibrium sorption. With mobile–immobile water: fraction of sites in contact with mobile water."),
    ("Immobile water θim", "ThIm [–]: immobile water content of the mobile–immobile (two-region) model; 0 = no immobile water."),
    ("Immobile water θim (fracture)", "Immobile water content of the fracture domain (dual permeability)."),
    ("Immobile water θim (matrix)", "Immobile water content of the matrix domain (dual permeability)."),
    ("Dw (liquid)", "Dw [L²/T]: molecular diffusion coefficient in free water (e.g. 1.7 cm²/d … 2 cm²/d for small ions)."),
    ("Dg (gas)", "Dg [L²/T]: molecular diffusion coefficient in the soil air (volatile solutes)."),
    ("Solute", "Solute (species) number and name."),
    // ------------------------------------------------------------------ Reactions
    ("Mat", "Material number."),
    ("Ks", "Ks [L³/M]: adsorption coefficient (Kd for linear sorption) of s = Ks c^β/(1 + η c^β)."),
    ("η (Nu)", "η (Nu) [L³/M]: Langmuir coefficient of s = Ks c^β/(1 + η c^β); 0 = Freundlich / linear."),
    ("β", "β [–]: Freundlich exponent of s = Ks c^β/(1 + η c^β); 1 = linear."),
    ("Henry kH", "kH [–]: Henry's law constant, gas concentration g = kH c (volatile solutes)."),
    ("μw", "μw [1/T]: first-order decay constant in the liquid phase."),
    ("μs", "μs [1/T]: first-order decay constant in the solid (sorbed) phase."),
    ("μg", "μg [1/T]: first-order decay in the gas phase; with dual non-equilibrium: fraction f_em of equilibrium sites in the mobile zone."),
    ("γw", "γw [1/T]: first-order rate of production of this solute from the previous solute of the chain, liquid phase."),
    ("γs", "γs [1/T]: first-order chain production in the solid phase."),
    ("γg", "γg [1/T]: first-order chain production in the gas phase; with dual non-equilibrium: rate ωS of the kinetic sites."),
    ("μ0w", "μ'w [M/L³/T]: zero-order production in the liquid phase (negative: zero-order decay)."),
    ("μ0s", "μ's [1/T]: zero-order production in the solid phase."),
    ("μ0g", "μ'g [M/L³/T]: zero-order production in the gas phase."),
    ("α (rate)", "α (Omega) [1/T]: first-order rate coefficient of the non-equilibrium mass transfer (kinetic sorption sites or mobile–immobile exchange)."),
    ("αs", "αs [1/T]: solute transfer coefficient between fracture and matrix (dual permeability)."),
    // ------------------------------------------------------------------ colloids
    ("rKa1", "katt,1 [1/T]: attachment coefficient of site 1 (or sticking efficiency with filtration theory)."),
    ("rKd1", "kdet,1 [1/T]: detachment coefficient of site 1."),
    ("SMax1", "Smax,1 [–]: maximum solid-phase concentration of site 1 (blocking)."),
    ("rKa2", "katt,2 [1/T]: attachment coefficient of site 2."),
    ("rKd2", "kdet,2 [1/T]: detachment coefficient of site 2."),
    ("SMax2", "Smax,2 [–]: maximum solid-phase concentration of site 2."),
    ("iPsi1", "Blocking function of site 1: 0 none, 1 Langmuir, 2 ripening, 3 random sequential adsorption (RSA), 4 straining, 5 straining + blocking."),
    ("iPsi2", "Blocking function of site 2 (codes as iPsi1)."),
    ("Dc", "dc [L]: diameter of the sand grains (collectors), filtration theory and straining."),
    ("Dp", "dp [L]: diameter of the colloids (particles), filtration theory."),
    ("ω immobile (matrix)", "Mass-transfer coefficient [1/T] between mobile and immobile matrix water for colloids (dual permeability); 0 = immobile water inaccessible."),
    // ------------------------------------------------------------------ T / θ dependence
    ("B", "B [–]: exponent of Walker's formula, rate = rate_ref (θ/θref)^B."),
    ("href (L)", "href [L]: reference pressure head; θref = θ(href)."),
    ("ReacMin0", "Rate factor at (and below) θ0 (trapezoidal water-content dependence)."),
    ("θ0", "Water content below which the reaction factor is ReacMin0."),
    ("θ1", "Water content from which the reaction factor is 1."),
    ("θ2", "Water content up to which the reaction factor is 1 (0 = no reduction)."),
    ("θ3", "Water content at and above which the reaction factor is ReacMin1."),
    ("ReacMin1", "Rate factor at (and above) θ3."),
    ("Reaction", "Reaction coefficient the row applies to."),
    // ------------------------------------------------------------------ Solute BC
    ("Concentration BC", "First-type (Dirichlet) condition: the concentration at the boundary is prescribed."),
    ("Concentration flux BC", "Third-type (Cauchy) condition: the solute flux equals the water flux times the given concentration (recommended for the surface)."),
    ("Volatilization (gas boundary layer)", "Volatile solutes diffuse through a stagnant air layer of thickness dSurf to the atmosphere (Jury et al., 1983)."),
    ("Boundary layer thickness dSurf:", "dSurf [L]: thickness of the stagnant boundary layer at the surface."),
    ("Atmospheric gas concentration cAtm:", "cAtm [M/L³]: concentration of the solute in the atmosphere above the boundary layer."),
    ("Pulse duration (BC concentrations set to 0 afterwards, unless atmospheric)", "tPulse [T]: duration of the solute application; afterwards the boundary concentrations are 0 (only without time-variable boundary conditions)."),
    ("c top", "Concentration [M/L³] at the upper boundary (or of the infiltrating water)."),
    ("c bottom", "Concentration [M/L³] at the lower boundary."),
    // ------------------------------------------------------------------ profile
    ("cos α (1 = vertical flow)", "cos α [–]: cosine of the angle between the flow direction and the vertical axis; 1 = vertical, 0 = horizontal flow."),
    ("Initial condition as water content θ", "lInitW: the initial condition is given as water contents instead of pressure heads."),
    ("Constant", "The same value at all selected nodes."),
    ("Linear", "Values interpolated linearly between the top and the bottom of the selection."),
    ("Hydrostatic equilibrium", "h = h(bottom) − (z(bottom) − z)·cos α from the lowest selected node: no flow at the start."),
    ("Hoffman & van Genuchten (1983)", "Root distribution b(z) constant in the upper 20 % of the rooting depth Lr and decreasing linearly to zero at Lr."),
    ("Vrugt et al. (2001)", "Root distribution b(z) = (1 − z/zm) exp(−pz/zm |z* − z|)."),
    ("Profile depth", "Depth [L] of the soil profile."),
    ("Number of nodes", "Number of nodes of the finite element mesh (at most 1001)."),
    ("Rooting depth Lr", "Rooting depth [L] of the Hoffman & van Genuchten distribution."),
    ("Maximum rooting depth zm", "Maximum rooting depth [L] of the Vrugt distribution."),
    ("Depth of maximum intensity z*", "Depth [L] of the maximum root density (Vrugt)."),
    ("Shape parameter pz", "Shape parameter pz [–] of the Vrugt distribution (1 for the 1D function)."),
    ("Sub-regions = materials", "Sets the sub-region of every node to its material number, so that the mass balance is given per material."),
    // ------------------------------------------------------------------ Inverse
    ("Soil hydraulic parameters", "Estimate parameters of the soil hydraulic functions (θr, θs, α, n, Ks, l, and the hysteresis parameters)."),
    ("Solute transport parameters (solute 1)", "Estimate transport and reaction parameters of solute 1 (dispersivity, sorption, decay, non-equilibrium)."),
    ("Heat transport parameters", "Estimate the thermal parameters (λL, b1–b3, Cn, Co, Cw) from temperatures (HYDRUS 4.17)."),
    ("αw = 2 α", "Constraint: α of the wetting branch is twice α of the drying branch."),
    ("Ksw = Ks", "Constraint: the wetting branch has the same Ks."),
    ("No internal weighting", "All measurement sets have weight 1 (only the weights of the observations)."),
    ("Weighting by mean ratio", "Each measurement set is weighted by the ratio of the smallest mean of all sets to its own mean."),
    ("Weighting by standard deviation", "Each measurement set is weighted by 1/(√n σ) with its standard deviation σ."),
    ("Maximum number of iterations (MIT)", "MIT: maximum number of Marquardt–Levenberg iterations (0 = only the direct run with the initial estimates)."),
    ("Initial estimate", "Value of the parameter at the start of the estimation."),
    ("Minimum value", "Lower constraint of the parameter (0 = none)."),
    ("Maximum value", "Upper constraint of the parameter (0 = none)."),
    ("Fitted?", "Estimate this parameter (otherwise it keeps its initial value)."),
    ("#", "Number of the observation."),
    ("Type", "Observation type (HYDRUS Fit.in): 0 cumulative flux, 1 h, 2 θ, 3 flux, 4 concentration (or temperature), 5 retention θ(h), 6 K(h), 7–11 prior information, 12–15 profiles at print times."),
    ("Position", "Position of the observation: the observation node, the boundary (1 top, 2 bottom), the material or the print time, depending on the type."),
    ("Time / h / x", "Abscissa of the observation: the time, the pressure head (types 5, 6) or the depth (types 12–15)."),
    ("Value", "Observed value."),
    ("Weight", "Weight of the observation in the objective function."),
    ("+ Add observation", "Adds an observation (copy of the last one)."),
    ("Paste observations", "Appends lines \"x value type position weight\" from the clipboard (as in Fit.in)."),
    ("Create from the current project", "Creates inverse settings with the materials and parameters of the project; add observations afterwards."),
    ("Import Fit.in…", "Reads the settings and observations of a HYDRUS-1D Fit.in file."),
    ("Export Fit.in…", "Writes the settings and observations as a HYDRUS-1D Fit.in file."),
    // ------------------------------------------------------------------ results
    ("Variable", "Quantity shown in the plot."),
    ("Several times", "Shows the profiles of the selected print times together."),
    ("Time slider", "Shows one print time at a time on fixed axes; the slider and ▶ step through the times."),
    ("log |h|", "Logarithmic axis of the absolute pressure head."),
    ("log y", "Logarithmic y axis (conductivity, capacity)."),
    ("main wetting curve (dashed)", "Also draws the main wetting curve (hysteresis)."),
    ("Load…", "Loads measured data (two columns) from a text or CSV file and shows them as points with the simulated curves."),
    ("Fitted against observed", "Scatter plot of fitted against observed values with the 1:1 line."),
    ("Against time / pressure head", "Observed (points) and fitted (line) values of one measurement set against time, pressure head or depth."),
    ("Apply the fitted parameters to the project", "Writes the fitted values into the materials and makes them the initial estimates of a new estimation."),
    ("Save Fit.out…", "Writes the results in the layout of the HYDRUS-1D file Fit.out."),
    // ------------------------------------------------------------------ Rosetta
    ("Model", "Rosetta version: Rosetta 3 (Zhang & Schaap, 2017), Rosetta 2 or Rosetta 1 (Schaap et al., 2001)."),
    ("Textural class", "USDA textural class (model H1: class average parameters)."),
    // ------------------------------------------------------------------ boundary condition types
    ("Constant pressure head (from the initial condition at the top node)", "First-type condition: the pressure head of the top node keeps its initial value during the whole run."),
    ("Constant flux", "Second-type condition: a constant water flux across the boundary (value below)."),
    ("Atmospheric BC with surface layer", "Atmospheric condition: precipitation and potential evaporation from the time-variable data; the surface switches to a head condition when it saturates (water ponds up to hCritS, then runs off) or dries to hCritA (Neuman et al., 1974)."),
    ("Atmospheric BC with surface runoff", "Atmospheric condition without ponding: water that cannot infiltrate runs off immediately."),
    ("Variable pressure head", "First-type condition with a time-variable head (column hTop or hBot of the time-variable data)."),
    ("Variable flux (Evap − Prec)", "Second-type condition with a time-variable flux; at the surface the flux is Evap − Prec of the records."),
    ("Variable pressure head / flux (code in the evaporation column)", "Each record decides: KodTop in the Evap column, positive = head condition with hTop, negative = flux condition."),
    ("Constant pressure head (initial condition at the bottom node)", "First-type condition: the pressure head of the bottom node keeps its initial value (e.g. a constant groundwater table)."),
    ("Variable flux", "Second-type condition with the time-variable flux rBot of the records."),
    ("Free drainage", "Unit hydraulic gradient at the bottom (∂h/∂z = 0): the water leaves with the conductivity of the bottom node. For deep profiles with the groundwater far below."),
    ("Deep drainage: q = A·exp(B·|h − GWL0|)", "The bottom flux depends on the position of the groundwater table, q = −Aqh exp(Bqh |h − GWL0L|) (Hopmans & Stricker, 1989)."),
    ("Seepage face", "The bottom is a no-flux boundary while unsaturated and drains at the head hSeep once it saturates (e.g. lysimeters, laboratory columns)."),
    ("Horizontal drains", "The bottom flux is the discharge to parallel drains or ditches after Hooghoudt or Ernst, as in SWAP (van Dam et al., 1997)."),
    ("As HYDRUS 4.17 (the next atmospheric record ends it)", "Reproduces HYDRUS 4.17: the irrigation rate replaces the precipitation, and the next atmospheric record ends the irrigation."),
    ("Full duration, added to the precipitation", "The irrigation runs for its whole duration on top of the precipitation; no precipitation is lost."),
    ("Humidity specification:", "Whether the humidity column of the records holds the relative humidity or the vapor pressure."),
    ("Crop mode:", "How the crop (height, albedo, LAI, root depth) is described."),
    ("Leaf Area Index (LAI):", "How the leaf area index is obtained (from the crop height or given directly)."),
    ("Water uptake reduction model", "Stress response function α(h) that reduces the potential root water uptake in wet or dry soil."),
    ("Solute stress model", "How the osmotic head of the solutes reduces the water uptake."),
    ("Reported concentration:", "Resident (volume-averaged) or flux-averaged concentrations in the output."),
    // ------------------------------------------------------------------ soil profile editor
    ("Quantity:", "Nodal quantity shown in the profile and edited in the selection."),
    ("Top of the selection", "Value at the uppermost selected node (linear distribution)."),
    ("Bottom of the selection", "Value at the lowest selected node (linear distribution)."),
    ("h at the lowest selected node", "Pressure head [L] at the lowest selected node; the others follow hydrostatic equilibrium."),
    ("Sub-region", "Number of the sub-region of the mass balance; sub-regions do not change the simulation."),
    ("size above", "Relative size of the finite elements just above the fixed point."),
    ("size below", "Relative size of the finite elements just below the fixed point."),
    ("Add at depth", "Adds an observation node at the node nearest to this depth."),
    ("Add fixed point at", "Adds a fixed point (a depth that is always a node) of the discretization."),
    ("Only the selected nodes", "Lists only the selected nodes in the table."),
    // ------------------------------------------------------------------ results tables
    ("time", "Time [T] of the line."),
    ("water volume", "Volume of water in the profile (or sub-region) per unit area [L]."),
    ("WatBalT", "Absolute error of the water balance [L]: change of storage minus the cumulative boundary fluxes and sink terms since the start."),
    ("WatBalR", "Relative error of the water balance [%]: WatBalT relative to the larger of the change of storage and the cumulative fluxes. Below about 1 % is good."),
    ("mass", "Solute mass in the profile per unit area [M/L²]."),
    ("CncBalT", "Absolute error of the solute mass balance [M/L²]."),
    ("CncBalR", "Relative error of the solute mass balance [%]."),
    ("region", "Sub-region number (page Soil profile, quantity Sub-regions)."),
    ("length", "Length [L] of the sub-region."),
    ("inflow", "Rate of change [L/T] of the water volume of the sub-region (net inflow)."),
    ("mean h", "Mean pressure head [L] of the sub-region."),
    ("Parameter", "Estimated parameter (material, name)."),
    ("Water fluxes: rTop, vTop, vBot, vRoot", "rTop: potential surface flux; vTop: actual surface flux; vBot: actual bottom flux; vRoot: actual transpiration (root water uptake). Negative = into the soil at the top / out of the profile at the bottom (T_Level.out)."),
    ("Cumulative water fluxes", "Cumulative potential and actual surface flux, bottom flux and root water uptake since the start [L]."),
    ("Pressure heads: top, root zone, bottom", "Pressure head at the surface node, mean pressure head of the root zone and at the bottom node."),
    ("Surface: runoff, cumulative infiltration/evaporation", "Surface runoff rate and the cumulative infiltration and evaporation."),
    ("Soil water storage", "Volume of water in the whole profile per unit area [L]."),
    ("Temperature at top and bottom", "Temperatures of the surface and the bottom node."),
    ("Solute: boundary fluxes", "Solute fluxes across the surface and the bottom, and the root solute uptake."),
    ("Solute: cumulative fluxes", "Cumulative solute fluxes since the start."),
    ("Solute: concentrations top / bottom", "Liquid concentrations at the surface and at the bottom node."),
    ("Water content θ(h)", "Retention curve: water content against the absolute pressure head."),
    ("Hydraulic conductivity K(h)", "Unsaturated hydraulic conductivity against the absolute pressure head."),
    ("Hydraulic conductivity K(θ)", "Unsaturated hydraulic conductivity against the water content."),
    ("Water capacity C(h)", "Soil water capacity C = dθ/dh against the absolute pressure head."),
    ("Initial", "Initial estimate of the parameter (start 0)."),
    ("Fitted", "Value of the parameter at the end of the estimation."),
    ("S.E.", "Standard error of the fitted parameter, from the covariance matrix of the last iteration."),
    ("Lower 95 %", "Lower limit of the 95 % confidence interval (Student t with n − p degrees of freedom)."),
    ("Upper 95 %", "Upper limit of the 95 % confidence interval."),
    ("Objective function SSQ", "Weighted sum of squared residuals SSQ = Σ (Wi (observed − fitted))² at the end of the estimation."),
    ("R² (observed vs fitted)", "Coefficient of determination of the regression of the fitted on the observed values (weighted); 1 = perfect fit."),
    ("Mean weighted error ME", "ME = Σ Wi ri / n: mean of the weighted residuals (bias)."),
    ("Mean weighted absolute error MAE", "MAE = Σ |Wi ri| / n."),
    ("Root mean square weighted error RMSE", "RMSE = √(Σ (Wi ri)² / (n − 1))."),
    ("Akaike information criterion AIC", "AIC = n ln(SSQ/n) + 2p (Akaike, 1974): compares models with different numbers p of fitted parameters; lower is better."),
    ("Bayesian information criterion BIC", "BIC = n ln(SSQ/n) + p ln n (Schwarz, 1978): like AIC with a stronger penalty for parameters."),
    ("Start", "Number of the starting point (0 = the initial estimates)."),
    ("Final SSQ", "Objective function at the end of the iteration from this starting point."),
    ("Iterations", "Number of Marquardt–Levenberg iterations of this starting point."),
    ("End", "Why the iteration stopped."),
    ("Iteration", "Number of the Marquardt–Levenberg iteration (0 = initial estimates)."),
    ("SSQ", "Objective function (weighted sum of squared residuals)."),
    ("constraints:", "Constraints between the drying and wetting branches of the hysteresis model during the estimation."),
    ("Resident concentrations", "The concentrations of the objective function are resident (volume-averaged) liquid concentrations."),
    ("Log. resident concentrations", "Logarithms of the resident concentrations (for data over several orders of magnitude)."),
    ("Flux concentrations", "Flux-averaged concentrations (effluent)."),
    ("Total concentrations (liquid + solid)", "Total solute mass per volume of soil, liquid and sorbed."),
    ("Resident concentrations (mobile + immobile)", "Resident concentrations of mobile and immobile water together."),
    ("θs (drying) > θs (wetting)", "No constraint between the saturated water contents of the branches."),
    ("θm = θs", "Constraint θm = θs of the drying branch."),
    ("θm = θs(wetting) = θs", "Constraint: the same saturated water content for both branches."),
    // ------------------------------------------------------------------ Figure editor
    ("Figures", "Opens the figure editor in a window of its own: diagrams of the results, the input and measured data with titles, annotations and several panels, exported as PNG or SVG. The figures are saved with the project."),
    ("New figure", "Creates a figure from a template (profiles, observation nodes, fluxes, water balance, hydraulic properties, comparison of runs …). Everything can be changed afterwards."),
    ("Duplicate", "Copies the figure, e.g. to make a variant with other data or another style."),
    ("Delete figure", "Removes the figure from the project."),
    ("Export PNG", "Writes the figure as a PNG image with the size and resolution of the figure (the resolution is stored in the file, so word processors place it at its size)."),
    ("Export SVG", "Writes the figure as an SVG vector graphic (sharp at every size; editable in Inkscape or Illustrator). The size is given in millimetres."),
    ("Copy image", "Copies the figure as an image to the clipboard (paste it into a document or a presentation)."),
    ("Figure name", "Name of the figure in the list of figures and the default file name of the export. Not drawn."),
    ("Title", "Title above the figure; empty: no title. Markup: x_{s} subscript, x^{2} superscript, *italic*, **bold**."),
    ("Subtitle", "Second line under the title, in a smaller size and grey."),
    ("Caption", "Text under the figure (source, note), wrapped to the width of the figure. A line break starts a new line."),
    ("Size (mm)", "Width and height of the figure in millimetres: the size it has in a document (journals: about 85 mm for one column, 170 mm for two). Font sizes in points refer to this size."),
    ("Resolution (dpi)", "Dots per inch of the PNG export and the copied image (300 for print, 150 for screens). The preview and the SVG do not depend on it."),
    ("Panels (rows × columns)", "Number of panels (diagrams) in a grid, e.g. 1 × 2 for two diagrams side by side. Each panel has its own layers, axes, legend and annotations."),
    ("Theme", "Look of the panels: Grid (frame and light grid), Classic (axis lines left and bottom, as in many journals), Minimal (no frame), Dark (presentations)."),
    ("Palette", "Colours of the series without a fixed colour, in order. Okabe–Ito is safe for colour-blind readers; Viridis suits series in an order (print times, depths); Grey for black-and-white print."),
    ("Transparent background (PNG, SVG)", "Leaves out the background in the exported files (the preview keeps it)."),
    ("SVG: text as outlines", "Writes the text of the SVG as glyph outlines: the file looks the same everywhere without the fonts, but the text cannot be edited. Off: text elements (editable; Noto Sans should be installed where the file is opened)."),
    ("Font", "Font family of all texts of the figure. Noto Sans is bundled with the program and has all symbols (Greek letters, sub- and superscripts, mathematical signs); characters a system font lacks are taken from Noto Sans."),
    ("Bold title", "Draws the title in bold."),
    ("Title size (pt)", "Font size of the title in points (1 pt = 1/72 inch = 0.353 mm)."),
    ("Subtitle size (pt)", "Font size of the subtitle in points."),
    ("Caption size (pt)", "Font size of the caption in points."),
    ("Panel title size (pt)", "Font size of the panel titles in points."),
    ("Axis title size (pt)", "Font size of the axis titles in points."),
    ("Tick label size (pt)", "Font size of the numbers at the axes in points (also the default size of annotations)."),
    ("Legend size (pt)", "Font size of the legend entries in points."),
    ("Panel", "The panel whose settings are shown below. A click on a panel in the preview selects it, too."),
    ("Panel title", "Title above the panel, e.g. \"(a) Loam\". Markup as in the title."),
    ("Legend", "Position of the legend of the panel: inside a corner of the plot area, outside on the right or below, or none."),
    ("Visible", "Shows or hides the layer without removing it."),
    ("Add layer", "Adds a layer (a copy of the selected one): another variable, source or run in the same panel."),
    ("Data source", "Where the values come from: the results (boundary fluxes T_Level, observation nodes, profiles at the print times, mass balance …), the input (hydraulic properties, time-variable boundary data, initial condition), measured data, the parameter estimation or an imported data file."),
    ("x variable", "Column of the source for the horizontal axis; the menu groups the columns (time and position, water, cumulative fluxes, solutes, heat …)."),
    ("y variable", "Column of the source for the vertical axis."),
    ("Scale factors x, y", "Factors applied to the values, e.g. 10 to show cm as mm, −1 to make downward fluxes positive, 1/24 to show hours as days."),
    ("Items", "The observation nodes, print times, materials or data sets of the layer: all, or a selection. Each item is a series with its own colour and legend entry."),
    ("Runs", "The runs shown: the current results and runs kept for comparison. With several runs and several items the colour shows the item and the line style the run."),
    ("Geometry", "How the series is drawn: line, points, line and points, steps (the value holds from the previous x, like the records of the time-variable boundary data), area down to zero, or bars."),
    ("Colour", "Palette: the next colours of the figure palette. Off: one fixed colour for all series of the layer."),
    ("Line width (pt)", "Width of the lines in points."),
    ("Line style", "Solid, dashed, dotted or dash-dot line."),
    ("Marker", "Shape and size (points) of the markers of points and line-and-points layers."),
    ("Fill opacity", "Opacity of areas and bars (0 transparent, 1 opaque); the outline keeps the full colour."),
    ("Legend label", "Text of the legend entry; empty: automatic (item, run or variable). {item}, {run} and {y} are replaced by the print time / node / material, the run and the variable."),
    ("In the legend", "Lists the series of the layer in the legend."),
    ("Right y axis", "Plots the layer against a second y axis on the right side (e.g. precipitation next to fluxes)."),
    ("Axis title", "auto: the variable of the layers with its unit. Otherwise your own text with markup, e.g. θ (cm^{3} cm^{−3}) or K (cm d^{−1})."),
    ("Range", "auto: from the data, extended to the next ticks. Otherwise the range from the first to the second value (to let the values increase downward or to the left, use Reversed)."),
    ("Tick step", "Distance of the labelled ticks; 0: automatic. Logarithmic axes: the step in decades."),
    ("Logarithmic", "Logarithmic scale (values ≤ 0 are left out), e.g. for pressure heads or conductivities."),
    ("Reversed", "The values increase downward (depth) or to the left."),
    ("Grid lines", "Lines at the ticks across the plot area."),
    ("Minor ticks", "Small ticks between the labelled ones (and grid lines when the grid is on); logarithmic axes: at 2 … 9 times the powers of ten."),
    ("Add annotation", "Adds a text, a horizontal or vertical line (e.g. field capacity, a layer boundary, the start of irrigation) or an arrow to the selected panel."),
    ("Kind", "Text, horizontal line at y, vertical line at x, or arrow from (x, y) to the arrow end."),
    ("Text", "Text of the annotation (markup: x_{s}, x^{2}, *italic*, **bold**); lines and arrows show it next to them."),
    ("Position x, y", "Position of the annotation: data values of the axes, or 0–1 across the plot area when \"Relative to the plot area\" is on (0, 0 bottom left)."),
    ("Arrow end x, y", "Where the arrow points to (the head of the arrow)."),
    ("Font size (pt)", "Font size of the annotation in points; 0: the size of the tick labels."),
    ("Annotation colour", "Colour of the annotation; black follows the text colour of the theme (white in the dark theme)."),
    ("Text alignment", "The text starts at (left), is centred on or ends at (right) the position."),
    ("Line", "Line style and width of lines and arrows."),
    ("Relative to the plot area (0–1)", "On: the position is a fraction of the plot area and stays in place when the data change. Off: the position is in the units of the axes."),
    ("Annotation on the right y axis", "The y values of the annotation refer to the right y axis."),
    ("Place with the mouse", "On: a click in the selected panel of the preview moves the selected annotation there; a right click sets the end of an arrow."),
    ("Import table", "Reads a text or CSV file with columns (separated by tabs, semicolons, commas or blanks; a header line gives the names, units in brackets) as a data source of the figure, e.g. measurements or the results of another model. The table is stored in the project."),
    ("Run name", "Name of the kept run in the layer settings and the legend, e.g. \"Ks × 0.5\" or \"no roots\"."),
    ("Keep current results", "Keeps a copy of the current results and the project under the name, for a comparison with later runs. Kept runs stay until the program closes; save them as files to keep them longer."),
    ("Load run", "Reads a run saved with \"Save…\" or the results written by the command line program (--json)."),
    // ------------------------------------------------------------------ Rosetta inputs
    ("Sand", "Sand fraction [%] (0.05–2 mm, USDA)."),
    ("Silt", "Silt fraction [%] (0.002–0.05 mm)."),
    ("Clay", "Clay fraction [%] (< 0.002 mm). Sand + silt + clay = 100 %."),
    ("Bulk density", "Dry bulk density [g/cm³] (models with BD)."),
    ("θ at 33 kPa (field capacity)", "Measured water content [cm³/cm³] at −33 kPa (≈ −330 cm, field capacity)."),
    ("θ at 1500 kPa (wilting point)", "Measured water content [cm³/cm³] at −1500 kPa (≈ −15000 cm, wilting point)."),
    ("Textural class (class averages)", "Model H1: average parameters of the USDA textural class."),
    ("Sand, silt and clay (SSC)", "Model H2: prediction from the sand, silt and clay fractions."),
    ("SSC + bulk density (BD)", "Model H3: texture and bulk density."),
    ("SSC + BD + θ at 33 kPa", "Model H4: texture, bulk density and the water content at 33 kPa."),
    ("SSC + BD + θ at 33 and 1500 kPa", "Model H5: texture, bulk density and the water contents at 33 and 1500 kPa (the most informed prediction)."),
    ("geometric means of α, n and Ks", "Off: arithmetic means of the neural-network ensemble (as rosetta-soil and the Rosetta web site). On: 10 to the mean of the logarithms."),
];

fn map() -> &'static HashMap<&'static str, &'static str> {
    static M: OnceLock<HashMap<&'static str, &'static str>> = OnceLock::new();
    M.get_or_init(|| TIPS.iter().copied().collect())
}

/// Description of the input labelled `label`: exact match, else the label without trailing units in brackets
/// ("Dispersivity Dl (cm)", "cRoot [mmol/cm³]") and without a trailing number ("cTop2", "CncBalR 1 (%)").
pub fn tip(label: &str) -> Option<&'static str> {
    let m = map();
    let mut l = label.trim();
    loop {
        if let Some(t) = m.get(l) {
            return Some(t);
        }
        let shorter = if l.ends_with(')') && l.contains(" (") {
            &l[..l.rfind(" (").unwrap()]
        } else if l.ends_with(']') && l.contains(" [") {
            &l[..l.rfind(" [").unwrap()]
        } else {
            l.trim_end_matches(|c: char| c.is_ascii_digit())
        };
        let shorter = shorter.trim_end();
        if shorter.len() == l.len() || shorter.is_empty() {
            return None;
        }
        l = shorter;
    }
}

fn show(ui: &mut Ui, text: &str) {
    ui.set_max_width(440.0);
    ui.label(text);
    ui.add_space(2.0);
    ui.label(RichText::new("F1: help on this page").weak().small());
}

/// Tooltips on responses.
pub trait TipExt {
    /// Adds the description of `key` (see [`tip`]) as a tooltip, also when the widget is disabled.
    fn tip(self, key: &str) -> Self;
    /// Adds `text` as a tooltip (formatted like the other descriptions).
    fn tip_text(self, text: &str) -> Self;
}

impl TipExt for Response {
    fn tip(self, key: &str) -> Self {
        match tip(key) {
            Some(t) => self.tip_text(t),
            None => self,
        }
    }
    fn tip_text(self, text: &str) -> Self {
        self.on_hover_ui(|ui| show(ui, text)).on_disabled_hover_ui(|ui| show(ui, text))
    }
}

/// A label with its description.
pub fn lbl(ui: &mut Ui, text: &str) -> Response {
    ui.label(text).tip(text)
}

/// A column heading (strong) with its description.
pub fn hdr(ui: &mut Ui, text: &str) -> Response {
    ui.strong(text).tip(text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys_are_unique() {
        let mut k: Vec<&str> = TIPS.iter().map(|t| t.0).collect();
        k.sort_unstable();
        for w in k.windows(2) {
            assert_ne!(w[0], w[1], "duplicate tooltip key");
        }
    }

    #[test]
    fn lookup_variants() {
        assert!(tip("Dispersivity Dl (cm)").is_some());
        assert!(tip("cTop2").is_some());
        assert!(tip("CncBalR 1 (%)").is_some());
        assert!(tip("cRoot [mmol/cm³]").is_some());
        assert!(tip("no such label").is_none());
    }

    /// Every literal label of the input helpers and every `.tip("…")`, `lbl`, `hdr` key used in the sources has a
    /// description.
    #[test]
    fn every_used_key_has_a_description() {
        let sources = [
            ("pre.rs", include_str!("pre.rs")),
            ("profile.rs", include_str!("profile.rs")),
            ("post.rs", include_str!("post.rs")),
            ("inverse_ui.rs", include_str!("inverse_ui.rs")),
            ("rosetta_ui.rs", include_str!("rosetta_ui.rs")),
            ("main.rs", include_str!("main.rs")),
            ("figure/ui.rs", include_str!("figure/ui.rs")),
        ];
        let patterns = ["field(ui, ch, \"", "field_u(ui, ch, \"", "check(ui, ch, &mut ", ".tip(\"", "lbl(ui, \"", "hdr(ui, \"", "row(ui, \""];
        let mut missing = vec![];
        for (file, src) in sources {
            for pat in patterns {
                for part in src.split(pat).skip(1) {
                    let part = if pat.starts_with("check") {
                        // check(ui, ch, &mut <expr>, "label")
                        match part.find(", \"") {
                            Some(p) if !part[..p].contains('\n') => &part[p + 3..],
                            _ => continue,
                        }
                    } else {
                        part
                    };
                    let Some(end) = part.find('"') else { continue };
                    let key = &part[..end];
                    if key.is_empty() || key.contains('{') {
                        continue;
                    }
                    if tip(key).is_none() {
                        missing.push(format!("{file}: {key}"));
                    }
                }
            }
        }
        assert!(missing.is_empty(), "labels without a tooltip:\n{}", missing.join("\n"));
    }
}
