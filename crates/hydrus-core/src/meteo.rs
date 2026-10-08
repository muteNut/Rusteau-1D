//! Meteorological ET (port of the non-daily, non-energy-balance parts of
//! TIME.FOR: SetMeteo, RadGlobal, Cloudiness, CropRes, RadLongNet, Aero).

use crate::model::{MeteoRecord, MeteoSettings};
use crate::vapor::latent_heat_volumetric;

fn ea(t: f64) -> f64 {
    0.6108 * ((17.27 * t) / (t + 237.3)).exp()
}

/// RadGlobal: potential (extraterrestrial) radiation.
fn rad_global(lat: f64, day_no: f64) -> (f64, f64, f64, f64) {
    let pi = std::f64::consts::PI;
    let sc = 118.08;
    let lat1 = (lat - lat.trunc()) * (5.0 / 3.0) + lat.trunc();
    let lat2 = lat1 * pi / 180.0;
    let sol_declin = (2.0 * pi / 365.0 * day_no - 1.39).sin() * 0.4093;
    let omega = (-sol_declin.tan() * lat2.tan()).clamp(-1.0, 1.0).acos();
    let xx = sol_declin.sin() * lat2.sin();
    let yy = sol_declin.cos() * lat2.cos();
    let dr = 1.0 + 0.033 * ((2.0 * pi / 365.0) * day_no).cos();
    let ra = sc / pi * dr * (omega * xx + omega.sin() * yy);
    (ra, omega, xx, yy)
}

/// Cloudiness: cloudiness factor CloudF and relative sunshine duration n/N.
/// Returns `(cloud_f, n_n)`.
fn cloudiness(
    i_sun_sh: i32,
    sun_hours: f64,
    omega: f64,
    long_wave_b: f64,
    long_wave_a: f64,
    rad: f64,
    rad_cs: f64, // clear-sky radiation: Ra*(as + bs)
    ac: f64,
    bc: f64,
) -> (f64, f64) {
    let pi = std::f64::consts::PI;
    match i_sun_sh {
        0 => {
            let nn = 24.0 / pi * omega;
            let n_n = if nn > 0.0 { (sun_hours / nn).min(1.0) } else { 0.0 };
            (long_wave_b + long_wave_a * n_n, n_n)
        }
        1 => {
            // CloudF = SunHours; n_N = (CloudF - b)/a (not clamped, as in Cloudiness)
            (sun_hours, (sun_hours - long_wave_b) / long_wave_a.max(1e-12))
        }
        2 => {
            let cover = (2.330 - 3.330 * sun_hours).clamp(0.0001, 1.0);
            (long_wave_b + long_wave_a * (1.0 - cover), 1.0 - cover)
        }
        _ => {
            // iSunSh = 3: cloudiness from measured solar radiation (Eq. 57), no clamp;
            // n_N = 1 - Cover with Cover from Tt = Rad/Ra
            let cf = if rad_cs > 0.0 { ac * rad / rad_cs + bc } else { long_wave_b };
            (cf, 0.0)
        }
    }
}

/// CropRes: crop canopy resistance, soil-cover fraction and the LAI actually used.
/// Returns `(rc, scf, lai)`.
fn crop_res(i_lai: i32, lai_in: f64, r_extinct: f64, i_crop: i32, crop_height: f64) -> (f64, f64, f64) {
    let l_crop = i_crop != 0 && crop_height > 0.0;
    if !l_crop {
        return (0.0, 0.0, lai_in);
    }
    let lai = match i_lai {
        1 => 0.24 * crop_height,                // clipped grass
        2 => 1.5 * crop_height.ln() + 5.5,      // alfalfa
        3 => {
            if lai_in < 1.0 {
                -((1.0 - lai_in).ln()) / r_extinct.max(0.1)
            } else {
                10.0
            }
        }
        _ => lai_in,
    };
    let mut rc = 0.0;
    let mut scf = 0.0;
    if lai > 0.0 {
        rc = 200.0 / lai;
        scf = (1.0 - (-r_extinct.max(0.1) * lai).exp()).max(0.0);
    }
    (rc, scf, lai)
}

/// RadLongNet: net longwave radiation.
fn rad_long_net(t_max: f64, t_min: f64, a1: f64, b1: f64, ea_dew: f64, cloud_f: f64) -> f64 {
    let sigma = 0.00000000245 * ((t_max + 273.16).powi(4) + (t_min + 273.16).powi(4));
    let emissivity = a1 + b1 * ea_dew.max(0.0).sqrt();
    sigma * cloud_f * emissivity
}

/// Aero: aerodynamic and radiation terms of the Penman-Monteith equation.
#[allow(clippy::too_many_arguments)]
fn aero(
    rn: f64,
    rc: f64,
    crop_height: f64,
    wind_height: f64,
    temp_height: f64,
    wind_ms: f64,
    altitude: f64,
    t_max: f64,
    t_min: f64,
    ea_max: f64,
    ea_min: f64,
    ea_dew: f64,
) -> Option<(f64, f64)> {
    let ch = crop_height;
    let dl0 = 0.667 * ch;
    if dl0 >= wind_height || dl0 >= temp_height {
        return None;
    }
    let aer_dyn_res = ((wind_height - dl0) / (0.123 * ch)).ln() * ((temp_height - dl0) / (0.0123 * ch)).ln() / 0.41f64.powi(2);
    let raa = if wind_ms > 0.0 { aer_dyn_res / wind_ms } else { 0.0 };
    let aero_t_cff = 0.622 * 3.486 * 86400.0 / aer_dyn_res / 1.01;
    let p_atm = 101.3 * ((293.0 - 0.0065 * altitude) / 293.0).powf(5.253);
    let t_aver = (t_max + t_min) / 2.0;
    let lambda = 2.501 - 0.002361 * t_aver;
    let gamma = 0.0016286 * p_atm / lambda;
    let gamma1 = if raa > 0.0 { gamma * (1.0 + rc / raa) } else { gamma };
    let dlt = 2049.0 * ea_max / (t_max + 237.3).powi(2) + 2049.0 * ea_min / (t_min + 237.3).powi(2);
    let dl_dl = dlt / (dlt + gamma1);
    let gm_dl = gamma / (dlt + gamma1);
    let ea_mean = (ea_max + ea_min) / 2.0;
    let aero_term = gm_dl * aero_t_cff / (t_aver + 273.0) * wind_ms * (ea_mean - ea_dew);
    let rad_term = if lambda > 0.0 { dl_dl * rn / lambda } else { 0.0 };
    Some((aero_term, rad_term))
}

/// Table: linear interpolation of the crop growth table (columns 2..5 of
/// `[time, height, albedo, LAI, root depth]`), constant outside the table.
pub fn growth_table(tab: &[[f64; 5]], t: f64) -> (f64, f64, f64, f64) {
    let pick = |r: &[f64; 5]| (r[1], r[2], r[3], r[4]);
    if tab.is_empty() {
        return (0.0, 0.0, 0.0, 0.0);
    }
    if t <= tab[0][0] {
        return pick(&tab[0]);
    }
    if t >= tab[tab.len() - 1][0] {
        return pick(&tab[tab.len() - 1]);
    }
    let mut out = pick(&tab[0]);
    for w in tab.windows(2) {
        if t > w[0][0] && t <= w[1][0] {
            let f = (t - w[0][0]) / (w[1][0] - w[0][0]);
            out = (
                w[0][1] + (w[1][1] - w[0][1]) * f,
                w[0][2] + (w[1][2] - w[0][2]) * f,
                w[0][3] + (w[1][3] - w[0][3]) * f,
                w[0][4] + (w[1][4] - w[0][4]) * f,
            );
        }
    }
    out
}

/// Crop properties valid at time `t`: iCrop = 3 record values, iCrop = 2 growth table,
/// iCrop = 1 constants of Meteo.in. Returns `(crop_height [cm], albedo, LAI, root depth)`.
fn crop_state(
    mp: &MeteoSettings,
    t: f64,
    h: Option<f64>,
    a: Option<f64>,
    l: Option<f64>,
    xr: Option<f64>,
) -> (f64, f64, f64, Option<f64>) {
    match mp.i_crop {
        3 => (h.unwrap_or(mp.crop_height), a.unwrap_or(mp.albedo), l.unwrap_or(mp.lai), xr),
        2 if !mp.growth.is_empty() => {
            let (ch, al, lai, x) = growth_table(&mp.growth, t);
            (ch, al, lai, Some(x))
        }
        1 => (mp.crop_height, mp.albedo, mp.lai, if mp.x_root > 0.0 { Some(mp.x_root) } else { None }),
        _ => (mp.crop_height, mp.albedo, mp.lai, None),
    }
}

/// Result of SetMeteo for one record.
#[derive(Clone, Copy, Debug, Default)]
pub struct PotEt {
    /// Potential evaporation [mm/d]
    pub evap_p: f64,
    /// Potential transpiration [mm/d]
    pub trans_p: f64,
    /// LAI after CropRes (used by the interception model)
    pub lai: f64,
    /// Soil cover fraction
    pub scf: f64,
    /// Root depth supplied with the meteo data, if any
    pub x_root: Option<f64>,
    /// Combined (reference/crop) evapotranspiration ETcomb [mm/d] and its terms (Meteo.out)
    pub et_comb: f64,
    pub rns: f64,
    pub rnl: f64,
    pub rad_term: f64,
    pub aero_term: f64,
}

/// SetMeteo: potential evaporation and transpiration [mm/d] for one meteorological record.
/// `is_hourly` is iMetHour (first record closer than one day to the initial time).
/// Returns `None` when `Aero` fails (crop height above the measurement heights, ierr = 3).
pub fn potential_et_ex(mp: &MeteoSettings, rec: &MeteoRecord, t_conv: f64, is_hourly: bool) -> Option<PotEt> {
    let tt_conv = 24.0 * 3600.0 * t_conv;
    let day_no = (rec.t / tt_conv).rem_euclid(365.0);
    let t_aver = (rec.t_max + rec.t_min) / 2.0;

    let (crop_height, albedo, lai_in, x_root) = crop_state(mp, rec.t, rec.crop_height, rec.albedo, rec.lai, rec.x_root);
    let (rc, scf, lai) = crop_res(mp.i_lai, lai_in, mp.r_extinct, mp.i_crop, crop_height);
    // CropRes sets the crop height to 0.1 cm when there is no crop
    let ch_eff = if mp.i_crop == 0 || crop_height <= 0.0 { 0.1 } else { crop_height };

    if mp.hargreaves {
        let (ra, _, _, _) = rad_global(mp.latitude, day_no);
        let etcomb = 0.0023 * 0.408 * ra * (t_aver + 17.8) * (rec.t_max - rec.t_min).max(0.0).sqrt();
        return Some(PotEt { evap_p: etcomb * (1.0 - scf), trans_p: etcomb * scf, lai, scf, x_root, et_comb: etcomb, ..Default::default() });
    }

    let ea_max = ea(rec.t_max);
    let ea_min = ea(rec.t_min);
    let ea_dew = if mp.i_rel_hum == 0 {
        rec.rh_mean / (50.0 / ea_min + 50.0 / ea_max)
    } else {
        rec.rh_mean
    };

    let (rn, rns_o, rnl_o) = if mp.i_radiation != 2 {
        let (ra, omega, xx, yy) = rad_global(mp.latitude, day_no);
        let rad_cs = ra * (mp.short_wave_a + mp.short_wave_b * 1.0);

        let (mut cloud_f, mut n_n) = cloudiness(
            mp.i_sun_sh,
            rec.sun_hours,
            omega,
            mp.long_wave_b,
            mp.long_wave_a,
            rec.rad,
            rad_cs,
            mp.cloud_fact_ac,
            mp.cloud_fact_bc,
        );

        let mut rad = rec.rad;
        if mp.i_radiation == 0 {
            rad = ra * (mp.short_wave_a + mp.short_wave_b * n_n); // Equation 52
        }
        if mp.i_sun_sh == 3 {
            let tt = rad / ra;
            n_n = 1.0 - (2.330 - 3.330 * tt).clamp(0.1, 1.0);
        }
        let _ = n_n;

        if mp.i_radiation == 1 && mp.i_sun_sh == 3 && is_hourly {
            // Sub-daily clear-sky radiation diurnal variation
            let pi = std::f64::consts::PI;
            let mut sum_sine = 0.0;
            for k in 1..=24 {
                let sine1 = xx + yy * (2.0 * pi / 24.0 * (k as f64 - 12.0)).cos();
                sum_sine += sine1.max(0.0) / 24.0;
            }
            let hour_no = 24.0 * day_no.fract();
            let sine = xx + yy * (2.0 * pi / 24.0 * (hour_no - 12.0)).cos();
            let rad_csh = if sum_sine > 0.0 { (sine * rad_cs / sum_sine).max(0.0) } else { 0.0 };
            cloud_f = if rad_csh <= 0.0001 {
                mp.cloud_fact_ac * 0.6 + mp.cloud_fact_bc // night time
            } else if rec.rad >= rad_csh {
                mp.cloud_fact_ac * 1.0 + mp.cloud_fact_bc
            } else {
                (mp.cloud_fact_ac * rec.rad / rad_csh + mp.cloud_fact_bc).max(0.01)
            };
        }

        let rns = (1.0 - albedo) * rad;
        let rnl = rad_long_net(rec.t_max, rec.t_min, mp.long_wave_a1, mp.long_wave_b1, ea_dew, cloud_f);
        (rns - rnl, rns, rnl)
    } else {
        (rec.rad, 0.0, 0.0)
    };

    let wind_ms = rec.wind_kmd / 86.4;
    let (aero_term, rad_term) = aero(
        rn,
        rc,
        ch_eff,
        mp.wind_height,
        mp.temp_height,
        wind_ms,
        mp.altitude,
        rec.t_max,
        rec.t_min,
        ea_max,
        ea_min,
        ea_dew,
    )?;

    let mut etcomb = (rad_term + aero_term).max(0.0);
    let row = (1.0 - 7.37e-6 * (t_aver - 4.0).powi(2) + 3.79e-8 * (t_aver - 4.0).powi(3)) * 1000.0;
    etcomb = etcomb / row * 1000.0;
    Some(PotEt { evap_p: etcomb * (1.0 - scf), trans_p: etcomb * scf, lai, scf, x_root, et_comb: etcomb, rns: rns_o, rnl: rnl_o, rad_term, aero_term })
}

/// SetMeteo: potential evaporation and transpiration [mm/d] for one meteorological record
/// (iMetHour is guessed from the spacing of the first two records).
pub fn potential_et(mp: &MeteoSettings, rec: &MeteoRecord, t_conv: f64) -> (f64, f64) {
    let tt_conv = 24.0 * 3600.0 * t_conv;
    let is_hourly = mp.records.len() > 1 && (mp.records[1].t - mp.records[0].t) <= (0.9999 * tt_conv);
    potential_et_ex(mp, rec, t_conv, is_hourly)
        .map(|p| (p.evap_p, p.trans_p))
        .unwrap_or((0.0, 0.0))
}

/// Interception state container holding carry-over water between time steps.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct InterceptionState {
    pub excess: f64,
}

/// Canopy interception model (port of Intercept and IntercepS from TIME.FOR).
///
/// * `prec_in`: Precipitation intensity [L/T]
/// * `trans_p`: Potential transpiration [L/T]
/// * `lai`: Leaf area index [-]
/// * `a_interc`: Interception constant [L] or [L/T] (default ~0.25 mm)
/// * `scf`: Soil cover fraction [-]
/// * `state`: Carry-over intercepted water
///
/// Returns: `(net_prec, net_trans_p, actual_interc)` in [L/T].
pub fn calculate_interception(
    prec_in: f64,
    trans_p: f64,
    lai: f64,
    a_interc: f64,
    scf: f64,
    state: &mut InterceptionState,
) -> (f64, f64, f64) {
    // Intercept: new interception only with LAI > 0 (and a meaningful capacity)
    let mut new_interc = 0.0;
    if lai > 0.0 && a_interc > 0.0 {
        if prec_in > 0.0 {
            let denom = 1.0 + (scf * prec_in) / (a_interc * lai);
            new_interc = (a_interc * lai * (1.0 - 1.0 / denom)).min(prec_in);
        }
        // Capacity limit
        let max_interc = a_interc * lai;
        if new_interc + state.excess > max_interc {
            new_interc = (max_interc - state.excess).max(0.0);
        }
    }

    let net_prec = (prec_in - new_interc).max(0.0);
    let total_interc = state.excess + new_interc; // old + new interception
    state.excess = 0.0;

    // Transpiration consumes intercepted water first
    if trans_p < total_interc {
        state.excess = total_interc - trans_p;
    }
    let net_trans = (trans_p - total_interc).max(0.0);

    (net_prec, net_trans, total_interc)
}

/// Snow accumulation and melt tracking (port of subroutine Snow from TIME.FOR).
///
/// * `prec`: Incoming precipitation [L/T]
/// * `temp`: Air temperature [°C]
/// * `dt`: Time step length [T]
/// * `snow_mf`: Snowmelt degree-day factor [L/(T·°C)]
/// * `snow_layer`: Current snowpack equivalent depth [L]
/// * `r_evap`: Potential soil evaporation [L/T]
/// * `x_conv`: Length conversion to meters
///
/// Returns: `(net_prec, updated_snow_layer, net_evap, min_step_needed)`
pub fn calculate_snow(
    mut prec: f64,
    temp: f64,
    dt: f64,
    snow_mf: f64,
    mut snow_layer: f64,
    mut r_evap: f64,
    x_conv: f64,
    c_top: &mut [f64],
    c_t: &[f64],
) -> (f64, f64, f64, bool) {
    let prec_old = prec;
    let r_evap_old = r_evap;
    let safe_dt = dt.max(1e-8);

    // Rain vs snow fraction Q
    let q = if snow_layer < 0.001 * x_conv {
        if temp < -2.0 {
            1.0
        } else if temp < 2.0 {
            1.0 - ((temp + 2.0) / 4.0)
        } else {
            0.0
        }
    } else {
        1.0
    };

    let r_top = prec * (1.0 - q);
    let snow_f = prec * q;

    let mut snow_melt = if temp > 0.0 && snow_layer > 0.0 {
        temp * snow_mf * dt
    } else {
        0.0
    };

    snow_layer = snow_layer + snow_f * dt - snow_melt;

    if snow_layer < 0.0 {
        snow_melt += snow_layer;
        snow_layer = 0.0;
    } else if snow_layer > 0.0 && r_evap > 0.0 {
        if r_evap * dt < snow_layer {
            snow_layer -= r_evap * dt;
            r_evap = 0.0;
        } else {
            r_evap = (r_evap * dt - snow_layer) / safe_dt;
            snow_layer = 0.0;
        }
    }

    prec = r_top + snow_melt / safe_dt;

    // Check if time step needs reduction due to abrupt rain/melt flux changes
    let min_step = (prec_old - prec).abs() > prec.abs() * 0.2 && prec > 0.0;

    // Solute concentration mixing in snowpack
    if snow_layer > 0.001 * x_conv {
        let denom = snow_layer + dt * (prec_old - r_evap_old);
        if denom > 0.0 {
            for (ct, &cin) in c_top.iter_mut().zip(c_t.iter()) {
                *ct = (snow_layer * (*ct) + dt * prec_old * cin) / denom;
            }
        }
    }

    (prec, snow_layer, r_evap, min_step)
}

/// Atmospheric stability iteration for aerodynamic resistance to heat and vapor flow (AeroRes).
pub fn aero_res(
    temp_height: f64,
    wind_height: f64,
    wind_ms: f64,
    t_kelv_s: f64,
    t_kelv_a: f64,
) -> f64 {
    let r_k = 0.41;
    let pi = std::f64::consts::PI;
    let g = 9.81;
    let ca = 1200.0; // Volumetric heat capacity of air [J/m3/K]

    let zm = 0.001; // Roughness parameter for momentum [m]
    let zh = zm;
    let dl = 0.0;
    let t_height = temp_height / 100.0; // cm -> m
    let w_height = wind_height / 100.0;

    if wind_ms <= 0.0 {
        let diff0 = 2.12e-5;
        let diff_t = diff0 * (t_kelv_a / 273.15).powi(2);
        return t_height / diff_t;
    }

    if (t_kelv_s - t_kelv_a).abs() < 0.01 {
        // Neutral atmosphere
        return (1.0 / wind_ms / r_k / r_k)
            * ((t_height - dl) / zh).ln()
            * ((w_height - dl) / zm).ln();
    }

    let mut psim = 0.0;
    let mut psih = 0.0;
    let mut r_v = 0.0;
    let mut rv_min = 0.0;
    let mut rv_max = 0.0;

    for i in 1..=6 {
        let uu = wind_ms * r_k / (((w_height - dl + zm) / zm).ln() + psim);
        r_v = 1.0 / uu / r_k * (((t_height - dl + zh) / zh).ln() + psih);

        if i == 1 {
            rv_min = 0.1 * r_v;
            rv_max = 10.0 * r_v;
        } else if r_v > rv_max {
            return rv_max;
        } else if r_v < rv_min {
            return rv_min;
        }

        let denom = ca * (t_kelv_s - t_kelv_a) / r_v;
        let mo = if denom.abs() > 1e-12 {
            -ca * t_kelv_a * uu.powi(3) / r_k / g / denom
        } else {
            1e10
        };

        let zeta = (t_height - dl) / mo;
        if zeta < 0.0 {
            // Unstable
            let xx = (1.0 - 16.0 * zeta).max(0.0).powf(0.25);
            psih = -2.0 * ((1.0 + xx * xx) / 2.0).ln();
            psim = -2.0 * ((1.0 + xx) / 2.0).ln() - ((1.0 + xx * xx) / 2.0).ln() + 2.0 * xx.atan() - pi / 2.0;
        } else if zeta > 0.0 {
            // Stable
            if zeta < 1.0 {
                psih = 5.0 * zeta;
                psim = psih;
            } else {
                psih = 5.0;
                psim = psih;
            }
        }
    }
    r_v
}

/// Result of one call of `evapor` (port of subroutine Evapor in TIME.FOR).
#[derive(Clone, Copy, Debug, Default)]
pub struct EvaporOut {
    /// Evaporation flux [kg/m2/s]
    pub evap: f64,
    /// Heat flux into the soil, positive downward [W/m2]
    pub heat_flux: f64,
    /// Sensible heat flux [W/m2]
    pub sens_flux: f64,
    /// Latent heat of vaporization xLatent(TempA)/rho_w(TempS) [J/kg]
    pub lat: f64,
    /// Aerodynamic resistance to heat flow [s/m]
    pub r_h: f64,
    pub rn: f64,
    pub rns: f64,
    pub rnl: f64,
    pub temp_a: f64,
    /// Density of soil water at the surface temperature [kg/m3]
    pub row: f64,
}

/// Port of subroutine Evapor (non daily-generated meteo data, i.e. lMetDaily = false):
/// surface energy balance including the hourly variation of air temperature and of
/// shortwave radiation.
///
/// * `t_days`   - time in days (for DayNo / hour of day)
/// * `t_proj`   - time in project time units (for the air temperature sine)
/// * `period`   - period of the temperature sine (heat tPeriod, project time units)
/// * `is_hourly`- iMetHour = 1 (meteo data given at intervals of one day or less)
/// * `daily`    - lMetDaily: `Some((TempA, RH_A, Rst))` from `DailyMet`/`SetDayMet`
///   (air temperature, relative humidity and net shortwave input at the current time)
#[allow(clippy::too_many_arguments)]
pub fn evapor(
    mp: &MeteoSettings,
    t_days: f64,
    t_proj: f64,
    period: f64,
    temp_s: f64,
    t_max_a: f64,
    t_min_a: f64,
    rad_in: f64,
    sun_hours: f64,
    rh_mean: f64,
    wind_ms: f64,
    h_top: f64,
    theta_top: f64,
    is_hourly: bool,
    x_conv: f64,
    daily: Option<(f64, f64, f64)>,
) -> EvaporOut {
    let pi = std::f64::consts::PI;
    let g = 9.81;
    let x_mol = 0.018015;
    let r_gas = 8.314;
    let ca = 1200.0;

    // Hourly variations of atmospheric temperature
    let t_mean = (t_max_a + t_min_a) / 2.0;
    let mut temp_a = t_mean;
    if period > 0.0 {
        temp_a = t_mean + (t_max_a - t_min_a) / 2.0 * (2.0 * pi * t_proj / period - 7.0 * pi / 12.0).sin();
    }
    let mut rh_mean = rh_mean;
    if let Some((ta, rha, _)) = daily {
        temp_a = ta; // lMetDaily: RHMean = RH_A, TempA from DailyMet
        rh_mean = rha;
    }
    let t_kelv_s = temp_s + 273.15;
    let t_kelv_a = temp_a + 273.15;
    let rovs_a = 0.001 * (31.3716 - 6014.79 / t_kelv_a - 0.00792495 * t_kelv_a).exp() / t_kelv_a;
    let roa = rh_mean / 100.0 * rovs_a;

    let (rn, rns, rnl);
    if mp.i_radiation != 2 {
        let day_no = t_days.rem_euclid(365.0);
        let (ra, omega, xx, yy) = rad_global(mp.latitude, day_no);
        let sc = 118.08;
        let mut rad = rad_in;

        // Cloudiness
        let (mut n_n, mut cover, mut tt) = (0.0_f64, 0.0_f64, 0.0_f64);
        match mp.i_sun_sh {
            0 => {
                let nn = 24.0 / pi * omega;
                n_n = if nn > 0.0 { (sun_hours / nn).min(1.0) } else { 0.0 };
                cover = 1.0 - n_n;
            }
            1 => {
                n_n = (sun_hours - mp.long_wave_b) / mp.long_wave_a;
                cover = 1.0 - n_n;
            }
            2 => {
                tt = sun_hours;
                cover = (2.330 - 3.330 * tt).clamp(0.0001, 1.0);
                n_n = 1.0 - cover;
            }
            _ => {}
        }
        let _ = n_n;

        if mp.i_radiation == 0 || mp.i_sun_sh == 3 {
            if mp.i_radiation == 0 {
                rad = ra * (mp.short_wave_a + mp.short_wave_b * n_n); // Equation 52
            }
            if mp.i_sun_sh == 3 {
                if is_hourly {
                    let hour_no = 24.0 * day_no.fract();
                    let sine = xx + yy * (2.0 * pi / 24.0 * (hour_no - 12.0)).cos();
                    let ra_h = (sc * sine).max(0.0);
                    if ra_h <= 0.0001 {
                        cover = 0.6; // night time
                    } else {
                        tt = (rad / ra_h).min(1.0);
                        cover = (2.330 - 3.330 * tt).clamp(0.1, 1.0);
                    }
                } else {
                    tt = rad / ra;
                    cover = (2.330 - 3.330 * tt).clamp(0.1, 1.0);
                }
            }
        }
        let rad_h = if let Some((_, _, rst)) = daily {
            // lMetDaily: RadH = Rst (the albedo is still the soil-moisture dependent one)
            let _ = (tt, sc);
            rst
        } else if mp.i_radiation == 0 || !is_hourly {
            if mp.i_sun_sh != 2 {
                tt = rad / ra;
            }
            let hour_no = 24.0 * day_no.fract();
            let sine = xx + yy * (2.0 * pi / 24.0 * (hour_no - 12.0)).cos();
            (sc * tt * sine).max(0.0) // Equation 12.7
        } else {
            rad
        };

        // Surface albedo, van Bavel and Hillel (1976)
        let albedo = if theta_top > 0.25 {
            0.1
        } else if theta_top > 0.1 {
            0.35 - theta_top
        } else {
            0.25
        };
        rns = (1.0 - albedo) * rad_h;

        // RadLongNet1
        let sigma = 4.899e-9;
        let es = ea(temp_a);
        let ea_act = rh_mean / 100.0 * es;
        let epsi = 1.24 * (ea_act / t_kelv_a).powf(1.0 / 7.0); // Brutsaert (1975)
        let epsi_a = ((1.0 - 0.84 * cover) * epsi + 0.84 * cover).clamp(0.0, 1.0);
        let epsi_s = (0.9 + 0.18 * theta_top).min(1.0);
        let rlu = epsi_s * sigma * t_kelv_s.powi(4);
        let rld = epsi_a * sigma * t_kelv_a.powi(4);
        rnl = rld - rlu;
        rn = rns + rnl;
    } else {
        rn = rad_in;
        rns = 0.0;
        rnl = 0.0;
    }

    let r_v = aero_res(mp.temp_height, mp.wind_height, wind_ms, t_kelv_s, t_kelv_a);
    let r_s = if theta_top < 0.15 { 10.0 * (35.63 * (0.15 - theta_top)).exp() } else { 10.0 };
    let r_h = r_v;
    let sens_flux = ca * (temp_s - temp_a) / r_h;

    let h_m = h_top / x_conv;
    let hr = (h_m * x_mol * g / r_gas / t_kelv_s).exp();
    let rovs_s = 0.001 * (31.3716 - 6014.79 / t_kelv_s - 0.00792495 * t_kelv_s).exp() / t_kelv_s;
    let rov = rovs_s * hr;
    let evap = ((rov - roa) / (r_v + r_s)).max(0.0);
    let row = (1.0 - 7.37e-6 * (temp_s - 4.0).powi(2) + 3.79e-8 * (temp_s - 4.0).powi(3)) * 1000.0;
    let lat = latent_heat_volumetric(temp_a) / row;
    let r_conv = 1.0e6 / 86400.0;
    let heat_flux = rn * r_conv - sens_flux - lat * evap;

    EvaporOut { evap, heat_flux, sens_flux, lat, r_h, rn, rns, rnl, temp_a, row }
}

// ------------------------------------------------------------------------------------
// Daily-generated meteorological data (lMetDaily): DailyMet, DailyTemp, Humidity, SetDayMet
// ------------------------------------------------------------------------------------

/// One daily record with the estimated relative-humidity limits (output of `Humidity`).
#[derive(Clone, Copy, Debug, Default)]
pub struct DailyRec {
    pub t: f64,
    pub rad: f64,
    pub t_max: f64,
    pub t_min: f64,
    pub rh_max: f64,
    pub rh_min: f64,
    pub ea_mean: f64,
    pub wind_ms: f64,
    pub sun_hours: f64,
    pub crop_height: f64,
    pub albedo: f64,
    pub lai: f64,
    pub x_root: f64,
}

/// State of DailyMet: current (`cur`), next (`next`) and previous day (`prev_*`).
#[derive(Clone, Debug, Default)]
pub struct DailyMetState {
    /// Index of the current record (the one with time tAtm) in `MeteoSettings::records`
    pub idx: usize,
    pub cur: DailyRec,
    pub next: DailyRec,
    pub prev_t_max: f64,
    pub prev_t_min: f64,
    pub prev_rh_max: f64,
    pub prev_rh_min: f64,
    /// Air temperature [C] and relative humidity [%] at the current time step
    pub temp_a: f64,
    pub rh_a: f64,
}

/// Humidity: maximum and minimum relative humidity from daily values.
/// Returns `(rh_max, rh_min, ea_mean)`.
pub fn humidity(t_max: f64, t_min: f64, rh_mean_in: f64, i_rel_hum: i32) -> (f64, f64, f64) {
    let mut rh_mean = rh_mean_in;
    let es_tmax = ea(t_max);
    let es_tmin = ea(t_min);
    let (mut rh_max, mut rh_min, ea_mean);
    if i_rel_hum == 0 {
        rh_min = 2.0 * rh_mean * es_tmin / (es_tmax + es_tmin);
        rh_max = 2.0 * rh_mean * es_tmax / (es_tmax + es_tmin);
        ea_mean = rh_min / 100.0 * es_tmax;
    } else {
        ea_mean = rh_mean;
        rh_min = ea_mean / es_tmax * 100.0;
        rh_max = ea_mean / es_tmin * 100.0;
    }
    if rh_max >= 100.0 {
        if i_rel_hum == 1 {
            rh_mean = ea_mean * (50.0 / es_tmin + 50.0 / es_tmax); // Equation 13
        }
        rh_min = rh_mean - (100.0 - rh_mean);
        rh_max = 100.0;
    }
    (rh_max, rh_min, ea_mean)
}

fn daily_rec(mp: &MeteoSettings, i: usize) -> DailyRec {
    let r = &mp.records[i];
    let (rh_max, rh_min, ea_mean) = humidity(r.t_max, r.t_min, r.rh_mean, mp.i_rel_hum);
    DailyRec {
        t: r.t,
        rad: r.rad,
        t_max: r.t_max,
        t_min: r.t_min,
        rh_max,
        rh_min,
        ea_mean,
        wind_ms: r.wind_kmd / 86.4,
        sun_hours: r.sun_hours,
        crop_height: r.crop_height.unwrap_or(mp.crop_height),
        albedo: r.albedo.unwrap_or(mp.albedo),
        lai: r.lai.unwrap_or(mp.lai),
        x_root: r.x_root.unwrap_or(mp.x_root),
    }
}

/// DailyTemp: temperature (or, with swapped arguments, relative humidity) at time `t` from
/// the daily extremes. The maximum occurs at 13:00 and the minimum at 01:00.
#[allow(clippy::too_many_arguments)]
pub fn daily_temp(
    t: f64,
    dt: f64,
    t_init: f64,
    t_end: f64,
    t_atm: f64,
    t_max: f64,
    t_max_n: f64,
    t_max_o: f64,
    t_min: f64,
    t_min_n: f64,
    t_min_o: f64,
) -> f64 {
    let pi = std::f64::consts::PI;
    let max_time = 13.0 / 24.0;
    let min_time = 1.0 / 24.0;
    let tol = dt * 0.001;
    let blend = |mt: f64, cur: f64, next: f64, prev: f64| -> f64 {
        if t < t_init + mt {
            cur
        } else if t >= t_end - 1.0 + mt {
            cur
        } else if t >= t_atm - 1.0 + mt && (t < t_atm || (t - t_atm).abs() <= tol) {
            (next - cur) * (t - (t_atm - 1.0 + mt)) + cur
        } else if (t > t_atm - 1.0 || (t - (t_atm - 1.0)).abs() <= tol) && t < t_atm - 1.0 + mt {
            (cur - prev) * (t - (t_atm - 2.0 + mt)) + prev
        } else {
            cur
        }
    };
    let t_max_a = blend(max_time, t_max, t_max_n, t_max_o);
    let t_min_a = blend(min_time, t_min, t_min_n, t_min_o);
    (t_max_a + t_min_a) / 2.0 + (t_max_a - t_min_a) / 2.0 * (2.0 * pi * (t - max_time)).cos()
}

/// DailyMet: `i_kod` = 1 (initialise: read the first two days), 2 (move to the next day at
/// a record time), 4 (only evaluate TempA / RH_A at time `t`).
///
/// All times are given in project time units; `tt_conv` converts days to project units
/// (86400 * tConv) so that the 13:00 / 01:00 extremes are right for any time unit.
#[allow(clippy::too_many_arguments)]
pub fn daily_met(mp: &MeteoSettings, st: &mut DailyMetState, i_kod: i32, t: f64, dt: f64, t_init: f64, t_end: f64, tt_conv: f64) {
    let (t, dt, t_init, t_end) = (t / tt_conv, dt / tt_conv, t_init / tt_conv, t_end / tt_conv);
    let n = mp.records.len();
    if n == 0 || (t_end - t).abs() <= 0.001 * dt {
        return;
    }
    let mut i_daily = i_kod;
    if ((t_end - 1.0) - t).abs() <= 0.001 * dt {
        i_daily = 3; // last day: do not read further data
    }
    if i_daily == 1 {
        st.idx = 0;
        st.cur = daily_rec(mp, 0);
        st.prev_t_max = st.cur.t_max;
        st.prev_t_min = st.cur.t_min;
        st.prev_rh_max = st.cur.rh_max;
        st.prev_rh_min = st.cur.rh_min;
    }
    if i_daily == 2 || i_daily == 3 {
        st.prev_t_max = st.cur.t_max;
        st.prev_t_min = st.cur.t_min;
        st.prev_rh_max = st.cur.rh_max;
        st.prev_rh_min = st.cur.rh_min;
        st.cur = st.next;
        st.idx = (st.idx + 1).min(n - 1);
    }
    if i_daily <= 2 {
        let ni = st.idx + 1;
        st.next = if ni < n { daily_rec(mp, ni) } else { st.cur };
    }
    if i_daily == 2 {
        return; // do not calculate temperature and RH
    }
    let (c, nx) = (st.cur, st.next);
    st.temp_a = daily_temp(t, dt, t_init, t_end, c.t / tt_conv, c.t_max, nx.t_max, st.prev_t_max, c.t_min, nx.t_min, st.prev_t_min);
    // relative humidity is lowest when the temperature is highest
    st.rh_a = daily_temp(t, dt, t_init, t_end, c.t / tt_conv, c.rh_min, nx.rh_min, st.prev_rh_min, c.rh_max, nx.rh_max, st.prev_rh_max);
}

/// Output of `SetDayMet`.
#[derive(Clone, Copy, Debug, Default)]
pub struct DayMetOut {
    /// Potential evaporation / transpiration [mm/d] (zero when the energy balance is used)
    pub evap_p: f64,
    pub trans_p: f64,
    /// Solar radiation at the current time step [MJ/m2/d] and the (possibly recomputed) daily Rad
    pub rst: f64,
    pub rad: f64,
    pub rns: f64,
    pub rnl: f64,
    pub lai: f64,
    pub scf: f64,
    pub x_root: Option<f64>,
    pub et_comb: f64,
    pub rad_term: f64,
    pub aero_term: f64,
}

/// SetDayMet: potential evapotranspiration (Penman-Monteith) or the radiation terms for the
/// energy balance from the daily-generated temperature and humidity at time `t`.
/// Returns `None` when `Aero` fails.
pub fn set_day_met(mp: &MeteoSettings, st: &DailyMetState, t: f64, t_conv: f64, l_en_bal: bool) -> Option<DayMetOut> {
    let pi = std::f64::consts::PI;
    let tt_conv = 24.0 * 3600.0 * t_conv;
    let t_days = t / tt_conv;
    let cur = st.cur;

    let (crop_height, albedo, lai_in, x_root) = match mp.i_crop {
        2 if !mp.growth.is_empty() => {
            let (h, a, l, x) = growth_table(&mp.growth, t);
            (h, a, l, Some(x))
        }
        3 => (cur.crop_height, cur.albedo, cur.lai, Some(cur.x_root)),
        1 => (mp.crop_height, mp.albedo, mp.lai, if mp.x_root > 0.0 { Some(mp.x_root) } else { None }),
        _ => (mp.crop_height, mp.albedo, mp.lai, None),
    };
    let (rc, scf, lai) = crop_res(mp.i_lai, lai_in, mp.r_extinct, mp.i_crop, crop_height);
    let ch_eff = if mp.i_crop == 0 || crop_height <= 0.0 { 0.1 } else { crop_height };

    let temp_a = st.temp_a;
    let es = ea(temp_a);
    let ea_act = es * st.rh_a / 100.0;

    let day_no = (cur.t / tt_conv).rem_euclid(365.0);
    let mut out = DayMetOut { lai, scf, x_root, rad: cur.rad, ..Default::default() };
    let rn;
    if mp.i_radiation != 2 {
        let (ra, omega, xx, yy) = rad_global(mp.latitude, day_no);
        let rad_cs = ra * (mp.short_wave_a + mp.short_wave_b * 1.0);
        let mut rad = cur.rad;
        if mp.i_radiation == 0 {
            let (_, n_n) = cloudiness(mp.i_sun_sh, cur.sun_hours, omega, mp.long_wave_b, mp.long_wave_a, rad, rad_cs, mp.cloud_fact_ac, mp.cloud_fact_bc);
            rad = ra * (mp.short_wave_a + mp.short_wave_b * n_n); // Equation 52
        }
        // The average of Rst over the 24 hours equals the daily mean Rad
        let mut sum = 0.0;
        for k in 1..=24 {
            let sine1 = xx + yy * (2.0 * pi / 24.0 * (k as f64 - 12.0)).cos();
            sum += sine1.max(0.0) / 24.0;
        }
        let t2 = (t_days - day_no) * 24.0;
        let sine = xx + yy * (2.0 * pi / 24.0 * (t2 - 12.0)).cos();
        let rst = if sum > 0.0 { (sine * rad / sum).max(0.0) } else { 0.0 };
        let rns = (1.0 - albedo) * rst;

        let (mut cloud_f, _) = cloudiness(mp.i_sun_sh, cur.sun_hours, omega, mp.long_wave_b, mp.long_wave_a, rad, rad_cs, mp.cloud_fact_ac, mp.cloud_fact_bc);
        if mp.i_radiation == 1 && mp.i_sun_sh == 3 && rad_cs > 0.0 {
            cloud_f = mp.cloud_fact_ac * rad / rad_cs + mp.cloud_fact_bc; // Equation 57
        }
        let rnl = rad_long_net(temp_a, temp_a, mp.long_wave_a1, mp.long_wave_b1, ea_act, cloud_f);
        rn = rns - rnl;
        out.rst = rst;
        out.rad = rad;
        out.rns = rns;
        out.rnl = rnl;
    } else {
        rn = cur.rad;
    }
    if l_en_bal {
        return Some(out);
    }

    let (aero_term, rad_term) = aero(
        rn,
        rc,
        ch_eff,
        mp.wind_height,
        mp.temp_height,
        cur.wind_ms,
        mp.altitude,
        temp_a,
        temp_a,
        es,
        es,
        ea_act,
    )?;
    let mut etcomb = (rad_term + aero_term).max(0.0);
    let row = (1.0 - 7.37e-6 * (temp_a - 4.0).powi(2) + 3.79e-8 * (temp_a - 4.0).powi(3)) * 1000.0;
    etcomb = etcomb / row * 1000.0; // [kg/m2/d] -> [mm/d]
    out.evap_p = etcomb * (1.0 - scf);
    out.trans_p = etcomb * scf;
    out.et_comb = etcomb;
    out.rad_term = rad_term;
    out.aero_term = aero_term;
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_evapor_outputs_are_finite_and_consistent() {
        let mp = MeteoSettings { i_radiation: 1, i_sun_sh: 3, latitude: 40.0, ..Default::default() };
        for &(t_days, is_hourly) in &[(180.25_f64, true), (180.5, true), (180.75, false), (181.0, false)] {
            let o = evapor(&mp, t_days, t_days, 1.0, 22.0, 30.0, 15.0, 20.0, 0.0, 50.0, 2.0, -100.0, 0.2, is_hourly, 100.0, None);
            assert!(o.evap >= 0.0 && o.evap.is_finite());
            assert!(o.heat_flux.is_finite() && o.rn.is_finite() && o.r_h > 0.0);
            let closure = o.rn * 1.0e6 / 86400.0 - (o.heat_flux + o.sens_flux + o.lat * o.evap);
            assert!(closure.abs() < 1e-9);
        }
    }

    #[test]
    fn test_daily_temp_reaches_daily_extremes() {
        let (tmax, tmin) = (20.0, 10.0);
        let t_hot = 13.0 / 24.0;
        let t_cold = 1.0 / 24.0;
        let a = daily_temp(t_hot, 0.01, 0.0, 10.0, 1.0, tmax, tmax, tmax, tmin, tmin, tmin);
        let b = daily_temp(t_cold, 0.01, 0.0, 10.0, 1.0, tmax, tmax, tmax, tmin, tmin, tmin);
        assert!((a - tmax).abs() < 1e-9, "T(13:00) = {a}");
        assert!((b - tmin).abs() < 1e-9, "T(01:00) = {b}");
    }

    #[test]
    fn test_daily_met_is_independent_of_the_time_unit() {
        let rec = |t: f64, tmax: f64, tmin: f64| MeteoRecord { t, rad: 15.0, t_max: tmax, t_min: tmin, rh_mean: 60.0, wind_kmd: 100.0, sun_hours: 0.0, ..Default::default() };
        let temp = |scale: f64, tt_conv: f64| {
            let mut mp = MeteoSettings::default();
            mp.records = vec![rec(1.0 * scale, 20.0, 10.0), rec(2.0 * scale, 26.0, 12.0), rec(3.0 * scale, 22.0, 9.0)];
            let mut st = DailyMetState::default();
            let (t_end, dt) = (3.0 * scale, 0.01 * scale);
            daily_met(&mp, &mut st, 1, 0.5 * scale, dt, 0.0, t_end, tt_conv);
            daily_met(&mp, &mut st, 2, 1.0 * scale, dt, 0.0, t_end, tt_conv); // day 1 ends
            daily_met(&mp, &mut st, 4, 1.6 * scale, dt, 0.0, t_end, tt_conv);
            (st.temp_a, st.rh_a)
        };
        let (ta_d, rh_d) = temp(1.0, 1.0); // days
        let (ta_h, rh_h) = temp(24.0, 24.0); // hours
        assert!((ta_d - ta_h).abs() < 1e-9 && (rh_d - rh_h).abs() < 1e-9, "{ta_d} vs {ta_h}");
    }

    #[test]
    fn test_humidity_limits() {
        let (rh_max, rh_min, ea_mean) = humidity(25.0, 10.0, 60.0, 0);
        assert!(rh_max <= 100.0 && rh_min >= 0.0 && rh_min < rh_max && ea_mean > 0.0);
        // very humid day: RHMax is capped at 100 % and RHMin moves symmetrically
        let (rh_max, rh_min, _) = humidity(25.0, 10.0, 95.0, 0);
        assert!((rh_max - 100.0).abs() < 1e-12 && (rh_min - 90.0).abs() < 1e-12);
    }

    #[test]
    fn test_snow_partition_and_melting() {
        let x_conv = 100.0; // cm
        let dt = 1.0; // days
        let snow_mf = 0.43; // cm/(day·°C)
        let mut c_top = vec![0.0];
        let c_t = vec![1.0];

        // 1. Below -2°C: All snow, 1.0 sublimated from 10.0 incoming -> 9.0 remaining
        let (p_net, snow_pack, evap_net, _) =
            calculate_snow(10.0, -5.0, dt, snow_mf, 0.0, 1.0, x_conv, &mut c_top, &c_t);
        assert_eq!(p_net, 0.0);
        assert!((snow_pack - 9.0).abs() < 1e-6);
        assert_eq!(evap_net, 0.0);

        // 2. Above 0°C with existing snowpack: Melt degree-day
        let (p_net, snow_pack_2, _, _) =
            calculate_snow(0.0, 5.0, dt, snow_mf, snow_pack, 0.0, x_conv, &mut c_top, &c_t);
        let expected_melt = 5.0 * 0.43 * 1.0;
        assert!((p_net - expected_melt).abs() < 1e-6);
        assert!((snow_pack_2 - (9.0 - expected_melt)).abs() < 1e-6);

        // 3. Above +2°C with no snowpack: Pure rain
        let (p_net, snow_pack_3, evap_net_3, _) =
            calculate_snow(5.0, 10.0, dt, snow_mf, 0.0, 2.0, x_conv, &mut c_top, &c_t);
        assert_eq!(p_net, 5.0);
        assert_eq!(snow_pack_3, 0.0);
        assert_eq!(evap_net_3, 2.0);
    }

    #[test]
    fn test_interception_capacity_and_transpiration_reduction() {
        let mut state = InterceptionState::default();
        let lai = 3.0;
        let a_interc = 0.25;
        let scf = 0.8;
        let prec = 10.0;
        let trans_p = 2.0;

        let (net_prec, net_trans, interc) =
            calculate_interception(prec, trans_p, lai, a_interc, scf, &mut state);

        assert!(net_prec < prec);
        assert!(interc > 0.0);
        assert!(net_trans < trans_p);
        assert!(interc <= a_interc * lai);
    }
	#[test]
    fn test_snow_and_interception_integration_pipeline() {
        let x_conv = 100.0; // cm
        let _t_conv = 1.0 / 86400.0; // days
        let dt = 1.0;

        // 1. Setup a test atmospheric & meteo configuration
        let mut mp = MeteoSettings::default();
        mp.lai = 2.0;
        mp.crop_height = 20.0;
        mp.records = vec![MeteoRecord {
            t: 1.0,
            rad: 15.0,
            t_max: -2.0,
            t_min: -8.0,
            rh_mean: 60.0,
            wind_kmd: 100.0,
            sun_hours: 6.0,
			..Default::default()
        }];

        // Resolve temperature from meteo record: (-2.0 + -8.0) / 2.0 = -5.0°C
        let m_rec = &mp.records[0];
        let temp_air = (m_rec.t_max + m_rec.t_min) / 2.0;
        assert_eq!(temp_air, -5.0);

        let mut prec = 10.0;
        let mut r_soil = 2.0;
        let mut snow_layer = 0.0;
        let snow_mf = 0.43;
        let mut c_top = vec![0.0];
        let c_t = vec![0.0];

        // 2. Snow processing: at -5°C, precipitation turns entirely into snowpack
        let (p_eff, s_layer, evap_eff, _) = calculate_snow(
            prec,
            temp_air,
            dt,
            snow_mf,
            snow_layer,
            r_soil,
            x_conv,
            &mut c_top,
            &c_t,
        );
        prec = p_eff;
        snow_layer = s_layer;
        r_soil = evap_eff;

        // Rain reaching surface should be zero; snowpack absorbs all 10 incoming units (minus 2 sublimated)
        assert_eq!(prec, 0.0);
        assert_eq!(evap_eff, 0.0);
        assert!((snow_layer - 8.0).abs() < 1e-6);

        // 3. Interception processing: with zero throughfall, canopy adds no net precipitation
        let mut interc_state = InterceptionState::default();
        let mut r_root = 1.5;
        let scf = (1.0 - (-0.463f64 * mp.lai).exp()).max(0.0);
        let a_interc = 0.25;

        let (p_net, tr_net, _) = calculate_interception(
            prec,
            r_root,
            mp.lai,
            a_interc,
            scf,
            &mut interc_state,
        );
        prec = p_net;
        r_root = tr_net;

        assert_eq!(prec, 0.0);
        assert_eq!(r_root, 1.5);

        // 4. Net top boundary flux (r_soil - prec)
        let r_top = r_soil.abs() - prec.abs();
        assert_eq!(r_top, 0.0);
    }
	
	#[test]
    fn test_dynamic_crop_overrides_and_snow_interception() {
        let x_conv = 100.0; // cm
        let t_conv = 1.0 / 86400.0; // days
        let dt = 1.0;

        // Base settings with static fallbacks
        let mp = MeteoSettings {
            crop_height: 10.0,
            albedo: 0.25,
            lai: 1.0,
            i_crop: 3, // Daily dynamic values from METEO.IN
            i_lai: 3,
            ..Default::default()
        };

        // Record with dynamic daily overrides (iCrop == 3)
        let rec = MeteoRecord {
            t: 1.0,
            rad: 20.0,
            t_max: 22.0,
            t_min: 12.0,
            rh_mean: 45.0,
            wind_kmd: 120.0,
            sun_hours: 9.0,
            crop_height: Some(50.0), // overrides mp.crop_height
            albedo: Some(0.18),      // overrides mp.albedo
            lai: Some(3.5),          // overrides mp.lai
            x_root: Some(60.0),      // dynamic root depth
        };

        // 1. Verify dynamic potential ET calculation runs with overrides
        let (evap_p, trans_p) = potential_et(&mp, &rec, t_conv);
        assert!(evap_p > 0.0);
        assert!(trans_p > 0.0);

        // 2. Verify dynamic rooting depth extraction
        assert_eq!(rec.x_root, Some(60.0));

        // 3. Verify canopy interception using the dynamic LAI
        let mut interc_state = InterceptionState::default();
        let dynamic_lai = rec.lai.unwrap();
        let scf = (1.0 - (-0.463f64 * dynamic_lai).exp()).max(0.0);
        let prec = 15.0;

        let (net_prec, net_trans, interc) = calculate_interception(
            prec,
            trans_p,
            dynamic_lai,
            0.25,
            scf,
            &mut interc_state,
        );

        assert!(net_prec < prec);
        assert!(interc > 0.0);
        assert!(net_trans <= trans_p);

        // 4. Verify snow degree-day melt with solute mixing slices
        let mut c_top = vec![0.0];
        let c_t = vec![2.0];
        let (p_melt, snow_rem, _, min_step) = calculate_snow(
            0.0,
            4.0, // 4°C melt condition
            dt,
            0.43,
            5.0, // 5 cm snowpack
            0.5, // 0.5 cm evaporation
            x_conv,
            &mut c_top,
            &c_t,
        );

        let expected_melt = 4.0 * 0.43 * dt;
		let expected_snow_rem = 5.0 - expected_melt - 0.5 * dt;
        assert!((p_melt - expected_melt).abs() < 1e-6);
		assert!((snow_rem - expected_snow_rem).abs() < 1e-6);
        assert!(min_step);
    }
	
#[test]
fn test_evapor_energy_balance_and_aero_res() {
    let rv = aero_res(150.0, 200.0, 2.5, 20.0 + 273.15, 18.0 + 273.15);
    assert!(rv > 0.0 && rv < 500.0);

    // Net radiation given (iRadiation = 2), constant air temperature (period = 0)
    let mp = MeteoSettings { i_radiation: 2, wind_height: 200.0, temp_height: 150.0, ..Default::default() };
    let (temp_s, temp_a, rh, rad, theta) = (22.0, 20.0, 50.0, 15.0, 0.20);
    let o = evapor(&mp, 180.5, 180.5, 0.0, temp_s, temp_a, temp_a, rad, 0.0, rh, 2.5, -100.0, theta, false, 100.0, None);
    assert!(o.evap > 0.0);
    assert!(o.sens_flux > 0.0); // soil warmer than air
    assert!((o.temp_a - temp_a).abs() < 1e-12);

    // G + H + LE = Rn with an independently evaluated latent heat
    let row = (1.0 - 7.37e-6 * (temp_s - 4.0).powi(2) + 3.79e-8 * (temp_s - 4.0).powi(3)) * 1000.0;
    let lat = (2.501e6 - 2369.2 * temp_a) * (1.0 - 7.37e-6 * (temp_a - 4.0).powi(2)
        + 3.79e-8 * (temp_a - 4.0).powi(3)) * 1000.0 / row;
    let rn_w = rad * 1e6 / 86400.0;
    assert!((rn_w - (o.heat_flux + o.sens_flux + lat * o.evap)).abs() < 1e-6);
}

	#[test]
    fn test_hourly_radiation_imethour_diurnal_curve() {
        let t_conv = 1.0 / 86400.0; // days
        let mp = MeteoSettings {
            latitude: 33.58,
            altitude: 306.0,
            short_wave_a: 0.25,
            short_wave_b: 0.50,
            i_radiation: 1, // Solar radiation measured
            i_sun_sh: 3,     // Cloudiness from solar radiation
            cloud_fact_ac: 1.35,
            cloud_fact_bc: -0.35,
            records: vec![
                MeteoRecord {
                    t: 328.042, // ~1:00 AM (nighttime)
                    rad: 0.0,
                    t_max: 15.2,
                    t_min: 15.2,
                    rh_mean: 33.0,
                    wind_kmd: 138.24,
                    sun_hours: 0.545,
                    ..Default::default()
                },
                MeteoRecord {
                    t: 328.083, // ~2:00 AM (sub-daily interval confirms is_hourly = true)
                    rad: 0.0,
                    t_max: 14.9,
                    t_min: 14.9,
                    rh_mean: 32.0,
                    wind_kmd: 103.68,
                    sun_hours: 0.545,
                    ..Default::default()
                },
                MeteoRecord {
                    t: 328.500, // 12:00 PM (solar noon)
                    rad: 43.2864,
                    t_max: 29.6,
                    t_min: 29.6,
                    rh_mean: 14.0,
                    wind_kmd: 198.72,
                    sun_hours: 0.545,
                    ..Default::default()
                },
            ],
            ..Default::default()
        };

        // 1. Nighttime record (rad = 0.0): should evaluate nighttime cloudiness and minimal/zero ET
        let (evap_night, trans_night) = potential_et(&mp, &mp.records[0], t_conv);
        assert!(evap_night >= 0.0);
        assert!(trans_night >= 0.0);

        // 2. Solar noon record (rad = 43.28 MJ/m2/d): should compute positive daytime ET
        let (evap_noon, trans_noon) = potential_et(&mp, &mp.records[2], t_conv);
        assert!(evap_noon > evap_night);
        assert!(trans_noon >= 0.0);
    }
	
	#[test]
    fn test_lai_partitioning_formula() {
        let r_pet: f64 = 0.5; // cm/day total potential evapotranspiration
        let lai: f64 = 2.5;
        let r_extinct: f64 = 0.463;

        // SC = 1 - exp(-k * LAI)
        let sc: f64 = 1.0 - (-r_extinct * lai).exp();
        let expected_r_root: f64 = r_pet * sc;
        let expected_r_soil: f64 = r_pet - expected_r_root;

        assert!((expected_r_root + expected_r_soil - r_pet).abs() < 1e-10);
        assert!(expected_r_root > 0.0 && expected_r_soil > 0.0);
        // At LAI 2.5 with k=0.463, roughly 68.5% goes to transpiration and 31.5% to soil evaporation
        assert!((sc - 0.685).abs() < 0.01);
    }

    #[test]
    fn test_cyclic_boundary_time_wrapping() {
        // Simulating a 24-hour cycle (t_period = 1.0 day) repeated over 3 days
        let t_period: f64 = 1.0;
        let t_init: f64 = 0.0;

        let eval_times: [f64; 3] = [0.25, 1.25, 2.25];
        let wrapped_times: Vec<f64> = eval_times
            .iter()
            .map(|&t: &f64| t_init + (t - t_init).rem_euclid(t_period))
            .collect();

        assert!((wrapped_times[0] - 0.25).abs() < 1e-12);
        assert!((wrapped_times[1] - 0.25).abs() < 1e-12);
        assert!((wrapped_times[2] - 0.25).abs() < 1e-12);
    }
	#[test]
	fn test_boundary_condition_cycling_execution() {
		use crate::model::*;
		use crate::sim::Simulation;

		let mut prj = Project::default();
		prj.time.t_init = 0.0;
		prj.time.t_max = 10.0;
		prj.time.dt = 0.5;
		prj.time.dt_min = 0.01;
		prj.time.dt_max = 0.5;

		prj.water.bc.atmospheric = true;
		prj.water.bc.top_time_variable = true;
		prj.atmosphere.bc_cycles = true;

		// Define a 2-day repeating cycle of precipitation
		prj.atmosphere.records = vec![
			AtmRecord {
				t: 1.0,
				prec: 2.0,
				evap: 0.1,
				transp: 0.0,
				h_crit_a: 100000.0,
				..Default::default()
			},
			AtmRecord {
				t: 2.0,
				prec: 0.0,
				evap: 0.5,
				transp: 0.0,
				h_crit_a: 100000.0,
				..Default::default()
			},
		];

		let mut sim = Simulation::new(prj).expect("Simulation should initialize");

		// Run across multiple cycles up to t = 10.0
		let mut steps = 0;
		while !sim.done && steps < 200 {
			sim.step();
			steps += 1;
		}

		// dt is REAL*4 and t double (TmCont), so the run ends within 0.5*dtMin of tMax (lEnd), not exactly at it.
		assert!(sim.done && (sim.t - 10.0).abs() <= 0.5 * 0.01, "Simulation should reach t = 10.0 via repeating cycles (t = {})", sim.t);
		assert!(!sim.failed);
	}
}