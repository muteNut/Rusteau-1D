//! Surface energy balance with vapor flow: first steps of the HYDRUS-1D example EnBal2b (Saito et al., 2006, Riverside),
//! with observation nodes at 0, 0.5, 1 and 2 cm, against the HYDRUS 4.17 output (Obs_Node.out, T_Level.out).
//!
//! HYDRUS 4.17 gives the latent heat carried through the surface by the soil vapor flux back to the top node
//! (F(N) = F(N) - HeatFl + xLat*vV); with the 2009 source the surface is 0.034 C too cold after the first step and the
//! second time step is cut by the temperature criterion (dtMaxT).
use rusteau_core::*;
use rusteau_io::read_legacy_project;

const SELECTOR: &str = "Pcp_File_Version=4
*** BLOCK A: BASIC INFORMATION *****************************************
Heading
EnBal2b, first steps
LUnit  TUnit  MUnit  (indicated units are obligatory for all input data)
cm
days
mmol
lWat   lChem lTemp  lSink lRoot lShort lWDep lScreen lVariabBC lEquil lInverse
 t     f     t      f     f     f      t     t       t         t         f
lSnow  lHP1   lMeteo  lVapor  lDummy  lFluxes lDummy  lDummy  lDummy  lDummy
 f       f       t       t       f       f       f       f       f       f
NMat    NLay  CosAlpha
  1       1       1
*** BLOCK B: WATER FLOW INFORMATION ************************************
MaxIt   TolTh   TolH       (maximum number of iterations and tolerances)
  10    0.001      1
TopInf WLayer KodTop InitCond
 t     t      -1       t
BotInf qGWLF FreeD SeepF KodBot DrainF  hSeep
 f     f     t     f     -1      f      0
    hTab1   hTabN
    1e-006   10000
    Model   Hysteresis
      0          0
   thr     ths    Alfa      n         Ks       l
  0.011   0.445  0.0277    1.38       34.2     0.5
*** BLOCK C: TIME INFORMATION ******************************************
        dt       dtMin       dtMax     DMul    DMul2  ItMin ItMax  MPL
      0.001      1e-005        0.01     1.3     0.7     3     7     1
      tInit        tMax
        328       328.1
  lPrintD  nPrintSteps tPrintInterval lEnter
     f           1             1       t
TPrint(1),TPrint(2),...,TPrint(MPL)
      328.1
*** BLOCK E: HEAT TRANSPORT INFORMATION *********************************************************
    Qn      Qo    Disper.    B1          B2          B3          Cn          Co           Cw
  0.555       0       5 1.47054e+016 -1.5518e+017 3.16617e+017 1.43327e+014 1.8737e+014 3.12035e+014
      tAmpl     tPeriod    Campbell   MeltConst  lDummy  lDummy  lDummy  lDummy  lDummy
          0           0          0       0.43       f       f       f       f       f
      kTopT       TTop      kBotT       TBot
         -1         20           0         20
*** END OF INPUT FILE 'SELECTOR.IN' ************************************
";

const METEO: &str = "Pcp_File_Version=4
* METEOROLOGICAL PARAMETERS AND INFORMATION |||||||||||||||||||||||||||||||
 MeteoRecords Radiation Penman-Hargreaves
            3        1       f
  lEnBal  lDaily  lDummy  lDummy  lDummy  lDummy  lDummy  lDummy  lDummy  lDummy
       t       f       f       f       f       t       f       f       f       f
 Latitude  Altitude
    33.58        306
 ShortWaveRadA  ShortWaveRadB
          0.25            0.5
 LongWaveRadA   LongWaveRadB
           0.9            0.1
 LongWaveRadA1  LongWaveRadB1
          0.34         -0.139
 WindHeight     TempHeight
        200            150
 iCrop (=0: no crop, =1: constant, =2: table, =3: daily)  SunShine  RelativeHum
         0                                                2         0
    Albedo
      0.23
Daily values
       t        Rad        TMax        TMin     RHMean      Wind    SunHours CropHeight     Albedo   LAI(SCF)      rRoot
      [T]  [MJ/m2/d]       [C]         [C]       [%]     [km/d]     [hour]      [L]           [-]        [-]        [L]
   328.042          0       15.2       15.2         33     138.24      0.545
   328.083          0       14.9       14.9         32     103.68      0.545
   328.125          0         15         15         32     138.24      0.545
end *** END OF INPUT FILE 'METEO.IN' **********************************
";

const ATMOSPH: &str = "Pcp_File_Version=4
*** BLOCK I: ATMOSPHERIC INFORMATION  **********************************
   MaxAL                    (MaxAL = number of atmospheric data-records)
      2
 DailyVar  SinusVar  lDummy  lDummy  lDummy  lDummy  lDummy  lDummy  lDummy  lDummy
       f       f       f       f       f       t       f       f       f       f
 hCritS                 (max. allowed pressure head at the soil surface)
      0
       tAtm        Prec       rSoil       rRoot      hCritA          rB          hB          ht        tTop        tBot        Ampl    RootDepth
        329           0           0           0      100000           0           0           0           0           0           0
        330           0           0           0      100000           0           0           0           0           0           0
end*** END OF INPUT FILE 'ATMOSPH.IN' **********************************
";

fn profile() -> String {
    // 101 nodes, 0.5 cm apart, initial water content 0.13, 17 C; observation nodes 1, 2, 3, 5
    let mut s = String::from("Pcp_File_Version=4\n    2\n    1  0.000000e+000  1.000000e+000  1.000000e+000\n    2 -5.000000e+001  1.000000e+000  1.000000e+000\n");
    s.push_str("  101    1    0    1 x         h      Mat  Lay      Beta           Axz            Bxz            Dxz          Temp          Conc           SConc\n");
    for i in 0..101 {
        s.push_str(&format!(
            "{:5}{:15.6e}  1.300000e-001    1    1  0.000000e+000  1.000000e+000  1.000000e+000  1.000000e+000  1.700000e+001\n",
            i + 1,
            -0.5 * i as f64
        ));
    }
    s.push_str("    4\n    1    2    3    5\n");
    s
}

#[test]
fn energy_balance_with_vapor_follows_hydrus_4_17() {
    let dir = std::env::temp_dir().join(format!("rusteau_enbal_vapor_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("SELECTOR.IN"), SELECTOR).unwrap();
    std::fs::write(dir.join("METEO.IN"), METEO).unwrap();
    std::fs::write(dir.join("ATMOSPH.IN"), ATMOSPH).unwrap();
    std::fs::write(dir.join("PROFILE.DAT"), profile()).unwrap();
    let prj = read_legacy_project(&dir).expect("project");
    let _ = std::fs::remove_dir_all(&dir);
    assert!(prj.processes.vapor && prj.atmosphere.meteo.as_ref().is_some_and(|m| m.l_en_bal));
    let mut sim = Simulation::new(prj).expect("simulation");

    // HYDRUS 4.17 (EnBal2b with observation nodes 1, 2, 3, 5): time, dt, vTop, temperatures at 0 / 0.5 / 1 / 2 cm
    let reference = [
        (328.0010, 0.0010, 0.41288, [15.485, 16.260, 16.638, 16.913]),
        (328.0023, 0.0013, 0.35178, [15.423, 15.654, 16.038, 16.605]),
        (328.0040, 0.00169, 0.30442, [14.753, 15.345, 15.686, 16.224]),
    ];
    for (t, dt, v_top, temps) in reference {
        sim.step();
        let tl = sim.res.tlevel.last().unwrap();
        assert!((tl.t - t).abs() < 1e-4, "t = {} (reference {t})", tl.t);
        assert!((tl.dt - dt).abs() < 1e-7, "dt = {} at t = {t} (reference {dt})", tl.dt);
        assert!((tl.v_top - v_top).abs() < 1e-4, "vTop = {} at t = {t} (reference {v_top})", tl.v_top);
        let ob = sim.res.obs.last().unwrap();
        for (k, tr) in temps.iter().enumerate() {
            let tc = ob.points[k].temp;
            assert!((tc - tr).abs() < 0.002, "Temp at observation node {} = {tc} at t = {t} (reference {tr})", k + 1);
        }
    }
}
