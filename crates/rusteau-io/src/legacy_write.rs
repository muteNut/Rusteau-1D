//! Writer for HYDRUS-1D 4.x projects ("legacy save"): the input files of a project folder (SELECTOR.IN, PROFILE.DAT,
//! ATMOSPH.IN, METEO.IN, FIT.IN, MATER.IN, MoistDep.in, Hysteresis.in, HYDRUS1D.DAT, DESCRIPT.TXT) and the project file `<name>.h1d`
//! next to the folder `<name>/`, the layout HYDRUS-1D 4.17 opens.
//!
//! The files follow the layout HYDRUS-1D 4.17 writes (file version 4, the header lines of 4.17, Windows line ends);
//! everything [`crate::legacy::read_legacy_project`] reads is written, so a project read back from the files equals
//! the saved one. Settings HYDRUS-1D cannot store (Rusteau extensions) are listed in the returned warnings.

use crate::fit::write_fit_in;
use rusteau_core::*;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

type R<T> = Result<T, HydrusError>;

/// The HYDRUS-1D 4.17 project file `<name>.h1d`: an OLE compound document with the window layout of the graphical
/// interface only (the model data are in the project folder). The same bytes serve every project.
const H1D_TEMPLATE: &[u8] = include_bytes!("../data/template.h1d");

/// Number as HYDRUS reads it (list-directed): the shortest representation that reads back to the same value.
fn g(v: f64) -> String {
    if v == 0.0 || !v.is_finite() {
        return "0".into();
    }
    let a = v.abs();
    if (1e-4..1e7).contains(&a) {
        format!("{v}")
    } else {
        format!("{v:e}")
    }
}

fn lg(b: bool) -> &'static str {
    if b {
        "t"
    } else {
        "f"
    }
}

/// Right-aligned fields of width `w`, separated by a blank.
fn row(vals: &[String], w: usize) -> String {
    vals.iter().map(|v| format!(" {v:>w$}")).collect::<String>()
}

fn nums(v: &[f64]) -> Vec<String> {
    v.iter().map(|&x| g(x)).collect()
}

/// `%15.6e` of the Microsoft C runtime (three exponent digits), as in PROFILE.DAT.
fn e3(v: f64) -> String {
    let s = format!("{:.6e}", v);
    let (m, e) = s.split_once('e').unwrap_or((&s, "0"));
    let ev: i32 = e.parse().unwrap_or(0);
    format!("{:>15}", format!("{m}e{}{:03}", if ev < 0 { '-' } else { '+' }, ev.abs()))
}

/// `2.3E+02`, `5.E+01`: the number format of HYDRUS1D.DAT.
fn e_ini(v: f64) -> String {
    if v == 0.0 || !v.is_finite() {
        return "0".into();
    }
    let mut ex = v.abs().log10().floor() as i32;
    let mut m = v / 10f64.powi(ex);
    if m.abs() >= 9.999_999_5 {
        m /= 10.0;
        ex += 1;
    }
    let mut ms = format!("{m:.6}");
    while ms.ends_with('0') {
        ms.pop();
    }
    format!("{ms}E{}{:02}", if ex < 0 { '-' } else { '+' }, ex.abs())
}

/// CRLF line ends (HYDRUS-1D is a Windows program).
fn crlf(s: &str) -> String {
    s.replace("\r\n", "\n").replace('\n', "\r\n")
}

fn write_file(dir: &Path, name: &str, text: &str, files: &mut Vec<PathBuf>) -> R<()> {
    let p = dir.join(name);
    std::fs::write(&p, crlf(text)).map_err(|e| HydrusError::Io(format!("{}: {}", p.display(), e)))?;
    files.push(p);
    Ok(())
}

fn time_unit(u: TimeUnit) -> &'static str {
    match u {
        TimeUnit::Sec => "sec",
        TimeUnit::Min => "min",
        TimeUnit::Hours => "hours",
        TimeUnit::Days => "days",
        TimeUnit::Years => "years",
    }
}

fn length_unit(u: LengthUnit) -> &'static str {
    match u {
        LengthUnit::Mm => "mm",
        LengthUnit::Cm => "cm",
        LengthUnit::M => "m",
    }
}

/// Number of material parameters per line of SELECTOR.IN (as read back).
fn n_water_par(prj: &Project) -> usize {
    if prj.water.hysteresis != Hysteresis::None {
        return 10;
    }
    match prj.water.model {
        SoilModel::ModifiedVG => 10,
        SoilModel::Durner | SoilModel::DualPorosityW => 9,
        SoilModel::DualPorosityH | SoilModel::DualPermeability => 11,
        _ => 6,
    }
}

fn water_header(prj: &Project) -> &'static str {
    if prj.water.hysteresis != Hysteresis::None {
        return "   thr     ths    Alfa      n         Ks       l      thm     thsW   AlfaW       KsW";
    }
    match prj.water.model {
        SoilModel::ModifiedVG => "   thr     ths    Alfa      n         Ks       l      thm     tha        thk      Kk",
        SoilModel::Durner => "   thr     ths    Alfa      n         Ks       l       w2    Alfa2       n2",
        SoilModel::DualPorosityW => "   thr     ths    Alfa      n         Ks       l     thrIm   thsIm      Omega",
        SoilModel::DualPorosityH => "   thr     ths    Alfa      n         Ks       l     thrIm   thsIm     AlfaIm     nIm      Omega",
        SoilModel::DualPermeability => "   thr     ths    Alfa      n         Ks       l     thrM    thsM      AlfaM      nM        KsM",
        _ => "   thr     ths    Alfa      n         Ks       l",
    }
}

/// Root depth column in ATMOSPH.IN (root growth from the atmospheric records).
fn atm_root_depth(prj: &Project) -> bool {
    prj.processes.root_growth && matches!(prj.root.growth, RootGrowthMode::FromAtmosphere)
}

fn needs_atmosph(prj: &Project) -> bool {
    let bc = &prj.water.bc;
    bc.top_time_variable || bc.bot_time_variable || bc.atmospheric
}

/// Settings of the project that the HYDRUS-1D files cannot hold.
pub fn legacy_save_warnings(prj: &Project) -> Vec<String> {
    let mut w = vec![];
    if prj.water.model == SoilModel::DualPermeability {
        w.push("The interface parameters of the dual-permeability model are Rusteau settings and are not saved.".into());
    }
    if prj.water.bc.irrigation.as_ref().is_some_and(|i| i.mode == IrrigationMode::FullDuration) {
        w.push("Triggered irrigation: the mode 'full duration' is a Rusteau setting; HYDRUS-1D uses its own rule.".into());
    }
    if prj.processes.solute {
        if prj.solute.i_moist_dep == 2 {
            w.push("The trapezoidal water-content dependence of the reactions (MoistDep.in) is a Rusteau setting; the tables are written to MoistDep.in, but HYDRUS-1D does not use them.".into());
        }
        if prj.solute.i_conc_type == 2 {
            w.push("Flux-averaged output concentrations are a Rusteau setting and are not saved.".into());
        }
    }
    if prj.time.flux_out {
        w.push("The extra flux output (T_Level1) is a Rusteau setting and is not saved.".into());
    }
    if prj.time.print_times.is_empty() {
        w.push("HYDRUS-1D needs at least one print time: the final time is written as print time.".into());
    }
    if !prj.water.use_tables {
        w.push("HYDRUS-1D always interpolates the hydraulic functions from tables; the analytic evaluation is a Rusteau setting.".into());
    }
    if !prj.options.is_default() {
        w.push(
            "The further options of the HYDRUS code (head extrapolation, vapor enhancement factor, non-equilibrium flow, centrifuge, density-dependent flow) are not saved: HYDRUS-1D does not read Options.in."
                .into(),
        );
    }
    if prj.inverse.as_ref().is_some_and(|s| !s.hydrus_stop) && prj.processes.inverse {
        w.push("The Rusteau iteration of the parameter estimation (frozen time levels, several starting points) is not stored; HYDRUS-1D runs its own.".into());
    }
    if !prj.figures.is_empty() {
        w.push(format!("The {} figure(s) of the figure editor are stored only in the Rusteau project (.h1dr).", prj.figures.len()));
    }
    w
}

/// `Hysteresis.in` (HysterIn of INPUT.FOR): two header lines, then `n  Theta  Kappa` for every node, the bottom
/// node first (HysterIn assigns the lines to the nodes in the internal order and ignores `n`). `None` without an
/// initial hysteresis state or without Kool & Parker hysteresis.
pub fn hysteresis_text(prj: &Project) -> Option<String> {
    let st = prj.profile.hysteresis.as_ref()?;
    if !prj.water.hysteresis.is_kool_parker() || st.len() != prj.profile.nodes.len() {
        return None;
    }
    let mut o = String::new();
    let _ = writeln!(o, "Hysteresis.in - initial water contents and branches (Kappa: -1 drying, 1 wetting), bottom node first");
    let _ = writeln!(o, "    n           Theta  Kappa");
    for (k, h) in st.iter().rev().enumerate() {
        let _ = writeln!(o, "{:5} {:>15} {:6}", k + 1, g(h.theta), if h.kappa > 0 { 1 } else { -1 });
    }
    Some(o)
}

/// SELECTOR.IN.
pub fn selector_text(prj: &Project) -> String {
    let mut o = String::new();
    let p = &prj.processes;
    let wf = &prj.water;
    let bc = &wf.bc;
    let n_mat = wf.materials.len();
    let n_lay = prj.profile.nodes.iter().map(|n| n.layer).max().unwrap_or(1).max(1);
    let ns = prj.n_solutes();
    let meteo = prj.atmosphere.meteo.is_some();
    let _ = writeln!(o, "Pcp_File_Version=4");
    let _ = writeln!(o, "*** BLOCK A: BASIC INFORMATION *****************************************");
    let _ = writeln!(o, "Heading");
    let _ = writeln!(o, "{}", prj.title.lines().next().unwrap_or(""));
    let _ = writeln!(o, "LUnit  TUnit  MUnit  (indicated units are obligatory for all input data)");
    let _ = writeln!(o, "{}", length_unit(prj.units.length));
    let _ = writeln!(o, "{}", time_unit(prj.units.time));
    let _ = writeln!(o, "{}", if prj.units.mass.trim().is_empty() { "-" } else { prj.units.mass.trim() });
    let _ = writeln!(o, "lWat   lChem lTemp  lSink lRoot lShort lWDep lScreen lVariabBC lEquil lInverse");
    let _ = writeln!(
        o,
        " {}     {}     {}      {}     {}     {}      {}     t       {}         {}         {}",
        lg(p.water_flow),
        lg(p.solute),
        lg(p.heat),
        lg(p.root_water_uptake),
        lg(p.root_growth),
        lg(p.short_output),
        lg(wf.l_w_dep),
        lg(bc.atmospheric),
        lg(p.equilibrium_adsorption),
        lg(p.inverse)
    );
    let _ = writeln!(o, "lSnow  lHP1   lMeteo  lVapor lActiveU lFluxes lIrrig  lDummy  lDummy  lDummy");
    let _ = writeln!(
        o,
        " {}       f       {}       {}       {}       f       {}       f       f       f",
        lg(prj.atmosphere.snow),
        lg(meteo),
        lg(p.vapor),
        lg(prj.root.l_act_rsu),
        lg(bc.irrigation.is_some())
    );
    let _ = writeln!(o, "NMat    NLay  CosAlpha");
    let _ = writeln!(o, "  {}       {}       {}", n_mat, n_lay, g(prj.cos_alpha));

    // ---------------- B: water flow
    let _ = writeln!(o, "*** BLOCK B: WATER FLOW INFORMATION ************************************");
    let _ = writeln!(o, "MaxIt   TolTh   TolH       (maximum number of iterations and tolerances)");
    let _ = writeln!(o, "{:>4} {:>8} {:>6}", wf.max_iter, g(wf.tol_th), g(wf.tol_h));
    let _ = writeln!(o, "TopInf WLayer KodTop InitCond");
    let _ = writeln!(o, " {}     {}     {:>3}       {}", lg(bc.top_time_variable), lg(bc.surface_layer), bc.kod_top, lg(wf.init_in_water_content));
    let _ = writeln!(o, "BotInf qGWLF FreeD SeepF KodBot DrainF  hSeep");
    let _ = writeln!(
        o,
        " {}     {}     {}     {}     {:>2}      {}      {}",
        lg(bc.bot_time_variable),
        lg(bc.gwl_flux),
        lg(bc.free_drainage),
        lg(bc.seepage_face),
        bc.kod_bot,
        lg(bc.drains.is_some()),
        g(bc.h_seep)
    );
    if (!bc.top_time_variable && bc.kod_top == -1)
        || (!bc.bot_time_variable && bc.kod_bot == -1 && !bc.gwl_flux && !bc.free_drainage && !bc.seepage_face && bc.drains.is_none())
    {
        let _ = writeln!(o, "         rTop         rBot        rRoot");
        let _ = writeln!(o, "{}", row(&nums(&[bc.r_top, bc.r_bot, bc.r_root]), 12));
    }
    if bc.gwl_flux {
        let _ = writeln!(o, "        GWL0L         Aqh          Bqh");
        let _ = writeln!(o, "{}", row(&nums(&[bc.gwl0l, bc.aqh, bc.bqh]), 12));
    }
    if let Some(d) = &bc.drains {
        let _ = writeln!(o, "  iPosDr");
        let _ = writeln!(o, "{:>8}", d.position);
        let _ = writeln!(o, "      zBotDr    rSpacing      Entres");
        let _ = writeln!(o, "{}", row(&nums(&[d.z_bot.abs(), d.spacing, d.entrance_resistance]), 12));
        let (head, vals): (&str, Vec<f64>) = match d.position {
            1 => ("     KhTopDr", vec![d.kh_top]),
            2 => ("      BaseGw     KhTopDr      WetPer", vec![d.base_gw.abs(), d.kh_top, d.wet_perimeter]),
            3 => ("      BaseGw     KhTopDr     KhBotDr      WetPer", vec![d.base_gw.abs(), d.kh_top, d.kh_bot, d.wet_perimeter]),
            4 => (
                "      BaseGw     KvTopDr     KvBotDr     KhBotDr      WetPer    zIntfDr",
                vec![d.base_gw.abs(), d.kv_top, d.kv_bot, d.kh_bot, d.wet_perimeter, d.z_interface.abs()],
            ),
            _ => (
                "      BaseGw     KhTopDr     KvTopDr     KhBotDr      WetPer    zIntfDr    GeoFacDr",
                vec![d.base_gw.abs(), d.kh_top, d.kv_top, d.kh_bot, d.wet_perimeter, d.z_interface.abs(), d.geo_factor],
            ),
        };
        let _ = writeln!(o, "{head}");
        let _ = writeln!(o, "{}", row(&nums(&vals), 11));
    }
    if let Some(ir) = &bc.irrigation {
        let _ = writeln!(o, "Node  Pressure  Irrig_rate  Duration  Lag_period");
        let _ = writeln!(o, "{:>4} {:>9} {:>9} {:>9} {:>9}", ir.obs_node, g(ir.h_trigger), g(ir.rate), g(ir.duration), g(ir.lag));
    }
    let _ = writeln!(o, "    hTab1   hTabN");
    let _ = writeln!(o, "{:>10} {:>7}", g(wf.h_tab1), g(wf.h_tab_n));
    let _ = writeln!(o, "    Model   Hysteresis");
    let _ = writeln!(o, "{:>7} {:>10}", wf.model.code(), wf.hysteresis.code());
    if wf.hysteresis != Hysteresis::None {
        let _ = writeln!(o, "    Kappa");
        let _ = writeln!(o, "{:>7}", wf.init_kappa);
    }
    let _ = writeln!(o, "{}", water_header(prj));
    let npar = n_water_par(prj);
    for m in &wf.materials {
        let mut v = vec![m.qr, m.qs, m.alpha, m.n, m.ks, m.l];
        if wf.hysteresis != Hysteresis::None {
            v.extend([m.qm, m.qs_w, m.alpha_w, m.ks_w]);
        } else {
            v.extend(m.extra.iter().take(npar - 6));
        }
        let _ = writeln!(o, "{} ", row(&nums(&v), 7));
    }

    // ---------------- C: time
    let tm = &prj.time;
    let print_times: Vec<f64> = if tm.print_times.is_empty() { vec![tm.t_max] } else { tm.print_times.clone() };
    let _ = writeln!(o, "*** BLOCK C: TIME INFORMATION ******************************************");
    let _ = writeln!(o, "        dt       dtMin       dtMax     DMul    DMul2  ItMin ItMax  MPL");
    let _ = writeln!(
        o,
        "{:>11} {:>11} {:>11} {:>7} {:>8} {:>5} {:>5} {:>5}",
        g(tm.dt),
        g(tm.dt_min),
        g(tm.dt_max),
        g(tm.d_mul),
        g(tm.d_mul2),
        tm.it_min,
        tm.it_max,
        print_times.len()
    );
    let _ = writeln!(o, "      tInit        tMax");
    let _ = writeln!(o, "{:>11} {:>11}", g(tm.t_init), g(tm.t_max));
    let _ = writeln!(o, "  lPrintD  nPrintSteps tPrintInterval lEnter");
    let _ = writeln!(o, "     {}     {:>7}     {:>9}       t", lg(tm.print_at_interval), tm.print_step.max(1), g(tm.print_interval));
    let _ = writeln!(o, "TPrint(1),TPrint(2),...,TPrint(MPL)");
    for ch in print_times.chunks(6) {
        let _ = writeln!(o, "{} ", row(&nums(ch), 11));
    }

    // ---------------- D: root growth
    if p.root_growth {
        let _ = writeln!(o, "*** BLOCK D: ROOT GROWTH INFORMATION ***********************************");
        let _ = writeln!(o, "iRootDepthEntry");
        match &prj.root.growth {
            RootGrowthMode::FromAtmosphere => {
                let _ = writeln!(o, "        0");
            }
            RootGrowthMode::Table(tab) => {
                let _ = writeln!(o, "        1");
                let _ = writeln!(o, "  nGrowth");
                let _ = writeln!(o, "{:>9}", tab.len());
                let _ = writeln!(o, "       Time   RootDepth");
                for (t, x) in tab {
                    let _ = writeln!(o, "{}", row(&nums(&[*t, *x]), 11));
                }
            }
            RootGrowthMode::Logistic { t_min, t_med, t_harv, x_min, x_med, x_max, period, fit_from_half } => {
                let _ = writeln!(o, "        2");
                let _ = writeln!(o, "     iRFak     tRMin     tRMed    tRHarv     xRMin     xRMed     xRMax   tPeriod");
                let mut v = vec![if *fit_from_half { "1".to_string() } else { "0".to_string() }];
                v.extend(nums(&[*t_min, *t_med, *t_harv, *x_min, *x_med, *x_max, *period]));
                let _ = writeln!(o, "{}", row(&v, 9));
            }
        }
    }

    // ---------------- E: heat
    if p.heat {
        let h = &prj.heat;
        let _ = writeln!(o, "*** BLOCK E: HEAT TRANSPORT INFORMATION *********************************************************");
        let _ = writeln!(o, "    Qn      Qo    Disper.    B1          B2          B3          Cn          Co           Cw");
        for m in 0..n_mat {
            let hm = h.materials.get(m).cloned().unwrap_or_default();
            let _ = writeln!(o, "{} ", row(&nums(&[hm.qn, hm.qo, hm.disp, hm.b1, hm.b2, hm.b3, hm.cn, hm.co, hm.cw]), 11));
        }
        let _ = writeln!(o, "      tAmpl     tPeriod    Campbell   MeltConst  lDummy  lDummy  lDummy  lDummy  lDummy");
        let _ = writeln!(
            o,
            "{:>11} {:>11} {:>10} {:>10}       f       f       f       f       f",
            g(h.amplitude),
            g(h.period),
            u8::from(h.campbell),
            g(prj.atmosphere.snow_mf)
        );
        let _ = writeln!(o, "      kTopT       TTop      kBotT       TBot");
        let _ = writeln!(o, "{:>11} {:>10} {:>10} {:>10}", h.k_top, g(h.t_top), h.k_bot, g(h.t_bot));
    }

    // ---------------- F: solute
    if p.solute {
        let s = &prj.solute;
        let _ = writeln!(o, "*** BLOCK F: SOLUTE TRANSPORT INFORMATION *****************************************************");
        let _ = writeln!(o, " Epsi  lUpW  lArtD lTDep    cTolA    cTolR   MaxItC    PeCr  No.Solutes  lTort   iBacter   lFiltr  nChPar");
        let _ = writeln!(
            o,
            "{:>5}     {}     {}     {} {:>9} {:>9} {:>5} {:>8} {:>8}       {} {:>9}        {}       16",
            g(s.epsi),
            lg(s.upstream_weighting),
            lg(s.artificial_dispersion),
            lg(s.l_tdep),
            g(s.tol_abs),
            g(s.tol_rel),
            s.max_iter,
            g(s.pe_cr),
            ns,
            lg(s.tortuosity),
            u8::from(s.l_bact),
            lg(s.l_filtr)
        );
        let _ = writeln!(o, "iNonEqul lWatDep lDualNEq lInitM  lInitEq lTort lDummy  lDummy  lDummy  lDummy  lCFTr");
        let _ = writeln!(
            o,
            "   0       {}       {}       {}       {}       {}       {}       f       f       f       f",
            lg(s.l_moist),
            lg(s.l_dual_neq),
            lg(s.mass_init),
            lg(s.equil_init),
            lg(s.tort_model != 0),
            lg(s.l_nequil)
        );
        let _ = writeln!(o, "     Bulk.d.     DisperL.      Frac      Mobile WC (1..NMat)");
        for m in 0..n_mat {
            let sm = s.materials.get(m).cloned().unwrap_or_default();
            let _ = writeln!(o, "{} ", row(&nums(&[sm.bulk_density, sm.disp_l, sm.frac, sm.th_immobile]), 11));
        }
        for sp in s.species.iter().take(ns) {
            let _ = writeln!(o, "         DifW       DifG                n-th solute");
            let _ = writeln!(o, "{} ", row(&nums(&[sp.diff_w, sp.diff_g]), 11));
            let _ = writeln!(o, "         Ks          Nu        Beta       Henry       SnkL1       SnkS1       SnkG1       SnkL1'      SnkS1'      SnkG1'      SnkL0       SnkS0       SnkG0        Alfa");
            for m in 0..n_mat {
                let q = sp.per_material.get(m).cloned().unwrap_or_default();
                let v = if s.l_bact {
                    let (s6, s7) = if s.l_filtr { (q.d_c, q.d_p) } else { (q.i_psi2 as f64, q.i_psi1 as f64) };
                    vec![q.ks, q.nu, q.beta, q.henry, q.mu_w, q.mu_s, s6, s7, q.s_max2, q.r_ka2, q.r_kd2, q.s_max1, q.r_ka1, q.r_kd1]
                } else {
                    vec![q.ks, q.nu, q.beta, q.henry, q.mu_w, q.mu_s, q.mu_g, q.gam_w, q.gam_s, q.gam_g, q.mu0_w, q.mu0_s, q.mu0_g, q.omega]
                };
                let _ = writeln!(o, "{} ", row(&nums(&v), 11));
            }
        }
        if s.l_tdep {
            for j in 0..ns {
                let t = s.t_dep.get(j).cloned().unwrap_or_default();
                if j == 0 {
                    let _ = writeln!(o, "       Temperature dependence of the transport and reaction parameters (activation energies)");
                }
                let _ = writeln!(o, "         DifW       DifG                n-th solute");
                let _ = writeln!(o, "{} ", row(&nums(&[t.diff_w, t.diff_g]), 11));
                let _ = writeln!(o, "         Ks          Nu        Beta       Henry       SnkL1       SnkS1       SnkG1       SnkL1'      SnkS1'      SnkG1'      SnkL0       SnkS0       SnkG0        Alfa");
                let v = [t.ks, t.nu, t.beta, t.henry, t.mu_w, t.mu_s, t.mu_g, t.gam_w, t.gam_s, t.gam_g, t.mu0_w, t.mu0_s, t.mu0_g, t.omega];
                let _ = writeln!(o, "{} ", row(&nums(&v), 11));
            }
        }
        if s.l_moist {
            for j in 0..ns {
                let w = s.w_dep.get(j).cloned().unwrap_or_default();
                if j == 0 {
                    let _ = writeln!(o, "       Water content dependence of the reaction parameters");
                }
                let _ = writeln!(o, "      nPar");
                let _ = writeln!(o, "          9");
                let _ = writeln!(o, "       Exponent B (SnkL1 SnkS1 SnkG1 SnkL1' SnkS1' SnkG1' SnkL0 SnkS0 SnkG0)");
                let _ = writeln!(o, "{} ", row(&nums(&w.exp_b), 11));
                let _ = writeln!(o, "       Reference pressure head");
                let _ = writeln!(o, "{} ", row(&nums(&w.h_ref), 11));
            }
        }
        let _ = writeln!(o, "      kTopSolute  SolTop    kBotSolute  SolBot");
        let mut v = vec![s.k_top.to_string()];
        v.extend(s.species.iter().take(ns).map(|sp| g(sp.c_top)));
        v.push(s.k_bot.to_string());
        v.extend(s.species.iter().take(ns).map(|sp| g(sp.c_bot)));
        let _ = writeln!(o, "{} ", row(&v, 11));
        if s.k_top == -2 {
            let _ = writeln!(o, "       dSurf        cAtm");
            let _ = writeln!(o, "{} ", row(&nums(&[s.d_surf, s.c_atm]), 11));
        }
        let _ = writeln!(o, "      tPulse");
        let _ = writeln!(o, "{:>11}", g(s.t_pulse));
    }

    // ---------------- G: root water uptake
    if p.root_water_uptake {
        let r = &prj.root;
        let _ = writeln!(o, "*** BLOCK G: ROOT WATER UPTAKE INFORMATION *****************************");
        let _ = writeln!(o, "     Model  (0 - Feddes, 1 - S shape)  cRootMax    OmegaC");
        let i_mo = match r.stress {
            RootStress::Feddes { .. } => 0,
            RootStress::SShaped { .. } => 1,
        };
        let mut v = vec![i_mo.to_string()];
        v.extend((0..ns).map(|j| g(r.c_root_max.get(j).copied().unwrap_or(0.0))));
        v.push(g(r.omega_c));
        let _ = writeln!(o, "{} ", row(&v, 11));
        match &r.stress {
            RootStress::Feddes { p0, p2h, p2l, p3, r2h, r2l, p_optm } => {
                let _ = writeln!(o, "       P0       P2H       P2L       P3          r2H        r2L");
                let _ = writeln!(o, "{} ", row(&nums(&[*p0, *p2h, *p2l, *p3, *r2h, *r2l]), 9));
                let _ = writeln!(o, "POptm(1),POptm(2),...,POptm(NMat)");
                let po: Vec<f64> = (0..n_mat).map(|m| p_optm.get(m).or(p_optm.last()).copied().unwrap_or(-25.0)).collect();
                let _ = writeln!(o, "{} ", row(&nums(&po), 8));
            }
            RootStress::SShaped { p50, exponent } => {
                let _ = writeln!(o, "       P50        P3");
                let _ = writeln!(o, "{} ", row(&nums(&[*p50, *exponent]), 9));
            }
        }
        if p.solute {
            let _ = writeln!(o, "     Solute Reduction");
            let _ = writeln!(o, "        {}", lg(r.solute_stress.is_some()));
            if let Some(ss) = &r.solute_stress {
                let _ = writeln!(o, "     Additive");
                let _ = writeln!(o, "        {}", lg(ss.additive));
                let a: Vec<f64> = (0..ns).map(|j| ss.a_osm.get(j).copied().unwrap_or(0.0)).collect();
                if ss.additive {
                    let _ = writeln!(o, "     aOsm(1..NS)");
                    let _ = writeln!(o, "{} ", row(&nums(&a), 11));
                } else {
                    let _ = writeln!(o, "        c50         P3c     aOsm(1..NS)    Model (0 - threshold, 1 - S shape)");
                    let mut v = nums(&[ss.c50, ss.p3c]);
                    v.extend(nums(&a));
                    v.push(u8::from(ss.s_shaped).to_string());
                    let _ = writeln!(o, "{} ", row(&v, 11));
                }
            }
        }
        if r.l_act_rsu && ns >= 1 {
            // HYDRUS 4.17 layout with the solute taken up actively (iActSolU, 1-based)
            let _ = writeln!(o, "      OmegaS     SPot     KM       cMin    OmegaW  iActSolU");
            let v = vec![
                g(r.omega_act.first().copied().unwrap_or(0.0)),
                g(r.s_pot),
                g(r.r_km.first().copied().unwrap_or(0.0)),
                g(r.c_min.first().copied().unwrap_or(0.0)),
                lg(r.l_omega_w).to_string(),
                (r.active_solute.min(ns - 1) + 1).to_string(),
            ];
            let _ = writeln!(o, "{} ", row(&v, 9));
        }
    }
    let _ = writeln!(o, "*** END OF INPUT FILE 'SELECTOR.IN' ************************************");
    o
}

/// PROFILE.DAT.
pub fn profile_text(prj: &Project) -> String {
    let mut o = String::new();
    let nodes = &prj.profile.nodes;
    let x0 = nodes.first().map(|n| n.depth).unwrap_or(0.0);
    let depth = prj.profile.depth();
    let l_chem = prj.processes.solute;
    let l_temp = prj.processes.heat;
    let ns = prj.n_solutes();
    let l_equil = prj.processes.equilibrium_adsorption;
    let _ = writeln!(o, "Pcp_File_Version=4");
    let mut fixed: Vec<FixedPoint> = prj.profile.fixed_points.iter().copied().filter(|f| f.depth >= -1e-12 && f.depth <= depth * (1.0 + 1e-12)).collect();
    fixed.sort_by(|a, b| a.depth.total_cmp(&b.depth));
    if fixed.first().is_none_or(|f| f.depth > 1e-9 * depth.max(1e-30)) {
        fixed.insert(0, FixedPoint::new(0.0));
    }
    if fixed.last().is_none_or(|f| f.depth < depth * (1.0 - 1e-9)) {
        fixed.push(FixedPoint::new(depth));
    }
    let _ = writeln!(o, "{:>5}", fixed.len());
    for (k, f) in fixed.iter().enumerate() {
        let _ = writeln!(o, "{:>5}{}{}{}", k + 1, e3(-(f.depth) + 0.0), e3(f.size_above), e3(f.size_below));
    }
    let mut head = format!(
        "{:>5}{:>5}{:>5}{:>5} x         h      Mat  Lay      Beta           Axz            Bxz            Dxz          Temp          Conc ",
        nodes.len(),
        u8::from(l_temp || l_chem),
        ns,
        u8::from(l_equil)
    );
    if l_chem && !l_equil {
        head.push_str("           SConc");
    }
    let _ = writeln!(o, "{head}");
    for (i, n) in nodes.iter().enumerate() {
        let mut line = format!(
            "{:>5}{}{}{:>5}{:>5}{}{}{}{}",
            i + 1,
            e3(-(n.depth - x0) + 0.0),
            e3(n.h),
            n.mat,
            n.layer,
            e3(n.beta),
            e3(n.ah),
            e3(n.ak),
            e3(n.ath)
        );
        if l_temp || l_chem {
            line.push_str(&e3(n.temp));
        }
        if l_chem {
            for j in 0..ns {
                line.push_str(&e3(n.conc.get(j).copied().unwrap_or(0.0)));
            }
            if !l_equil {
                for j in 0..ns {
                    line.push_str(&e3(n.sorb.get(j).copied().unwrap_or(0.0)));
                }
            }
        }
        let _ = writeln!(o, "{line}");
    }
    let obs = &prj.profile.observation_nodes;
    let _ = writeln!(o, "{:>5}", obs.len());
    if !obs.is_empty() {
        let _ = writeln!(o, "{}", obs.iter().map(|k| format!("{k:>5}")).collect::<String>());
    }
    o
}

/// ATMOSPH.IN.
pub fn atmosph_text(prj: &Project) -> String {
    let mut o = String::new();
    let at = &prj.atmosphere;
    let l_chem = prj.processes.solute;
    let l_temp = prj.processes.heat;
    let ns = prj.n_solutes();
    let root_depth = atm_root_depth(prj);
    let _ = writeln!(o, "Pcp_File_Version=4");
    let _ = writeln!(o, "*** BLOCK I: ATMOSPHERIC INFORMATION  **********************************");
    let _ = writeln!(o, "   MaxAL                    (MaxAL = number of atmospheric data-records)");
    let _ = writeln!(o, "{:>7}", at.records.len());
    let _ = writeln!(o, " DailyVar  SinusVar  lLay  lBCCycles lInterc lDummy  lDummy  lDummy  lDummy  lDummy");
    let _ = writeln!(
        o,
        "       {}       {}       {}       {}       {}       f       f       f       f       f",
        lg(at.daily_variation),
        lg(at.sinusoidal_precip),
        lg(at.lai_partitioning),
        lg(at.bc_cycles),
        lg(at.interception)
    );
    if at.lai_partitioning {
        let _ = writeln!(o, " rExtinct");
        let _ = writeln!(o, "{:>9}", g(at.extinction));
    }
    if at.interception {
        let _ = writeln!(o, " aInterc");
        let _ = writeln!(o, "{:>9}", g(at.interception_a));
    }
    let _ = writeln!(o, " hCritS                 (max. allowed pressure head at the soil surface)");
    let _ = writeln!(o, " {}", g(at.h_crit_s));
    let mut head: Vec<String> = ["tAtm", "Prec", "rSoil", "rRoot", "hCritA", "rB", "hB", "ht"].iter().map(|s| s.to_string()).collect();
    if l_temp || l_chem {
        head.extend(["tTop", "tBot", "Ampl"].iter().map(|s| s.to_string()));
    }
    if l_chem {
        for j in 0..ns {
            if j == 0 {
                head.push("cTop".into());
                head.push("cBot".into());
            } else {
                head.push(format!("cTop{}", j + 1));
                head.push(format!("cBot{}", j + 1));
            }
        }
    }
    if root_depth {
        head.push("RootDepth".into());
    }
    let _ = writeln!(o, "{}", row(&head, 11));
    for r in &at.records {
        let mut v = vec![r.t, r.prec, r.evap, r.transp, r.h_crit_a, r.r_bot, r.h_bot, r.h_top];
        if l_temp || l_chem {
            v.extend([r.t_top, r.t_bot, r.ampl]);
        }
        if l_chem {
            for j in 0..ns {
                v.push(r.c_top.get(j).copied().unwrap_or(0.0));
                v.push(r.c_bot.get(j).copied().unwrap_or(0.0));
            }
        }
        if root_depth {
            v.push(r.x_root);
        }
        let _ = writeln!(o, "{} ", row(&nums(&v), 11));
    }
    let _ = writeln!(o, "end*** END OF INPUT FILE 'ATMOSPH.IN' **********************************");
    o
}

/// METEO.IN.
pub fn meteo_text(prj: &Project, m: &MeteoSettings) -> String {
    let mut o = String::new();
    let xc = prj.units.x_conv() / 100.0; // cm -> length unit
    let _ = writeln!(o, "Pcp_File_Version=4");
    let _ = writeln!(o, "* METEOROLOGICAL PARAMETERS AND INFORMATION |||||||||||||||||||||||||||||||");
    let _ = writeln!(o, " MeteoRecords Radiation Penman-Hargreaves");
    let _ = writeln!(o, "{:>13} {:>8}       {}", m.records.len(), m.i_radiation, lg(m.hargreaves));
    let _ = writeln!(o, "  lEnBal  lDaily  lDummy  lDummy  lDummy  lDummy  lDummy  lDummy  lDummy  lDummy");
    let _ = writeln!(o, "       {}       {}       f       f       f       f       f       f       f       f", lg(m.l_en_bal), lg(m.l_daily));
    if m.i_radiation != 2 {
        let _ = writeln!(o, " Latitude  Altitude");
        let _ = writeln!(o, "{:>9} {:>10}", g(m.latitude), g(m.altitude));
        let _ = writeln!(o, " ShortWaveRadA  ShortWaveRadB");
        let _ = writeln!(o, "{:>14} {:>14}", g(m.short_wave_a), g(m.short_wave_b));
        let _ = writeln!(o, " LongWaveRadA   LongWaveRadB");
        let _ = writeln!(o, "{:>14} {:>14}", g(m.long_wave_a), g(m.long_wave_b));
        let _ = writeln!(o, " LongWaveRadA1  LongWaveRadB1");
        let _ = writeln!(o, "{:>14} {:>14}", g(m.long_wave_a1), g(m.long_wave_b1));
    }
    let _ = writeln!(o, " WindHeight     TempHeight");
    let _ = writeln!(o, "{:>11} {:>14}", g(m.wind_height), g(m.temp_height));
    let _ = writeln!(o, " iCrop (=0: no crop, =1: constant, =2: table, =3: daily)  SunShine  RelativeHum");
    let _ = writeln!(o, "{:>10} {:>55} {:>9}", m.i_crop, m.i_sun_sh, m.i_rel_hum);
    if m.i_radiation == 1 && m.i_sun_sh == 3 {
        let _ = writeln!(o, " CloudFactorAc  CloudFactorBc");
        let _ = writeln!(o, "{:>14} {:>14}", g(m.cloud_fact_ac), g(m.cloud_fact_bc));
    }
    if m.i_crop >= 1 {
        let _ = writeln!(o, " iLAI    rExtinct");
        let _ = writeln!(o, "{:>5} {:>11}", m.i_lai, g(m.r_extinct));
        let _ = writeln!(o, " iInterception");
        let _ = writeln!(o, "{:>5}", m.i_interc);
        if m.i_crop == 1 {
            let _ = writeln!(o, " CropHeight     Albedo   LAI(SCF)      rRoot");
            let _ = writeln!(o, "{} ", row(&nums(&[m.crop_height * xc, m.albedo, m.lai, m.x_root]), 10));
        } else if m.i_crop == 2 {
            let _ = writeln!(o, " nGrowth");
            let _ = writeln!(o, "{:>5}", m.growth.len());
            let _ = writeln!(o, "          t CropHeight     Albedo   LAI(SCF)      rRoot");
            for gr in &m.growth {
                let _ = writeln!(o, "{} ", row(&nums(&[gr[0], gr[1] * xc, gr[2], gr[3], gr[4]]), 10));
            }
        }
        if m.i_interc == 1 {
            let _ = writeln!(o, " aInterc");
            let _ = writeln!(o, "{:>9}", g(m.a_interc));
        }
    } else {
        let _ = writeln!(o, "    Albedo");
        let _ = writeln!(o, "{:>10}", g(m.albedo));
    }
    let _ = writeln!(o, "Daily values");
    let _ = writeln!(o, "       t        Rad        TMax        TMin     RHMean      Wind    SunHours CropHeight     Albedo   LAI(SCF)      rRoot");
    let _ = writeln!(o, "      [T]  [MJ/m2/d]       [C]         [C]       [%]     [km/d]     [hour]      [L]           [-]        [-]        [L]");
    for r in &m.records {
        let mut v = vec![r.t, r.rad, r.t_max, r.t_min, r.rh_mean, r.wind_kmd, r.sun_hours];
        if m.i_crop == 3 {
            v.extend([r.crop_height.unwrap_or(0.0) * xc, r.albedo.unwrap_or(m.albedo), r.lai.unwrap_or(0.0), r.x_root.unwrap_or(0.0)]);
        }
        let _ = writeln!(o, "{} ", row(&nums(&v), 10));
    }
    let _ = writeln!(o, "end *** END OF INPUT FILE 'METEO.IN' **********************************");
    o
}

/// MATER.IN (tabulated hydraulic properties, model 10).
pub fn mater_text(tabs: &[rusteau_core::material::MatTable]) -> String {
    let mut o = String::new();
    for (m, t) in tabs.iter().enumerate() {
        let _ = writeln!(o, "Material {}:  h  theta  K  C", m + 1);
        let _ = writeln!(o, "{:>6}", t.h.len());
        for k in 0..t.h.len() {
            let _ = writeln!(o, "{} ", row(&nums(&[t.h[k], t.the[k], t.con[k], t.cap[k]]), 13));
        }
    }
    o
}

/// HYDRUS1D.DAT (the project summary of the graphical interface).
pub fn hydrus1d_dat_text(prj: &Project) -> String {
    let p = &prj.processes;
    let mut o = String::new();
    let n_lay = prj.profile.nodes.iter().map(|n| n.layer).max().unwrap_or(1).max(1);
    let n_print = prj.time.print_times.len().max(1);
    let _ = writeln!(o, "[Main]");
    let _ = writeln!(o, "HYDRUS_Version=4");
    let _ = writeln!(o, "WaterFlow={}", u8::from(p.water_flow));
    let _ = writeln!(o, "SoluteTransport={}", u8::from(p.solute));
    let _ = writeln!(o, "Unsatchem=0");
    let _ = writeln!(o, "HP1=0");
    let _ = writeln!(o, "HeatTransport={}", u8::from(p.heat));
    let _ = writeln!(o, "EquilibriumAdsorption={}", u8::from(p.equilibrium_adsorption));
    let _ = writeln!(o, "MobileImmobile=0");
    let _ = writeln!(o, "RootWaterUptake={}", u8::from(p.root_water_uptake));
    let _ = writeln!(o, "RootGrowth={}", u8::from(p.root_growth));
    let _ = writeln!(o, "MaterialNumbers={}", prj.water.materials.len());
    let _ = writeln!(o, "SubregionNumbers={n_lay}");
    let _ = writeln!(o, "SpaceUnit={}", length_unit(prj.units.length));
    let _ = writeln!(o, "TimeUnit={}", time_unit(prj.units.time));
    let _ = writeln!(o, "PrintTimes={n_print}");
    let _ = writeln!(o, "NumberOfSolutes={}", prj.n_solutes());
    let _ = writeln!(o, "InitialCondition={}", u8::from(prj.water.init_in_water_content));
    let _ = writeln!(o, ";");
    let _ = writeln!(o, "[Profile]");
    let _ = writeln!(o, "NumberOfNodes={}", prj.profile.nodes.len());
    let _ = writeln!(o, "ProfileDepth={}", e_ini(prj.profile.depth()));
    let _ = writeln!(o, "ObservationNodes={}", prj.profile.observation_nodes.len());
    let _ = writeln!(o, "GridVisible=1");
    let _ = writeln!(o, "SnapToGrid=1");
    let _ = writeln!(o, "ProfileWidth=80");
    let _ = writeln!(o, "LeftMargin=40");
    let _ = writeln!(o, "GridOrgX=0");
    let _ = writeln!(o, "GridOrgY=0");
    let _ = writeln!(o, "GridDX=5.E+00");
    let _ = writeln!(o, "GridDY=5.E+00");
    let _ = writeln!(o, ";");
    o
}

/// Writes the input files of a HYDRUS-1D project into `dir` (created if needed). Returns the files written and the
/// settings that the files cannot hold.
pub fn write_legacy_project(prj: &Project, dir: &Path) -> R<(Vec<PathBuf>, Vec<String>)> {
    std::fs::create_dir_all(dir).map_err(|e| HydrusError::Io(format!("{}: {}", dir.display(), e)))?;
    let mut files = vec![];
    // remove input files of another spelling (e.g. ATMOSPH.IN next to Atmosph.in) that the new ones replace
    const NAMES: [&str; 10] =
        ["SELECTOR.IN", "PROFILE.DAT", "ATMOSPH.IN", "METEO.IN", "FIT.IN", "MATER.IN", "MOISTDEP.IN", "HYSTERESIS.IN", "HYDRUS1D.DAT", "DESCRIPT.TXT"];
    if let Ok(rd) = std::fs::read_dir(dir) {
        for e in rd.flatten() {
            let n = e.file_name().to_string_lossy().to_ascii_uppercase();
            if NAMES.contains(&n.as_str()) {
                let _ = std::fs::remove_file(e.path());
            }
        }
    }
    write_file(dir, "SELECTOR.IN", &selector_text(prj), &mut files)?;
    write_file(dir, "PROFILE.DAT", &profile_text(prj), &mut files)?;
    if needs_atmosph(prj) {
        write_file(dir, "ATMOSPH.IN", &atmosph_text(prj), &mut files)?;
    }
    if let Some(m) = &prj.atmosphere.meteo {
        write_file(dir, "METEO.IN", &meteo_text(prj, m), &mut files)?;
    }
    if prj.water.model == SoilModel::Tabular {
        if let Some(t) = &prj.water.tabs {
            write_file(dir, "MATER.IN", &mater_text(t), &mut files)?;
        }
    }
    if prj.processes.solute && prj.solute.i_moist_dep == 2 && !prj.solute.moist_tables.is_empty() {
        let p = dir.join("MoistDep.in");
        crate::moist_dep::write_moist_dep(&p, prj)?;
        files.push(p);
    }
    if let Some(text) = hysteresis_text(prj) {
        write_file(dir, "Hysteresis.in", &text, &mut files)?;
    }
    if let Some(s) = &prj.inverse {
        let p = dir.join("FIT.IN");
        let mut s = s.clone();
        s.n_mat = s.n_mat.max(prj.water.materials.len());
        write_fit_in(&p, &s)?;
        let text = std::fs::read_to_string(&p).unwrap_or_default();
        let _ = std::fs::write(&p, crlf(&text));
        files.push(p);
    }
    write_file(dir, "HYDRUS1D.DAT", &hydrus1d_dat_text(prj), &mut files)?;
    let title = if prj.title.trim().is_empty() { "Rusteau project" } else { prj.title.trim() };
    write_file(dir, "DESCRIPT.TXT", &format!("Pcp_File_Version=1\n{title}\n"), &mut files)?;
    Ok((files, legacy_save_warnings(prj)))
}

/// Saves a HYDRUS-1D 4.x project: `<name>.h1d` and the folder `<name>/` next to it with the input files.
/// `h1d` may name the `.h1d` file or the project folder. Returns the files written and the warnings.
pub fn save_h1d_project(prj: &Project, h1d: &Path) -> R<(Vec<PathBuf>, Vec<String>)> {
    let (file, dir) = h1d_paths(h1d);
    let (mut files, warnings) = write_legacy_project(prj, &dir)?;
    std::fs::write(&file, H1D_TEMPLATE).map_err(|e| HydrusError::Io(format!("{}: {}", file.display(), e)))?;
    files.insert(0, file);
    Ok((files, warnings))
}

/// The `.h1d` file and the project folder of a HYDRUS-1D project path.
pub fn h1d_paths(path: &Path) -> (PathBuf, PathBuf) {
    let is_h1d = path.extension().is_some_and(|e| e.eq_ignore_ascii_case("h1d"));
    let base = if is_h1d { path.with_extension("") } else { path.to_path_buf() };
    let name = base.file_name().map(|n| n.to_string_lossy().to_string()).filter(|n| !n.is_empty()).unwrap_or_else(|| "project".into());
    let parent = base.parent().map(Path::to_path_buf).unwrap_or_default();
    (parent.join(format!("{name}.h1d")), parent.join(name))
}
