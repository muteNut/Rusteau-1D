//! Result containers produced by a simulation.

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct SoluteTLevel {
    pub cv_top: f64,
    pub cv_bot: f64,
    pub cum_top: f64,
    pub cum_bot: f64,
    pub cum_ch0: f64,
    pub cum_ch1: f64,
    pub c_top: f64,
    pub c_root: f64,
    pub c_bot: f64,
    pub cv_root: f64,
    pub cum_root: f64,
    pub cum_neq: f64,
    /// Average concentration of the groundwater layer (saturated elements above the bottom), cGWL
    #[serde(default)]
    pub c_gwl: f64,
    /// Solute flux with the surface runoff, cRunOff = vRunOff * cTop, and its cumulative value CumCh(10)
    #[serde(default)]
    pub c_run_off: f64,
    #[serde(default)]
    pub cum_run_off: f64,
}

/// One row of T_LEVEL.OUT (water flow + boundary information)
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct TLevel {
    pub tlevel: usize,
    pub t: f64,
    pub dt: f64,
    pub iter_w: usize,
    pub iter_c: usize,
    pub it_cum: usize,
    pub kod_top: i32,
    pub kod_bot: i32,
    pub converged: bool,
    pub r_top: f64,
    pub r_root: f64,
    pub v_top: f64,
    pub v_root: f64,
    pub v_bot: f64,
    pub cum_r_top: f64,
    pub cum_r_root: f64,
    pub cum_v_top: f64,
    pub cum_v_root: f64,
    pub cum_v_bot: f64,
    pub h_top: f64,
    pub h_root: f64,
    pub h_bot: f64,
    pub run_off: f64,
    pub cum_run_off: f64,
    pub volume: f64,
    pub cum_infil: f64,
    pub cum_evap: f64,
    pub precip: f64,
    pub peclet: f64,
    pub courant: f64,
    pub temp_top: f64,
    pub temp_bot: f64,
    /// Cum(WTrans): cumulative water transfer between mobile and immobile domains (CumQ(11))
    #[serde(default)]
    pub cum_w_transf: f64,
    /// Water equivalent of the snow cover (SnowLayer)
    #[serde(default)]
    pub snow_layer: f64,
    pub solutes: Vec<SoluteTLevel>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct NodeOut {
    pub node: usize,
    /// depth below the surface (>= 0)
    pub depth: f64,
    pub h: f64,
    pub theta: f64,
    pub k: f64,
    pub c: f64,
    /// Total water flux: liquid + vapor (vi + vVi in NodOut)
    pub flux: f64,
    /// Vapor part of `flux` (vVi)
    #[serde(default)]
    pub flux_vapor: f64,
    pub sink: f64,
    /// Dual-porosity: water transfer to the immobile domain (SinkIm), immobile water
    /// content (ThNewIm) and solute transfer (STrans)
    #[serde(default)]
    pub sink_im: f64,
    #[serde(default)]
    pub th_im: f64,
    #[serde(default)]
    pub s_trans: f64,
    /// Vapor-flow output (Nod_Inf variant with ConLT, ConVh, ConVT and the vapor flux components)
    #[serde(default)]
    pub con_lt: f64,
    #[serde(default)]
    pub con_vh: f64,
    #[serde(default)]
    pub con_vt: f64,
    #[serde(default)]
    pub v_vap_iso: f64,
    #[serde(default)]
    pub v_vap_term: f64,
    pub kappa: i32,
    pub v_over_ks: f64,
    pub temp: f64,
    pub conc: Vec<f64>,
    pub sorb: Vec<f64>,
    pub sorb2: Vec<f64>,
    pub h_matrix: Option<f64>,
    pub th_matrix: Option<f64>,
    pub conc_matrix: Option<Vec<f64>>,
    pub sorb_matrix: Option<Vec<f64>>,
    pub flux_conc: Option<Vec<f64>>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ProfileOut {
    pub t: f64,
    pub nodes: Vec<NodeOut>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ObsPoint {
    pub node: usize,
    pub h: f64,
    pub theta: f64,
    pub temp: f64,
    pub flux: f64,
    pub conc: Vec<f64>,
    pub flux_conc: Option<Vec<f64>>,
    pub h_matrix: Option<f64>,
    pub th_matrix: Option<f64>,
    pub conc_matrix: Option<Vec<f64>>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ObsOut {
    pub t: f64,
    pub points: Vec<ObsPoint>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ALevel {
    pub t: f64,
    pub cum: [f64; 5],
    pub h_top: f64,
    pub h_root: f64,
    pub h_bot: f64,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct SubRegion {
    pub area: f64,
    /// Water volume (mobile + immobile domain, plus vapor when simulated)
    pub volume: f64,
    /// Immobile-domain part of `volume` (W-volumeI; only set for the total, dual-porosity)
    #[serde(default)]
    pub volume_im: f64,
    pub change: f64,
    pub h_mean: f64,
    /// Heat content (HeatVol) and mean temperature in K (tMean); only with heat transport
    #[serde(default)]
    pub heat_vol: f64,
    #[serde(default)]
    pub t_mean: f64,
    /// Solute mass in all domains and phases (ConVol + ConVolIm + ConVolIm2)
    pub c_vol: Vec<f64>,
    pub c_mean: Vec<f64>,
    /// Part of `c_vol` in the immobile domain / non-equilibrium sites (ConVolIm + ConVolIm2)
    #[serde(default)]
    pub c_vol_im: Vec<f64>,
    /// Mean concentration of the immobile phase (cMeanIm, or sorbed fraction sMeanIm)
    #[serde(default)]
    pub c_mean_im: Vec<f64>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct BalanceOut {
    pub t: f64,
    pub total: SubRegion,
    pub sub: Vec<SubRegion>,
    pub top_flux: f64,
    pub bot_flux: f64,
    pub wat_bal_t: f64,
    pub wat_bal_r: f64,
    pub sol_bal_t: Vec<f64>,
    pub sol_bal_r: Vec<f64>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ParameterStat {
    pub name: String,
    pub value: f64,
    pub se: f64,
    pub lower_95: f64,
    pub upper_95: f64,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct InverseOutputData {
    pub title: String,
    pub final_ssq: f64,
    pub iterations_run: usize,
    pub observations: Vec<crate::inverse::ObservationPoint>,
    pub simulated_values: Vec<f64>,
    pub r_squared: f64,
    pub parameters: Vec<crate::inverse::ParameterStat>,
    pub correlation_matrix: Vec<Vec<f64>>,
}

/// One row of Meteo.out. Which columns are meaningful depends on `Results::meteo_mode`:
/// 0 = Penman-Monteith with meteo records (ET .. ExInterc), 1 = Penman-Monteith with daily-generated data
/// (+ AirTemp, AirRH, SolarRad), 2 = surface energy balance (Rns, Rnl, Rn, Sensible, Latent, HeatFlux, Balance),
/// 3 = energy balance with daily-generated data (+ AirTemp, AirRH, SolarRad). Times in days.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct MeteoRow {
    pub t: f64,
    pub et: f64,
    pub evap_p: f64,
    pub transp_p: f64,
    pub rns: f64,
    pub rnl: f64,
    pub rad_term: f64,
    pub aero_term: f64,
    pub prec: f64,
    pub interc: f64,
    pub ex_interc: f64,
    pub rn: f64,
    pub sensible: f64,
    pub latent: f64,
    pub heat_flux: f64,
    pub balance: f64,
    pub air_temp: f64,
    pub air_rh: f64,
    pub solar_rad: f64,
}

/// One row of T_Level1.OUT (only written when lFluxOut is on): potential and actual fluxes
/// together with their cumulative values.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct TLevel1 {
    pub t: f64,
    /// PrecipP, EvaporP, FluxTopP, FluxTopA, InfiltrA, EvaporA, TranspP, TranspA, FluxBot, RunOff
    pub prec: f64,
    pub r_soil: f64,
    pub r_top: f64,
    pub v_top: f64,
    pub r_infil: f64,
    pub r_evap: f64,
    pub r_root: f64,
    pub v_root: f64,
    pub v_bot: f64,
    pub run_off: f64,
    /// Sum(PrecP) = CumQ(9), Sum(EvapP) = CumQ(10), then the cumulative fluxes CumQ(1,3,7,8,2,4,5,6)
    pub cum_prec: f64,
    pub cum_evap_p: f64,
    pub cum_r_top: f64,
    pub cum_v_top: f64,
    pub cum_infil: f64,
    pub cum_evap: f64,
    pub cum_r_root: f64,
    pub cum_v_root: f64,
    pub cum_v_bot: f64,
    pub cum_run_off: f64,
    pub volume: f64,
    pub snow_layer: f64,
    pub v_top_w: f64,
    pub v_top_v: f64,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Results {
    pub tlevel: Vec<TLevel>,
    #[serde(default)]
    pub tlevel1: Vec<TLevel1>,
    #[serde(default)]
    pub meteo: Vec<MeteoRow>,
    #[serde(default)]
    pub meteo_mode: u8,
    pub profiles: Vec<ProfileOut>,
    pub obs: Vec<ObsOut>,
    pub alevel: Vec<ALevel>,
    pub balance: Vec<BalanceOut>,
    pub messages: Vec<String>,
    pub n_solutes: usize,
    pub obs_nodes: Vec<usize>,
    pub finished: bool,
    pub failed: bool,
    pub inverse_results: Option<InverseOutputData>,
}
