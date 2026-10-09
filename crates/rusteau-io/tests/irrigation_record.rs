//! HYDRUS 4.17 SELECTOR.IN with triggered irrigation: `lIrrig` is the 7th logical of the `lSnow lHP1 lMeteo lVapor
//! lActiveU lFluxes lIrrig` line and the record `Node Pressure Irrig_rate Duration Lag_period` follows the bottom
//! boundary records (5SEASON_IRR saved by HYDRUS 4.17).
use rusteau_core::*;
use rusteau_io::read_legacy_project;

const SELECTOR: &str = "Pcp_File_Version=4
*** BLOCK A: BASIC INFORMATION *****************************************
Heading
Triggered irrigation
LUnit  TUnit  MUnit  (indicated units are obligatory for all input data)
cm
days
-
lWat   lChem lTemp  lSink lRoot lShort lWDep lScreen lVariabBC lEquil lInverse
 t     f     f      f     f     f      f     t       t         t         f
lSnow  lHP1   lMeteo  lVapor lActiveU lFluxes lIrrig  lDummy  lDummy  lDummy
 f       f       f       f       f       f       t       f       f       f
NMat    NLay  CosAlpha
  1       1       1
*** BLOCK B: WATER FLOW INFORMATION ************************************
MaxIt   TolTh   TolH       (maximum number of iterations and tolerances)
  20    0.001    0.1
TopInf WLayer KodTop InitCond
 t     f      -1       f
BotInf qGWLF FreeD SeepF KodBot DrainF  hSeep
 f     t     f     f     -1      f      0
        GWL0L         Aqh          Bqh
         230      -0.1687     -0.02674
Node  Pressure  Irrig_rate  Duration  Lag_period
   1      -200         2         1         1
    hTab1   hTabN
    1e-006   10000
    Model   Hysteresis
      0          0
   thr     ths    Alfa      n         Ks       l
  0.078   0.43   0.036    1.56     24.96     0.5
*** BLOCK C: TIME INFORMATION ******************************************
        dt       dtMin       dtMax     DMul    DMul2  ItMin ItMax  MPL
       0.01      2e-005         0.5     1.3     0.3     3     7     1
      tInit        tMax
          0           4
  lPrintD  nPrintSteps tPrintInterval lEnter
     f           1             1       t
TPrint(1),TPrint(2),...,TPrint(MPL)
          4
*** END OF INPUT FILE 'SELECTOR.IN' ************************************
";

const ATMOSPH: &str = "Pcp_File_Version=4
*** BLOCK I: ATMOSPHERIC INFORMATION  **********************************
   MaxAL                    (MaxAL = number of atmospheric data-records)
      4
 DailyVar  SinusVar  lLay  lBCCycles lInterc lDummy  lDummy  lDummy  lDummy  lDummy
       f       f       f       f       f       f       f       f       f       f
 hCritS                 (max. allowed pressure head at the soil surface)
      0
       tAtm        Prec       rSoil       rRoot      hCritA          rB          hB          ht   RootDepth
          1           0         0.5           0       10000           0           0           0
          2           0         0.5           0       10000           0           0           0
          3           0         0.5           0       10000           0           0           0
          4           0         0.5           0       10000           0           0           0
end*** END OF INPUT FILE 'ATMOSPH.IN' **********************************
";

fn profile() -> String {
    let mut s = String::from("Pcp_File_Version=4\n    2\n    1  0.000000e+000  1.000000e+000  1.000000e+000\n    2 -1.000000e+002  1.000000e+000  1.000000e+000\n");
    s.push_str("  101    0    0    1 x         h      Mat  Lay      Beta           Axz            Bxz            Dxz\n");
    for i in 0..101 {
        s.push_str(&format!(
            "{:5}{:15.6e} -1.500000e+002    1    1  0.000000e+000  1.000000e+000  1.000000e+000  1.000000e+000\n",
            i + 1,
            -(i as f64)
        ));
    }
    s.push_str("    2\n    6   21\n");
    s
}

#[test]
fn irrigation_record_of_hydrus_4_17_is_read() {
    let dir = std::env::temp_dir().join(format!("rusteau_irrig_record_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("SELECTOR.IN"), SELECTOR).unwrap();
    std::fs::write(dir.join("ATMOSPH.IN"), ATMOSPH).unwrap();
    std::fs::write(dir.join("PROFILE.DAT"), profile()).unwrap();
    let prj = read_legacy_project(&dir);
    let _ = std::fs::remove_dir_all(&dir);
    let prj = prj.expect("project");
    assert_eq!(
        prj.water.bc.irrigation,
        Some(TriggeredIrrigation { obs_node: 1, h_trigger: -200.0, rate: 2.0, duration: 1.0, lag: 1.0, mode: IrrigationMode::Hydrus417 })
    );
    // the records after it are read from the right lines
    assert_eq!((prj.water.h_tab1, prj.water.h_tab_n), (1e-6, 10000.0));
    assert_eq!(prj.water.model, SoilModel::VanGenuchten);
    assert_eq!(prj.water.materials[0].ks, 24.96);
    assert_eq!((prj.water.bc.gwl0l, prj.water.bc.aqh), (230.0, -0.1687));
}
