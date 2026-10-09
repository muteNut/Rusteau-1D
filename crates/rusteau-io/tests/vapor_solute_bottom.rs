//! Solute transport with vapor flow and a no-flux bottom (MatSet in SOLUTE.FOR):
//! `if (vN(1).gt.0 .or. (lVapor .and. rBot.eq.0.))` selects the third-type condition (prescribed solute flux `vN(1)*cBot`)
//! at the bottom for either direction of the liquid flux. With vapor flow and `rBot = 0` the liquid flux at node 1 is not zero
//! (the vapor flux compensates it), but no solute may leave through the bottom.
//!
//! The column is the thermal-gradient problem of the HYDRUS-1D example TEST11 (10 cm, 10 C at the top, 25 C at the bottom): the
//! liquid returns downwards, the solute piles up at the bottom (the reference prints 1.50 at node 51 after 0.25 d, the
//! zero-gradient outflow condition gave 0.98).
use rusteau_core::*;
use rusteau_io::read_legacy_project;

const SELECTOR: &str = "Pcp_File_Version=4
*** BLOCK A: BASIC INFORMATION *****************************************
Heading
Vapor flow and solute transport, no-flux bottom
LUnit  TUnit  MUnit  (indicated units are obligatory for all input data)
cm
days
mmol
lWat   lChem lTemp  lSink lRoot lShort lWDep lScreen lVariabBC lEquil lInverse
 t     t     t      f     f     f      t     t       f         t         f
lSnow  lHP1   lMeteo  lVapor  lDummy  lFluxes lDummy  lDummy  lDummy  lDummy
 f       f       f       t       f       f       f       f       f       f
NMat    NLay  CosAlpha
  1       1       0
*** BLOCK B: WATER FLOW INFORMATION ************************************
MaxIt   TolTh   TolH       (maximum number of iterations and tolerances)
  10   5e-006    0.1
TopInf WLayer KodTop InitCond
 f     f      -1       t
BotInf qGWLF FreeD SeepF KodBot DrainF  hSeep
 f     f     f     f     -1      f      0
         rTop         rBot        rRoot
           0            0            0
    hTab1   hTabN
    1e-006   10000
    Model   Hysteresis
      0          0
   thr     ths    Alfa      n         Ks       l
   0.03   0.499   0.036    1.56       33.7     0.5
*** BLOCK C: TIME INFORMATION ******************************************
        dt       dtMin       dtMax     DMul    DMul2  ItMin ItMax  MPL
       0.01      0.0001           5     1.3     0.7     3     7     2
      tInit        tMax
          0           1
  lPrintD  nPrintSteps tPrintInterval lEnter
     f           1          1000       t
TPrint(1),TPrint(2),...,TPrint(MPL)
       0.25           1
*** BLOCK E: HEAT TRANSPORT INFORMATION *********************************************************
    Qn      Qo    Disper.    B1          B2          B3          Cn          Co           Cw
   0.57       0       0 1.56728e+016 2.53474e+016 9.89388e+016 1.43327e+014 1.8737e+014 3.12035e+014
      tAmpl     tPeriod    Campbell   MeltConst  lDummy  lDummy  lDummy  lDummy  lDummy
          0           1          0          0       f       f       f       f       f
      kTopT       TTop      kBotT       TBot
          1         10           1         25
*** BLOCK F: SOLUTE TRANSPORT INFORMATION *****************************************************
 Epsi  lUpW  lArtD lTDep    cTolA    cTolR   MaxItC    PeCr  No.Solutes  lTort   iBacter   lFiltr  nChPar
  0.5     f     f     f         0         0     1        2        1       t       0        f       16
iNonEqul lWatDep lDualNEq lDummy  lDummy  lDummy  lDummy  lDummy  lDummy  lDummy  lDummy
   0       f       f       f       f       f       f       f       f       f       f
     Bulk.d.     DisperL.      Frac      Mobile WC (1..NMat)
        1.5           1           1           0
         DifW       DifG                n-th solute
          0           0
         Ks          Nu        Beta       Henry       SnkL1       SnkS1       SnkG1       SnkL1'      SnkS1'      SnkG1'      SnkL0       SnkS0       SnkG0        Alfa
          0           0           1           0           0           0           0           0           0           0           0           0           0           0
      kTopSolute  SolTop    kBotSolute  SolBot
         -1           0          -1           0
      tPulse
          5
*** END OF INPUT FILE 'SELECTOR.IN' ************************************
";

fn profile() -> String {
    // 51 nodes, 0.2 cm apart, initial water content 0.134, 20 C, concentration 1
    let mut s = String::from("Pcp_File_Version=4\n    2\n    1  0.000000e+000  1.000000e+000  1.000000e+000\n    2 -1.000000e+001  1.000000e+000  1.000000e+000\n");
    s.push_str("   51    1    1    1 x         h      Mat  Lay      Beta           Axz            Bxz            Dxz          Temp          Conc \n");
    for i in 0..51 {
        s.push_str(&format!(
            "{:5}{:16.6e}  1.340000e-001    1    1  0.000000e+000  1.000000e+000  1.000000e+000  1.000000e+000  2.000000e+001  1.000000e+000\n",
            i + 1,
            -0.2 * i as f64
        ));
    }
    s.push_str("    0\n");
    s
}

#[test]
fn no_flux_bottom_with_vapor_flow_does_not_let_solute_leave_through_the_bottom() {
    let dir = std::env::temp_dir().join(format!("rusteau_vapor_sol_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("SELECTOR.IN"), SELECTOR).unwrap();
    std::fs::write(dir.join("PROFILE.DAT"), profile()).unwrap();
    let prj = read_legacy_project(&dir).expect("project");
    assert!(prj.processes.vapor, "vapor flow expected");
    let mut sim = Simulation::new(prj).expect("simulation");
    assert_eq!(sim.run(|_| true), StepStatus::Finished, "{:?}", sim.res.messages);
    let _ = std::fs::remove_dir_all(&dir);

    // liquid flux at the bottom node is downward (balanced by the upward vapor flux), nothing leaves with it
    assert!(sim.v_new[0] < 0.0, "downward liquid flux at the bottom expected, v(1) = {}", sim.v_new[0]);
    let c_bottom = sim.conc(0, 0);
    assert!(c_bottom > 1.3, "solute must accumulate at the bottom node, c = {c_bottom} (reference 2.5 at t = 1)");
    // the accumulated amount comes from the profile above, the total mass is conserved (no loss through the bottom)
    let last = sim.res.balance.last().expect("balance");
    let err = last.sol_bal_r.first().copied().unwrap_or(0.0);
    assert!(err < 3.0, "solute balance error {err} %");
}

fn integral(x: &[f64], v: &[f64]) -> f64 {
    (0..x.len() - 1).map(|i| (x[i + 1] - x[i]) * (v[i] + v[i + 1]) / 2.0).sum()
}

#[test]
fn vapor_volume_in_the_balance_and_time_level_output() {
    // HYDRUS.FOR calls SubReg at PLevel = 0 before ThVNew is set: the initial W-volume holds no vapor while the old volume
    // does, so WatBalT carries the initial vapor volume and the first In-flow is -vapor/dt (HYDRUS 4.17 EnBal2b: W-volume
    // 6.5337 and In-flow -0.22724 at t = 328).  Later Balance.out volumes hold the vapor, the T_Level volume (TLInf) never.
    let dir = std::env::temp_dir().join(format!("rusteau_vapor_vol_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("SELECTOR.IN"), SELECTOR).unwrap();
    std::fs::write(dir.join("PROFILE.DAT"), profile()).unwrap();
    let prj = read_legacy_project(&dir).expect("project");
    let _ = std::fs::remove_dir_all(&dir);
    let mut sim = Simulation::new(prj).expect("simulation");

    let liquid0 = integral(&sim.x, &sim.th_old);
    let vapor0 = integral(&sim.x, &sim.th_v_old);
    assert!(vapor0 > 0.0);
    let b0 = sim.res.balance.first().expect("initial balance");
    assert!((b0.total.volume - liquid0).abs() < 1e-12, "initial W-volume {} vs liquid {}", b0.total.volume, liquid0);
    assert!((b0.total.change + vapor0 / sim.dt).abs() < 1e-9 * (vapor0 / sim.dt), "initial In-flow {}", b0.total.change);

    sim.step();
    let tl = sim.res.tlevel.last().expect("time level");
    assert!((tl.volume - integral(&sim.x, &sim.th_new)).abs() < 1e-12, "T_Level volume holds no vapor");

    while sim.res.balance.len() < 2 && !sim.done {
        sim.step();
    }
    let b1 = &sim.res.balance[1];
    let expected = integral(&sim.x, &sim.th_new) + integral(&sim.x, &sim.th_v_new);
    assert!((b1.total.volume - expected).abs() < 1e-12, "W-volume at the first print time holds the vapor");
    assert!((b1.wat_bal_t.abs() - vapor0).abs() < 0.2 * vapor0 + 1e-6, "WatBalT {} carries the initial vapor volume {}", b1.wat_bal_t, vapor0);
}
