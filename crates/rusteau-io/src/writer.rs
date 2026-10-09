//! Result export (CSV) — one file per HYDRUS output group.

use rusteau_core::{HydrusError, Results};
use std::fmt::Write as _;
use std::path::Path;
use rusteau_core::output::InverseOutputData;
use rusteau_core::inverse::OptimizationState;

fn w(dir: &Path, name: &str, s: String) -> Result<(), HydrusError> {
    std::fs::write(dir.join(name), s).map_err(|e| HydrusError::Io(format!("{}: {}", name, e)))
}

pub fn tlevel_csv(r: &Results) -> String {
    let mut s = String::new();
    s.push_str("time,dt,tlevel,iter_w,it_cum,kod_top,kod_bot,rTop,rRoot,vTop,vRoot,vBot,sum_rTop,sum_rRoot,sum_vTop,sum_vRoot,sum_vBot,hTop,hRoot,hBot,runoff,sum_runoff,volume,sum_infil,sum_evap,precip,snow_layer");
    for j in 0..r.n_solutes {
        let _ = write!(s, ",cvTop{0},cvBot{0},sum_cvTop{0},sum_cvBot{0},sum_cvCh0_{0},sum_cvCh1_{0},cTop{0},cRoot{0},cBot{0},cvRoot{0},sum_cvRoot{0},sum_cvNEql{0},cGWL{0},cRunOff{0},sum_cRunOff{0}", j + 1);
    }
    s.push_str(",temp_top,temp_bot,peclet,courant,cum_w_transf\n");
    for t in &r.tlevel {
        let _ = write!(
            s,
            "{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{}",
            t.t, t.dt, t.tlevel, t.iter_w, t.it_cum, t.kod_top, t.kod_bot, t.r_top, t.r_root, t.v_top, t.v_root, t.v_bot, t.cum_r_top, t.cum_r_root, t.cum_v_top, t.cum_v_root, t.cum_v_bot,
            t.h_top, t.h_root, t.h_bot, t.run_off, t.cum_run_off, t.volume, t.cum_infil, t.cum_evap, t.precip, t.snow_layer
        );
        for c in &t.solutes {
            let _ = write!(
                s,
                ",{},{},{},{},{},{},{},{},{},{},{},{},{},{},{}",
                c.cv_top, c.cv_bot, c.cum_top, c.cum_bot, c.cum_ch0, c.cum_ch1, c.c_top, c.c_root, c.c_bot, c.cv_root, c.cum_root, c.cum_neq, c.c_gwl, c.c_run_off, c.cum_run_off
            );
        }
        let _ = writeln!(s, ",{},{},{},{},{}", t.temp_top, t.temp_bot, t.peclet, t.courant, t.cum_w_transf);
    }
    s
}

pub fn profiles_csv(r: &Results) -> String {
    let mut s = String::from("time,node,depth,head,theta,K,C,flux,sink,kappa,v_over_Ks,temp,h_matrix,th_matrix");
    for j in 0..r.n_solutes {
        let _ = write!(s, ",conc{0},sorb{0},conc_matrix{0},flux_conc{0}", j + 1);
    }
    s.push('\n');
    for p in &r.profiles {
        for n in &p.nodes {
            let h_m_str = n.h_matrix.map(|v| v.to_string()).unwrap_or_default();
            let th_m_str = n.th_matrix.map(|v| v.to_string()).unwrap_or_default();
            let _ = write!(
                s,
                "{},{},{},{},{},{},{},{},{},{},{},{},{},{}",
                p.t, n.node, n.depth, n.h, n.theta, n.k, n.c, n.flux, n.sink, n.kappa, n.v_over_ks, n.temp, h_m_str, th_m_str
            );
            for j in 0..r.n_solutes {
                let c_val = n.conc.get(j).copied().unwrap_or(0.0);
                let s_val = n.sorb.get(j).copied().unwrap_or(0.0);
                let cm_val = n.conc_matrix.as_ref().and_then(|m| m.get(j)).map(|v| v.to_string()).unwrap_or_default();
                let fc_val = n.flux_conc.as_ref().and_then(|f| f.get(j)).map(|v| v.to_string()).unwrap_or_default();
                let _ = write!(s, ",{},{},{},{}", c_val, s_val, cm_val, fc_val);
            }
            s.push('\n');
        }
    }
    s
}

pub fn obs_csv(r: &Results) -> String {
    let mut s = String::from("time,node,head,theta,temp,flux,h_matrix,th_matrix");
    for j in 0..r.n_solutes {
        let _ = write!(s, ",conc{0},conc_matrix{0},flux_conc{0}", j + 1);
    }
    s.push('\n');
    for o in &r.obs {
        for p in &o.points {
            let h_m_str = p.h_matrix.map(|v| v.to_string()).unwrap_or_default();
            let th_m_str = p.th_matrix.map(|v| v.to_string()).unwrap_or_default();
            let _ = write!(s, "{},{},{},{},{},{},{},{}", o.t, p.node, p.h, p.theta, p.temp, p.flux, h_m_str, th_m_str);
            for j in 0..r.n_solutes {
                let c_val = p.conc.get(j).copied().unwrap_or(0.0);
                let cm_val = p.conc_matrix.as_ref().and_then(|m| m.get(j)).map(|v| v.to_string()).unwrap_or_default();
                let fc_val = p.flux_conc.as_ref().and_then(|f| f.get(j)).map(|v| v.to_string()).unwrap_or_default();
                let _ = write!(s, ",{},{},{}", c_val, cm_val, fc_val);
            }
            s.push('\n');
        }
    }
    s
}

pub fn balance_csv(r: &Results) -> String {
    let mut s = String::from("time,area,volume,inflow,h_mean,top_flux,bot_flux,WatBalT,WatBalR_percent\n");
    for b in &r.balance {
        let _ = writeln!(s, "{},{},{},{},{},{},{},{},{}", b.t, b.total.area, b.total.volume, b.total.change, b.total.h_mean, b.top_flux, b.bot_flux, b.wat_bal_t, b.wat_bal_r);
    }
    s
}

pub fn alevel_csv(r: &Results) -> String {
    let mut s = String::from("time,sum_rTop,sum_rRoot,sum_vTop,sum_vRoot,sum_vBot,hTop,hRoot,hBot\n");
    for a in &r.alevel {
        let _ = writeln!(s, "{},{},{},{},{},{},{},{},{}", a.t, a.cum[0], a.cum[1], a.cum[2], a.cum[3], a.cum[4], a.h_top, a.h_root, a.h_bot);
    }
    s
}

/// Generate FIT.OUT
pub fn fit_out_string(inv: &InverseOutputData) -> String {
    let mut s = String::new();
    let _ = writeln!(s, "************************************************************************");
    let _ = writeln!(s, " * Title: {:<60} *", inv.title);
    let _ = writeln!(s, " ************************************************************************");
    let _ = writeln!(s, "Number of iterations performed: {}", inv.iterations_run);
    let _ = writeln!(s, "Final Sum of Squared Residuals (SSQ): {:.6e}", inv.final_ssq);
    let _ = writeln!(s, "------------------------------------------------------------------------");
    let _ = writeln!(s, "------------Observed & Fitted Quantity -----------------------------");
    let _ = writeln!(s, "                                               Resi-");
    let _ = writeln!(s, "    No     Time           Obs        Fitted    dual   Type  Position");

    for (i, obs) in inv.observations.iter().enumerate() {
        let sim = inv.simulated_values.get(i).copied().unwrap_or(0.0);
        let residual = obs.value - sim;
        let _ = writeln!(
            s,
            "   {:<4}   {:<10.3}   {:<10.3}   {:<10.3}   {:<8.3}   {:<3}   {:<4}",
            obs.id, obs.time, obs.value, sim, residual, obs.q_type, obs.position
        );
    }

    let _ = writeln!(s, "------------------------------------------------------------------------");
    s
}

pub fn write_fit_out(
    dir: &Path,
    project_title: &str,
    opt_state: &OptimizationState,
) -> Result<(), HydrusError> {
    std::fs::create_dir_all(dir).map_err(|e| HydrusError::Io(e.to_string()))?;
    
    let inv = InverseOutputData {
        title: project_title.to_string(),
        final_ssq: opt_state.current_ssq,
        iterations_run: opt_state.iterations_run,
        observations: opt_state.settings.observations.clone(),
        simulated_values: opt_state.simulated_values.clone(),
        r_squared: opt_state.r_squared,
        parameters: opt_state.parameters.clone(),
        correlation_matrix: opt_state.correlation_matrix.clone(),
    };
    
    let content = fit_out_string(&inv);
    w(dir, "FIT.OUT", content)
}

/// T_Level1 (lFluxOut): potential / actual fluxes and the cumulative values.
pub fn tlevel1_csv(r: &Results) -> String {
    let mut s = String::from(
        "time,PrecipP,EvaporP,FluxTopP,FluxTopA,InfiltrA,EvaporA,TranspP,TranspA,FluxBot,RunOff,\
Sum(PrecP),Sum(EvapP),Sum(FlTopP),Sum(FlTopA),Sum(InfA),Sum(EvapA),Sum(TransP),Sum(TransA),Sum(FluxBot),Sum(RunOff),\
Storage,SnowLayer,vTopW,vTopV\n",
    );
    for t in &r.tlevel1 {
        let _ = writeln!(
            s,
            "{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{}",
            t.t, t.prec, t.r_soil, t.r_top, t.v_top, t.r_infil, t.r_evap, t.r_root, t.v_root, t.v_bot, t.run_off,
            t.cum_prec, t.cum_evap_p, t.cum_r_top, t.cum_v_top, t.cum_infil, t.cum_evap, t.cum_r_root,
            t.cum_v_root, t.cum_v_bot, t.cum_run_off, t.volume, t.snow_layer, t.v_top_w, t.v_top_v
        );
    }
    s
}

/// Fortran `e13.5` edit descriptor.
fn fe13_5(v: f64) -> String {
    let (sign, a) = if v < 0.0 { ("-", -v) } else { ("", v) };
    let (mut mant, mut exp) = (0.0, 0i32);
    if a > 0.0 {
        exp = a.log10().floor() as i32 + 1;
        mant = a / 10f64.powi(exp);
        if (mant * 1e5).round() >= 1e5 {
            mant /= 10.0;
            exp += 1;
        }
    }
    let digits = (mant * 1e5).round() as u64;
    // Intel Fortran writes e13.5 without the leading zero: ".32800E+03", "-.72580E+01"
    let body = format!("{}.{:05}E{}{:02}", sign, digits, if exp < 0 { '-' } else { '+' }, exp.abs());
    format!("{:>13}", body)
}

/// Meteo.out in the fixed-width layout of the Fortran code (header formats of MeteoIn, data formats of
/// SetMeteo / DayMeteoOut / UpdateEnergy, closing `end` line).
pub fn meteo_out_text(r: &Results) -> String {
    let mut s = String::new();
    let (h1, h2) = match r.meteo_mode {
        0 => (
            "   Time       ET        Evap    Transp     Rns        Rnl    RadTerm  AeroTerm     Prec     Interc   ExInterc",
            "    [d]     [mm/d]    [mm/d]    [mm/d] [MJ/m2/d]  [MJ/m2/d]   [mm/d]    [mm/d]    [mm/d]    [mm/d]    [mm/d]",
        ),
        1 => (
            "   Time       ET        Evap    Transp     Rns        Rnl    RadTerm  AeroTerm     Prec     Interc   ExInterc   AirTempAirRH   SolarRad",
            "    [d]     [mm/d]    [mm/d]    [mm/d] [MJ/m2/d]  [MJ/m2/d]   [mm/d]    [mm/d]    [mm/d]    [mm/d]    [mm/d]      [C][%]    [MJ/m2/d]",
        ),
        2 => (
            "     Time      Short.Rad.    Long.Rad.    Radiation    Sensible      Latent     Heat Flux     Balance",
            "      [d]       [MJ/m2/d]    [MJ/m2/d]    [MJ/m2/d]    [MJ/m2/d]   [MJ/m2/d]    [MJ/m2/d]    [MJ/m2/d]",
        ),
        _ => (
            "     Time      Short.Rad.    Long.Rad.    Radiation    Sensible      Latent     Heat Flux     Balance        AirTempAirRH      SolarRad",
            "      [d]       [MJ/m2/d]    [MJ/m2/d]    [MJ/m2/d]    [MJ/m2/d]   [MJ/m2/d]    [MJ/m2/d]    [MJ/m2/d]         [C] [%]       [MJ/m2/d]",
        ),
    };
    let _ = writeln!(s);
    let _ = writeln!(s, "{}", h1);
    let _ = writeln!(s, "{}", h2);
    let _ = writeln!(s);
    for m in &r.meteo {
        match r.meteo_mode {
            0 => {
                let _ = writeln!(s, "{:8.2}{:10.3}{:10.3}{:10.3}{:10.3}{:10.3}{:10.3}{:10.3}{:10.3}{:10.3}{:10.3}", m.t, m.et, m.evap_p, m.transp_p, m.rns, m.rnl, m.rad_term, m.aero_term, m.prec, m.interc, m.ex_interc);
            }
            1 => {
                let _ = writeln!(s, "{:8.2}{:10.3}{:10.3}{:10.3}{:10.3}{:10.3}{:10.3}{:10.3}{:10.3}{:10.3}{:10.3}{:10.3}{:10.3}{:10.3}", m.t, m.et, m.evap_p, m.transp_p, m.rns, m.rnl, m.rad_term, m.aero_term, m.prec, m.interc, m.ex_interc, m.air_temp, m.air_rh, m.solar_rad);
            }
            _ => {
                let mut vals = vec![m.t, m.rns, m.rnl, m.rn, m.sensible, m.latent, m.heat_flux, m.balance];
                if r.meteo_mode == 3 {
                    vals.extend([m.air_temp, m.air_rh, m.solar_rad]);
                }
                let line: String = vals.into_iter().map(fe13_5).collect();
                let _ = writeln!(s, "{}", line);
            }
        }
    }
    let _ = writeln!(s, "end");
    s
}

/// Meteo.out: column set depends on `Results::meteo_mode` (see `MeteoRow`).
pub fn meteo_csv(r: &Results) -> String {
    let mut s = String::new();
    let daily = r.meteo_mode == 1 || r.meteo_mode == 3;
    match r.meteo_mode {
        0 | 1 => s.push_str("Time[d],ET[mm/d],Evap[mm/d],Transp[mm/d],Rns[MJ/m2/d],Rnl[MJ/m2/d],RadTerm[mm/d],AeroTerm[mm/d],Prec[mm/d],Interc[mm/d],ExInterc[mm/d]"),
        _ => s.push_str("Time[d],ShortRad[MJ/m2/d],LongRad[MJ/m2/d],Radiation[MJ/m2/d],Sensible[MJ/m2/d],Latent[MJ/m2/d],HeatFlux[MJ/m2/d],Balance[MJ/m2/d]"),
    }
    if daily {
        s.push_str(",AirTemp[C],AirRH[%],SolarRad[MJ/m2/d]");
    }
    s.push('\n');
    for m in &r.meteo {
        let _ = match r.meteo_mode {
            0 | 1 => write!(s, "{},{},{},{},{},{},{},{},{},{},{}", m.t, m.et, m.evap_p, m.transp_p, m.rns, m.rnl, m.rad_term, m.aero_term, m.prec, m.interc, m.ex_interc),
            _ => write!(s, "{},{},{},{},{},{},{},{}", m.t, m.rns, m.rnl, m.rn, m.sensible, m.latent, m.heat_flux, m.balance),
        };
        if daily {
            let _ = write!(s, ",{},{},{}", m.air_temp, m.air_rh, m.solar_rad);
        }
        s.push('\n');
    }
    s
}

/// Triggered irrigation events (Irrig.out): trigger time, beginning, end and amount.
pub fn irrig_csv(r: &Results) -> String {
    let mut s = String::from("event,t_trigger,t_start,t_end,amount\n");
    for (k, e) in r.irrigation.iter().enumerate() {
        let _ = writeln!(s, "{},{},{},{},{}", k + 1, e.t_trigger, e.t_start, e.t_end, e.amount);
    }
    s
}

/// Write all result tables as CSV files into `dir`.
pub fn write_all(r: &Results, dir: &Path) -> Result<(), HydrusError> {
    std::fs::create_dir_all(dir).map_err(|e| HydrusError::Io(e.to_string()))?;
    w(dir, "T_LEVEL.csv", tlevel_csv(r))?;
    w(dir, "NOD_INF.csv", profiles_csv(r))?;
    w(dir, "OBS_NODE.csv", obs_csv(r))?;
    w(dir, "BALANCE.csv", balance_csv(r))?;
    w(dir, "A_LEVEL.csv", alevel_csv(r))?;
    if !r.tlevel1.is_empty() {
        w(dir, "T_LEVEL1.csv", tlevel1_csv(r))?;
    }
    if !r.meteo.is_empty() {
        w(dir, "METEO.csv", meteo_csv(r))?;
        w(dir, "Meteo.out", meteo_out_text(r))?;
    }
    if !r.irrigation.is_empty() {
        w(dir, "IRRIG.csv", irrig_csv(r))?;
    }
	if let Some(inv) = &r.inverse_results {
        let content = fit_out_string(inv);
        w(dir, "FIT.OUT", content)?;
    }
    Ok(())
}

#[cfg(test)]
mod meteo_out_tests {
    use super::*;

    #[test]
    fn fortran_e13_5_format() {
        assert_eq!(fe13_5(0.0), "   .00000E+00");
        assert_eq!(fe13_5(1.0), "   .10000E+01");
        assert_eq!(fe13_5(-123.456), "  -.12346E+03");
        assert_eq!(fe13_5(0.000123), "   .12300E-03");
        assert_eq!(fe13_5(9.999996), "   .10000E+02");
    }

    #[test]
    fn meteo_out_layout() {
        let mut r = Results::default();
        r.meteo_mode = 0;
        r.meteo.push(rusteau_core::output::MeteoRow { t: 1.0, et: 2.5, ..Default::default() });
        let txt = meteo_out_text(&r);
        let lines: Vec<&str> = txt.lines().collect();
        assert_eq!(lines[0], "");
        assert!(lines[1].starts_with("   Time       ET"));
        assert_eq!(lines[3], "");
        assert_eq!(lines[4], "    1.00     2.500     0.000     0.000     0.000     0.000     0.000     0.000     0.000     0.000     0.000");
        assert_eq!(*lines.last().unwrap(), "end");
    }
}

#[cfg(test)]
mod meteo_sample_tests {
    use super::*;
    use rusteau_core::output::MeteoRow;

    /// A real Meteo.out of the Fortran code (energy balance, 1217 time steps): the writer must reproduce it byte for byte.
    #[test]
    fn meteo_out_reproduces_the_fortran_sample() {
        // the sample is not part of the source package: without it the test is skipped
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/data/Meteo_sample.out");
        let Ok(sample) = std::fs::read_to_string(&path) else {
            eprintln!("skipped: {} not found", path.display());
            return;
        };
        let sample = sample.replace("\r\n", "\n");
        let mut r = Results::default();
        r.meteo_mode = 2;
        for v in sample.lines().filter_map(crate::reference::parse_numbers).filter(|v| v.len() == 8) {
            r.meteo.push(MeteoRow { t: v[0], rns: v[1], rnl: v[2], rn: v[3], sensible: v[4], latent: v[5], heat_flux: v[6], balance: v[7], ..Default::default() });
        }
        assert_eq!(r.meteo.len(), 1217);
        let txt = meteo_out_text(&r);
        for (i, (a, b)) in txt.lines().zip(sample.lines()).enumerate() {
            assert_eq!(a, b, "line {} differs", i + 1);
        }
        assert_eq!(txt.lines().count(), sample.lines().count());
    }
}
