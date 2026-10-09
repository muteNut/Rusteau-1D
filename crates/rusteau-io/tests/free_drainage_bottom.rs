//! Free drainage (unit gradient) at the bottom.
//!
//! WATFLOW.FOR `Reset` sets `rBot = -K cos(alpha)` and keeps the storage term of the bottom half-cell (F2 and
//! (ThNew-ThOld)*dx/dt) for every bottom condition.  An earlier version of the port dropped that storage once a wetting
//! front had reached a free-drainage bottom (h(1) = h(2) at once) to follow the 1SCALING / 2NOHYSTR sample outputs; those
//! files were written by an older HYDRUS build, and the compiled 2009 source keeps the storage (verified with gfortran:
//! 2NOHYSTR t = 13..24 and 1SCALING t >= 720 agree with the port to ~1e-4 only with the storage term).
use rusteau_core::*;
use rusteau_io::read_legacy_project;

fn selector(r_top: f64, t_max: f64) -> String {
    format!(
        "Pcp_File_Version=4
*** BLOCK A: BASIC INFORMATION *****************************************
Heading
Free drainage, constant infiltration
LUnit  TUnit  MUnit  (indicated units are obligatory for all input data)
cm
hours
-
lWat   lChem lTemp  lSink lRoot lShort lWDep lScreen lVariabBC lEquil lInverse
 t     f     f      f     f     f      f     t       f         t         f
lSnow  lHP1   lMeteo  lVapor  lDummy  lFluxes lDummy  lDummy  lDummy  lDummy
 f       f       f       f       f       f       f       f       f       f
NMat    NLay  CosAlpha
  1       1       1
*** BLOCK B: WATER FLOW INFORMATION ************************************
MaxIt   TolTh   TolH       (maximum number of iterations and tolerances)
  20    0.001    0.1
TopInf WLayer KodTop InitCond
 f     f      -1       f
BotInf qGWLF FreeD SeepF KodBot DrainF  hSeep
 f     f     t     f     -1      f      0
         rTop         rBot        rRoot
{r_top:>13}           0            0
    hTab1   hTabN
    1e-006   10000
    Model   Hysteresis
      0          0
   thr     ths    Alfa      n         Ks       l
   0.05    0.45    0.02       2          5     0.5
*** BLOCK C: TIME INFORMATION ******************************************
        dt       dtMin       dtMax     DMul    DMul2  ItMin ItMax  MPL
      0.001      0.0001         0.5     1.3     0.7     3     7     1
      tInit        tMax
          0{t_max:>11}
  lPrintD  nPrintSteps tPrintInterval lEnter
     f           1          1000       t
TPrint(1),TPrint(2),...,TPrint(MPL)
{t_max:>11}
*** END OF INPUT FILE 'SELECTOR.IN' ************************************
"
    )
}

/// 61 nodes, 1 cm apart; `h(depth)` gives the initial head.
fn profile(h: impl Fn(f64) -> f64) -> String {
    let mut s = String::from("Pcp_File_Version=4\n    2\n    1  0.000000e+000  1.000000e+000  1.000000e+000\n    2 -6.000000e+001  1.000000e+000  1.000000e+000\n");
    s.push_str("   61    0    0    1 x         h         Mat  Lay      Beta           Axz            Bxz            Dxz            Temp          Conc           SConc\n");
    for i in 0..61 {
        s.push_str(&format!(
            "{:5}{:16.6e}{:16.6e}    1    1  0.000000e+000  1.000000e+000  1.000000e+000  1.000000e+000              \n",
            i + 1,
            -(i as f64),
            h(i as f64)
        ));
    }
    s.push_str("    0\n");
    s
}

fn simulation(tag: &str, selector: String, profile: String) -> Simulation {
    let dir = std::env::temp_dir().join(format!("rusteau_free_drain_{tag}_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("SELECTOR.IN"), selector).unwrap();
    std::fs::write(dir.join("PROFILE.DAT"), profile).unwrap();
    let prj = read_legacy_project(&dir).expect("project");
    let _ = std::fs::remove_dir_all(&dir);
    let sim = Simulation::new(prj).expect("simulation");
    assert!(sim.free_d, "free drainage expected");
    sim
}

/// Uniformly dry start, constant infiltration: when the front reaches the bottom the storage of the bottom half-cell
/// lets the bottom node lag behind the node above it; in the final steady state the gradient is unity and the drainage
/// equals the infiltration rate.
#[test]
fn wetting_front_reaches_the_bottom_with_the_storage_term_of_the_source() {
    let mut sim = simulation("wet", selector(-1.0, 40.0), profile(|_| -300.0));
    let (mut arrived, mut max_lag) = (0usize, 0.0f64);
    let status = sim.run(|s| {
        if s.h_new[0] > -250.0 {
            arrived += 1;
            max_lag = max_lag.max(s.h_new[1] - s.h_new[0]);
        }
        true
    });
    assert_eq!(status, StepStatus::Finished, "{:?}", sim.res.messages);
    assert!(arrived > 10, "the front never reached the bottom ({arrived} time levels)");
    assert!(max_lag > 1e-3, "the bottom node follows the node above it without delay (max lag {max_lag} cm): storage term missing");

    // unit-gradient outflow in the steady state: rBot = -K at the bottom, equal to the infiltration rate
    let k_bot = (sim.con[0] + sim.con[1]) / 2.0;
    assert!((sim.r_bot + k_bot).abs() < 1e-9 * k_bot.max(1.0), "rBot = {}, K = {}", sim.r_bot, k_bot);
    assert!((sim.r_bot + 1.0).abs() < 0.02, "steady-state drainage must equal the infiltration rate, rBot = {}", sim.r_bot);
    assert!((sim.h_new[0] - sim.h_new[1]).abs() < 0.05, "steady state: h(1) = {}, h(2) = {}", sim.h_new[0], sim.h_new[1]);
}

/// Hydrostatic dry start without infiltration (the 2HYSTER situation): the bottom node still has its storage, i.e. it is
/// NOT pulled onto the head of the node above it (which is 1 cm lower) in the first time steps.
#[test]
fn hydrostatic_dry_start_keeps_the_bottom_head() {
    // h = -300 at the bottom (depth 60), decreasing by 1 cm per node towards the surface
    let mut sim = simulation("hydro", selector(0.0, 0.01), profile(|d| -300.0 - (60.0 - d)));
    let h0 = sim.h_new[0];
    assert!((h0 + 300.0).abs() < 1e-9, "bottom head {h0}");
    let mut worst = 0.0f64;
    let mut levels = 0usize;
    let status = sim.run(|s| {
        levels += 1;
        worst = worst.max((s.h_new[0] - h0).abs());
        true
    });
    assert_eq!(status, StepStatus::Finished, "{:?}", sim.res.messages);
    assert!(levels >= 3, "only {levels} time levels");
    assert!(worst < 0.05, "bottom head moved by up to {worst} cm in the hydrostatic dry start (the storage-free row gives 1 cm)");
}

/// Hydrostatic dry start with infiltration: the bottom head stays at its initial value in the first time steps, the
/// front arrives later and the profile ends in the unit-gradient steady state.
#[test]
fn front_arrival_after_hydrostatic_start() {
    let mut sim = simulation("hydro_wet", selector(-1.0, 40.0), profile(|d| -300.0 - (60.0 - d)));
    let h0 = sim.h_new[0];
    let (mut first_levels_ok, mut levels, mut arrived) = (true, 0usize, 0usize);
    let status = sim.run(|s| {
        levels += 1;
        if levels <= 3 && (s.h_new[0] - h0).abs() > 0.05 {
            first_levels_ok = false;
        }
        if s.h_new[0] > -250.0 {
            arrived += 1;
        }
        true
    });
    assert_eq!(status, StepStatus::Finished, "{:?}", sim.res.messages);
    assert!(first_levels_ok, "bottom head moved in the first time steps of the hydrostatic start");
    assert!(arrived > 10, "the front never reached the bottom");
    assert!((sim.r_bot + 1.0).abs() < 0.02, "steady-state drainage must equal the infiltration rate, rBot = {}", sim.r_bot);
}
