//! Rosetta pedotransfer functions: van Genuchten parameters, saturated and unsaturated conductivity from the soil
//! texture (sand, silt and clay percentages), the bulk density and the water contents at 33 and 1500 kPa.
//!
//! Rosetta is an ensemble of neural networks trained on bootstrap samples of a soil database; the prediction is the
//! mean of the ensemble and its spread is the uncertainty. Three versions:
//!
//! * Rosetta 1 (Schaap, Leij & van Genuchten 2001, J. Hydrol. 251: 163–176, doi:10.1016/S0022-1694(01)00466-8), the
//!   models of the Rosetta Lite program in HYDRUS-1D;
//! * Rosetta 2 (Schaap, Nemes & van Genuchten 2004, Vadose Zone J. 3: 1455–1463), Rosetta 1 with the retention
//!   parameters corrected by linear functions;
//! * Rosetta 3 (Zhang & Schaap 2017, J. Hydrol. 547: 39–53, doi:10.1016/j.jhydrol.2017.01.004), retrained on a larger
//!   database (the default: smaller errors, in particular of θr, θs and Ks).
//!
//! The unsaturated conductivity parameters K0 and L (Schaap & Leij 2000) come from the van Genuchten parameters.
//! The network weights are those of the rosetta-soil package of the USDA-ARS U.S. Salinity Laboratory (CC0 1.0),
//! converted by `tools/rosetta_export.py`.

use std::sync::OnceLock;

const DATA: &[u8] = include_bytes!("../data/rosetta.bin");

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RosettaVersion {
    V1,
    V2,
    V3,
}

impl RosettaVersion {
    pub const ALL: [RosettaVersion; 3] = [RosettaVersion::V3, RosettaVersion::V2, RosettaVersion::V1];
    pub fn name(self) -> &'static str {
        match self {
            RosettaVersion::V1 => "Rosetta 1 (Schaap et al. 2001; Rosetta Lite of HYDRUS-1D)",
            RosettaVersion::V2 => "Rosetta 2 (Schaap et al. 2004)",
            RosettaVersion::V3 => "Rosetta 3 (Zhang & Schaap 2017)",
        }
    }
}

/// Estimate of α, n, Ks and K0 from the ensemble (θr, θs and L are always arithmetic means).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Estimate {
    /// arithmetic mean of the ensemble
    Arithmetic,
    /// geometric mean (10 to the mean of the logarithms)
    Geometric,
    /// mean of the log10 values (α, n, Ks and K0 are returned as log10)
    Log,
}

/// Input data: sand, silt and clay in % (sum 99–101), bulk density in g/cm³ (0.5–2), water contents at 33 kPa and
/// 1500 kPa in cm³/cm³ (0 < θ1500 < θ33 < 1).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SoilData {
    pub sand: f64,
    pub silt: f64,
    pub clay: f64,
    pub bulk_density: Option<f64>,
    pub th33: Option<f64>,
    pub th1500: Option<f64>,
}

impl SoilData {
    pub fn texture(sand: f64, silt: f64, clay: f64) -> Self {
        SoilData { sand, silt, clay, ..Default::default() }
    }

    fn separates_ok(&self) -> bool {
        let s = self.sand + self.silt + self.clay;
        (99.0..=101.0).contains(&s) && self.sand >= 0.0 && self.silt >= 0.0 && self.clay >= 0.0
    }

    /// The most detailed model the data allow: 2 (texture), 3 (+ bulk density), 4 (+ θ33), 5 (+ θ1500); None when the
    /// texture is invalid.
    pub fn best_model(&self) -> Option<u8> {
        if !self.separates_ok() {
            return None;
        }
        let mut code = 2;
        if self.bulk_density.is_some_and(|b| (0.5..=2.0).contains(&b)) {
            code = 3;
            if self.th33.is_some_and(|t| t > 0.0 && t < 1.0) {
                code = 4;
                if let (Some(t33), Some(t15)) = (self.th33, self.th1500) {
                    if t15 > 0.0 && t15 < t33 {
                        code = 5;
                    }
                }
            }
        }
        Some(code)
    }

    /// Why the data do not allow model `code` (None: they do).
    pub fn problem(&self, code: u8) -> Option<String> {
        if !self.separates_ok() {
            return Some(format!("sand + silt + clay must be 99–101 % (is {:.1} %)", self.sand + self.silt + self.clay));
        }
        if code >= 3 && !self.bulk_density.is_some_and(|b| (0.5..=2.0).contains(&b)) {
            return Some("the bulk density must be 0.5–2 g/cm³".into());
        }
        if code >= 4 && !self.th33.is_some_and(|t| t > 0.0 && t < 1.0) {
            return Some("the water content at 33 kPa must be between 0 and 1".into());
        }
        if code >= 5 && !matches!((self.th33, self.th1500), (Some(a), Some(b)) if b > 0.0 && b < a) {
            return Some("the water content at 1500 kPa must be positive and smaller than at 33 kPa".into());
        }
        None
    }

    fn inputs(&self, code: u8) -> Vec<f64> {
        let mut v = vec![self.sand, self.silt, self.clay];
        if code >= 3 {
            v.push(self.bulk_density.unwrap_or(0.0));
        }
        if code >= 4 {
            v.push(self.th33.unwrap_or(0.0));
        }
        if code >= 5 {
            v.push(self.th1500.unwrap_or(0.0));
        }
        v
    }
}

/// Name of the input data of model `code` (2–5).
pub fn model_name(code: u8) -> &'static str {
    match code {
        2 => "Sand, silt and clay (SSC)",
        3 => "SSC + bulk density (BD)",
        4 => "SSC + BD + θ at 33 kPa",
        _ => "SSC + BD + θ at 33 and 1500 kPa",
    }
}

/// Predicted parameters: θr, θs [cm³/cm³], α [1/cm], n [-], Ks [cm/day], K0 [cm/day], L [-] (α, n, Ks and K0 as
/// log10 for [`Estimate::Log`]) with the standard deviations of the ensemble.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Prediction {
    pub model: u8,
    pub theta_r: f64,
    pub theta_s: f64,
    pub alpha: f64,
    pub n: f64,
    pub ks: f64,
    pub k0: f64,
    pub l: f64,
    /// standard deviations in the order θr, θs, α, n, Ks, K0, L
    pub std: [f64; 7],
}

impl Prediction {
    pub fn values(&self) -> [f64; 7] {
        [self.theta_r, self.theta_s, self.alpha, self.n, self.ks, self.k0, self.l]
    }
}

// ------------------------------------------------------------------------------------------------- networks

#[derive(Clone, Copy)]
enum Act {
    Sigmoid,
    Tansig,
    Linear,
    Relu,
}

#[derive(Clone, Copy)]
struct Scaler {
    kind: u8,
    p: [f64; 3],
}

impl Scaler {
    fn apply(&self, x: f64) -> f64 {
        let p = &self.p;
        match self.kind {
            1 => x / p[0],
            2 => x.log10() / p[0] + p[1],
            3 => p[0] * (x + p[1]),
            4 => (x - p[0]) * p[1] + p[2],
            5 => x * p[0] + p[1],
            _ => x,
        }
    }
}

struct Net {
    name: String,
    ninput: usize,
    noutput: usize,
    nboot: usize,
    layers: Vec<(usize, Act)>,
    inputs: Vec<Scaler>,
    outputs: Vec<Scaler>,
    /// weights and biases of all members, member after member
    w: Vec<f32>,
    per_member: usize,
}

impl Net {
    /// Outputs of every ensemble member for one input vector.
    fn predict(&self, x: &[f64]) -> Vec<Vec<f64>> {
        debug_assert_eq!(x.len(), self.ninput);
        let a0: Vec<f64> = x.iter().zip(&self.inputs).map(|(&v, s)| s.apply(v)).collect();
        (0..self.nboot)
            .map(|b| {
                let mut k = b * self.per_member;
                let mut a = a0.clone();
                for &(n, act) in &self.layers {
                    let nin = a.len();
                    let mut z = vec![0.0; n];
                    for (r, zr) in z.iter_mut().enumerate() {
                        let row = &self.w[k + r * nin..k + (r + 1) * nin];
                        *zr = row.iter().zip(&a).map(|(&w, &x)| w as f64 * x).sum::<f64>();
                    }
                    k += n * nin;
                    for (r, zr) in z.iter_mut().enumerate() {
                        *zr += self.w[k + r] as f64;
                    }
                    k += n;
                    for zr in z.iter_mut() {
                        *zr = match act {
                            Act::Sigmoid => 1.0 / (1.0 + (-*zr).exp()),
                            Act::Tansig => -1.0 + 2.0 / (1.0 + (-2.0 * *zr).exp()),
                            Act::Linear => *zr,
                            Act::Relu => zr.max(0.0),
                        };
                    }
                    a = z;
                }
                a.iter().take(self.noutput).zip(&self.outputs).map(|(&v, s)| s.apply(v)).collect()
            })
            .collect()
    }
}

fn nets() -> &'static [Net] {
    static NETS: OnceLock<Vec<Net>> = OnceLock::new();
    NETS.get_or_init(|| parse(DATA).expect("rosetta.bin"))
}

fn parse(d: &[u8]) -> Option<Vec<Net>> {
    let mut pos = 0usize;
    let mut take = |n: usize| -> Option<&[u8]> {
        let s = d.get(pos..pos + n)?;
        pos += n;
        Some(s)
    };
    if take(4)? != b"RSTA" {
        return None;
    }
    let u32_ = |b: &[u8]| u32::from_le_bytes([b[0], b[1], b[2], b[3]]) as usize;
    let _version = u32_(take(4)?);
    let n_models = u32_(take(4)?);
    let mut out = vec![];
    for _ in 0..n_models {
        let ln = take(1)?[0] as usize;
        let name = String::from_utf8_lossy(take(ln)?).to_string();
        let (ninput, noutput, nboot, nlayer) = (u32_(take(4)?), u32_(take(4)?), u32_(take(4)?), u32_(take(4)?));
        let mut layers = vec![];
        for _ in 0..nlayer {
            let n = u32_(take(4)?);
            let act = match take(1)?[0] {
                0 => Act::Sigmoid,
                1 => Act::Tansig,
                2 => Act::Linear,
                _ => Act::Relu,
            };
            layers.push((n, act));
        }
        let mut scalers = |n: usize| -> Option<Vec<Scaler>> {
            let mut v = vec![];
            for _ in 0..n {
                let kind = take(1)?[0];
                let mut p = [0.0; 3];
                for pk in p.iter_mut() {
                    let b = take(8)?;
                    *pk = f64::from_le_bytes(b.try_into().ok()?);
                }
                v.push(Scaler { kind, p });
            }
            Some(v)
        };
        let inputs = scalers(ninput)?;
        let outputs = scalers(noutput)?;
        let mut per_member = 0;
        let mut nin = ninput;
        for &(n, _) in &layers {
            per_member += n * nin + n;
            nin = n;
        }
        let bytes = take(4 * per_member * nboot)?;
        let w: Vec<f32> = bytes.chunks_exact(4).map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]])).collect();
        out.push(Net { name, ninput, noutput, nboot, layers, inputs, outputs, w, per_member });
    }
    Some(out)
}

fn net(name: &str) -> &'static Net {
    nets().iter().find(|n| n.name == name).unwrap_or_else(|| panic!("Rosetta model {name} missing"))
}

/// Retention parameter corrections of Rosetta 2 (slope, offset for θr, θs, log10 α, log10 n).
fn rose2_coefficients(code: u8) -> [(f64, f64); 4] {
    match code {
        2 => [(0.969, 0.000), (1.103, -0.048), (1.050, 0.159), (0.655, 0.004)],
        3 => [(0.995, 0.000), (1.000, 0.002), (1.028, 0.162), (0.660, 0.003)],
        4 => [(1.156, 0.000), (0.974, 0.011), (1.039, 0.067), (0.796, 0.006)],
        _ => [(1.220, 0.000), (1.045, -0.020), (0.956, -0.040), (0.877, -0.003)],
    }
}

fn mean(v: &[f64]) -> f64 {
    v.iter().sum::<f64>() / v.len().max(1) as f64
}

/// Sample standard deviation (n - 1).
fn std(v: &[f64]) -> f64 {
    if v.len() < 2 {
        return 0.0;
    }
    let m = mean(v);
    (v.iter().map(|x| (x - m).powi(2)).sum::<f64>() / (v.len() - 1) as f64).sqrt()
}

/// Members of the unsaturated conductivity model (log10 K0, L) for the retention parameters θr, θs, α, n.
fn unsat_k(p: [f64; 4]) -> Vec<Vec<f64>> {
    let [thr, ths, alp, n] = p;
    if !(0.0 <= thr && thr < ths && ths < 1.0 && alp > 0.0 && n > 1.0) {
        return vec![vec![f64::NAN, f64::NAN]];
    }
    net("rose1_unsat_k").predict(&p)
}

/// Rosetta prediction with model `code` (2–5; None: the most detailed the data allow).
pub fn predict(version: RosettaVersion, data: &SoilData, code: Option<u8>, estimate: Estimate) -> Result<Prediction, String> {
    let best = data.best_model().ok_or_else(|| data.problem(2).unwrap_or_default())?;
    let code = code.unwrap_or(best).clamp(2, 5);
    if let Some(p) = data.problem(code) {
        return Err(p);
    }
    let x = data.inputs(code);
    // members: retention (θr, θs, log10 α, log10 n) and log10 Ks
    let (mut retc, ksat): (Vec<Vec<f64>>, Vec<f64>) = match version {
        RosettaVersion::V3 => {
            let m = net(&format!("rose3_mod{code}_0")).predict(&x);
            (m.iter().map(|r| r[..4].to_vec()).collect(), m.iter().map(|r| r[4]).collect())
        }
        _ => (
            net(&format!("rose1_mod{code}_0")).predict(&x),
            net(&format!("rose1_mod{code}_1")).predict(&x).iter().map(|r| r[0]).collect(),
        ),
    };
    if version == RosettaVersion::V2 {
        let c = rose2_coefficients(code);
        for r in retc.iter_mut() {
            for (k, (slope, off)) in c.iter().enumerate() {
                r[k] = slope * r[k] + off;
            }
        }
    }
    let col = |k: usize| -> Vec<f64> { retc.iter().map(|r| r[k]).collect() };
    let lin = |v: Vec<f64>| -> Vec<f64> { v.iter().map(|x| 10f64.powf(*x)).collect() };
    let (thr, ths, la, ln) = (col(0), col(1), col(2), col(3));
    let (a_lin, n_lin, k_lin) = (lin(la.clone()), lin(ln.clone()), lin(ksat.clone()));
    let std_lin = [std(&thr), std(&ths), std(&a_lin), std(&n_lin), std(&k_lin)];
    let geo = [mean(&thr), mean(&ths), 10f64.powf(mean(&la)), 10f64.powf(mean(&ln))];
    let mut out = Prediction { model: code, theta_r: mean(&thr), theta_s: mean(&ths), alpha: 0.0, n: 0.0, ks: 0.0, k0: 0.0, l: 0.0, std: [0.0; 7] };
    match estimate {
        Estimate::Arithmetic => {
            (out.alpha, out.n, out.ks) = (mean(&a_lin), mean(&n_lin), mean(&k_lin));
            out.std[..5].copy_from_slice(&std_lin);
            let u = unsat_k([out.theta_r, out.theta_s, out.alpha, out.n]);
            let k0: Vec<f64> = u.iter().map(|r| 10f64.powf(r[0])).collect();
            let l: Vec<f64> = u.iter().map(|r| r[1]).collect();
            (out.k0, out.l, out.std[5], out.std[6]) = (mean(&k0), mean(&l), std(&k0), std(&l));
        }
        Estimate::Geometric => {
            (out.alpha, out.n, out.ks) = (geo[2], geo[3], 10f64.powf(mean(&ksat)));
            out.std[..5].copy_from_slice(&std_lin);
            let u = unsat_k(geo);
            let lk0: Vec<f64> = u.iter().map(|r| r[0]).collect();
            let k0: Vec<f64> = lk0.iter().map(|x| 10f64.powf(*x)).collect();
            let l: Vec<f64> = u.iter().map(|r| r[1]).collect();
            (out.k0, out.l, out.std[5], out.std[6]) = (10f64.powf(mean(&lk0)), mean(&l), std(&k0), std(&l));
        }
        Estimate::Log => {
            (out.alpha, out.n, out.ks) = (mean(&la), mean(&ln), mean(&ksat));
            out.std[..5].copy_from_slice(&[std(&thr), std(&ths), std(&la), std(&ln), std(&ksat)]);
            let u = unsat_k(geo);
            let lk0: Vec<f64> = u.iter().map(|r| r[0]).collect();
            let l: Vec<f64> = u.iter().map(|r| r[1]).collect();
            (out.k0, out.l, out.std[5], out.std[6]) = (mean(&lk0), mean(&l), std(&lk0), std(&l));
        }
    }
    Ok(out)
}

/// USDA soil texture class of a sand / silt / clay composition (percent).
pub fn texture_class(sand: f64, silt: f64, clay: f64) -> &'static str {
    if silt + 1.5 * clay < 15.0 {
        "Sand"
    } else if silt + 2.0 * clay < 30.0 {
        "Loamy sand"
    } else if (7.0..20.0).contains(&clay) && sand > 52.0 || clay < 7.0 && silt < 50.0 {
        "Sandy loam"
    } else if (7.0..27.0).contains(&clay) && (28.0..50.0).contains(&silt) && sand <= 52.0 {
        "Loam"
    } else if silt >= 50.0 && (12.0..27.0).contains(&clay) || (50.0..80.0).contains(&silt) && clay < 12.0 {
        "Silt loam"
    } else if silt >= 80.0 && clay < 12.0 {
        "Silt"
    } else if (20.0..35.0).contains(&clay) && silt < 28.0 && sand > 45.0 {
        "Sandy clay loam"
    } else if (27.0..40.0).contains(&clay) && sand > 20.0 && sand <= 45.0 {
        "Clay loam"
    } else if (27.0..40.0).contains(&clay) && sand <= 20.0 {
        "Silty clay loam"
    } else if clay >= 35.0 && sand > 45.0 {
        "Sandy clay"
    } else if clay >= 40.0 && silt >= 40.0 {
        "Silty clay"
    } else {
        "Clay"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn data() -> Vec<SoilData> {
        vec![
            SoilData { sand: 30.0, silt: 30.0, clay: 40.0, bulk_density: Some(1.5), th33: Some(0.3), th1500: Some(0.1) },
            SoilData::texture(20.0, 60.0, 20.0),
            SoilData { sand: 55.0, silt: 25.0, clay: 20.0, bulk_density: Some(1.1), ..Default::default() },
            SoilData { sand: 70.0, silt: 20.0, clay: 10.0, bulk_density: Some(1.6), th33: Some(0.2), th1500: None },
            SoilData { sand: 10.0, silt: 40.0, clay: 50.0, bulk_density: Some(1.3), th33: Some(0.4), th1500: Some(0.25) },
        ]
    }

    fn check(version: RosettaVersion, est: Estimate, expect: &[[f64; 14]]) {
        for (d, e) in data().iter().zip(expect) {
            let p = predict(version, d, None, est).unwrap();
            let got: Vec<f64> = p.values().iter().chain(p.std.iter()).copied().collect();
            for (k, (g, x)) in got.iter().zip(e).enumerate() {
                let tol = 2e-5 * x.abs().max(1e-3);
                assert!((g - x).abs() <= tol, "{version:?} {est:?} {d:?} value {k}: {g} vs {x}");
            }
        }
    }

    /// Reference values of rosetta-soil 0.3.2 (`rosetta(version, data, estimate)`, weights in double precision).
    #[test]
    fn rosetta3_reproduces_rosetta_soil() {
        check(
            RosettaVersion::V3,
            Estimate::Arithmetic,
            &[
                [0.06872133198, 0.3839050853, 0.00369145407, 1.512296014, 10.45220673, 1.020283782, 0.2241580265, 0.01363580508, 0.01497666546, 0.001200996548, 0.1175669761, 4.81602764, 0.4967836368, 1.401698792],
                [0.08994502219, 0.430136648, 0.003803295121, 1.499297864, 15.89954955, 0.8908630388, 0.1726442593, 0.006710949118, 0.008790220586, 0.000666689101, 0.04539463667, 3.178862087, 0.4489771681, 1.611883051],
                [0.09130753033, 0.485031958, 0.009748718589, 1.417196484, 84.78337269, 2.901505955, -0.3463378623, 0.01277779758, 0.01306870656, 0.002214833208, 0.05767077407, 27.03573985, 1.820278795, 1.224819304],
                [0.06797608206, 0.3524546745, 0.01676953578, 1.412861538, 24.66792325, 5.500830773, -1.029312153, 0.007262164724, 0.006440380454, 0.002066570202, 0.03392411751, 2.931759282, 3.257971598, 0.9572137229],
                [0.1274449145, 0.4885261497, 0.008155552399, 1.239791035, 15.39584536, 1.148958427, -1.256587486, 0.01727873716, 0.01433937884, 0.001905876435, 0.04276815245, 5.45923244, 0.8423213169, 2.044718989],
            ],
        );
        check(
            RosettaVersion::V3,
            Estimate::Geometric,
            &[
                [0.06872133198, 0.3839050853, 0.003523961284, 1.507557708, 9.609985358, 0.8716046632, 0.2488566024, 0.01363580508, 0.01497666546, 0.001200996548, 0.1175669761, 4.81602764, 0.4706371405, 1.427998588],
                [0.08994502219, 0.430136648, 0.003747695099, 1.498605525, 15.58587292, 0.7776068052, 0.1813930927, 0.006710949118, 0.008790220586, 0.000666689101, 0.04539463667, 3.178862087, 0.4417081207, 1.622812872],
                [0.09130753033, 0.485031958, 0.009497557076, 1.416027254, 80.54058415, 2.360587642, -0.3253937879, 0.01277779758, 0.01306870656, 0.002214833208, 0.05767077407, 27.03573985, 1.742853532, 1.244061517],
                [0.06797608206, 0.3524546745, 0.01664337702, 1.412454334, 24.49661072, 4.640998322, -1.024349695, 0.007262164724, 0.006440380454, 0.002066570202, 0.03392411751, 2.931759282, 3.211451747, 0.957981603],
                [0.1274449145, 0.4885261497, 0.007941379189, 1.23905413, 14.5013757, 0.9181493565, -1.233734804, 0.01727873716, 0.01433937884, 0.001905876435, 0.04276815245, 5.45923244, 0.8171731658, 2.07787213],
            ],
        );
        check(
            RosettaVersion::V3,
            Estimate::Log,
            &[
                [0.06872133198, 0.3839050853, -2.452968872, 0.1782739455, 0.982722726, -0.05968045488, 0.2488566024, 0.01363580508, 0.01497666546, 0.1295518353, 0.03479132364, 0.1748672304, 0.2066236738, 1.427998588],
                [0.08994502219, 0.430136648, -2.426235749, 0.1756873293, 1.192731131, -0.1092399473, 0.1813930927, 0.006710949118, 0.008790220586, 0.07416848676, 0.01323730353, 0.0871380394, 0.2194767877, 1.622812872],
                [0.09130753033, 0.485031958, -2.022388088, 0.1510716122, 1.906014775, 0.3730201292, -0.3253937879, 0.01277779758, 0.01306870656, 0.1002532617, 0.01764865101, 0.1417065399, 0.2540276761, 1.244061517],
                [0.06797608206, 0.3524546745, -1.778758549, 0.1499744155, 1.389106001, 0.6666114114, -1.024349695, 0.007262164724, 0.006440380454, 0.05345792842, 0.01043641592, 0.05125769568, 0.2511930707, 0.957981603],
                [0.1274449145, 0.4885261497, -2.100104067, 0.09309027979, 1.161409204, -0.03708666581, -1.233734804, 0.01727873716, 0.01433937884, 0.1003108951, 0.014987375, 0.1507001236, 0.2608589398, 2.07787213],
            ],
        );
    }

    #[test]
    fn rosetta1_and_2_reproduce_rosetta_soil() {
        check(
            RosettaVersion::V1,
            Estimate::Arithmetic,
            &[
                [0.05047777852, 0.4026809712, 0.003622440454, 1.609192286, 14.82684882, 1.472794095, 0.5961960232, 0.007871771358, 0.01105438212, 0.0007573555302, 0.08632857529, 9.070439159, 0.7583068073, 1.516271263],
                [0.0692636014, 0.4327265486, 0.004980729877, 1.638688413, 15.40937851, 1.829290121, 0.2825341909, 0.005614337126, 0.01044184174, 0.0007222199806, 0.03951756653, 3.374075748, 1.163636573, 1.38632687],
                [0.06764788366, 0.4939369088, 0.01534910545, 1.452496438, 88.58907699, 8.763354251, -0.3347169528, 0.0099656679, 0.01598895065, 0.004048709638, 0.05658818837, 33.4354874, 6.012850908, 0.9447323965],
                [0.04296669714, 0.3563785917, 0.02615595685, 1.386379232, 23.63439411, 13.646813, -1.134623804, 0.006263787607, 0.006264124443, 0.00304539437, 0.04090113439, 3.865315662, 7.084083889, 0.9291502907],
                [0.08673921058, 0.5048317574, 0.01025211689, 1.280483048, 16.04263383, 3.055106892, -0.6378007143, 0.0103379805, 0.01078218681, 0.002334856989, 0.0487064344, 6.600723459, 1.754968117, 1.501188589],
            ],
        );
        check(
            RosettaVersion::V2,
            Estimate::Geometric,
            &[
                [0.0615828898, 0.4008016149, 0.004154618595, 1.505378762, 12.61419739, 1.187965635, 0.3503345459, 0.009603561056, 0.01155182931, 0.000841332021, 0.07102758488, 9.070439159, 0.6485224315, 1.388085741],
                [0.06711642975, 0.4292973832, 0.005454035481, 1.394486531, 15.01917991, 1.459761101, 0.2010992687, 0.005440292675, 0.01151735144, 0.0008414139168, 0.02199180172, 3.374075748, 0.8140375989, 1.403788341],
                [0.06730964425, 0.4959369088, 0.01918187387, 1.287608948, 83.15191959, 8.471974599, -1.040757468, 0.009915839561, 0.01598895065, 0.005389759727, 0.03304081292, 33.4354874, 5.392066117, 1.102699834],
                [0.0496695019, 0.3581127483, 0.02629530463, 1.314590406, 23.32121329, 9.966973885, -1.496145677, 0.007240938474, 0.006101257207, 0.003205230248, 0.03087104175, 3.865315662, 6.648944276, 1.086488269],
                [0.1058218369, 0.5075491865, 0.01118639717, 1.232809143, 14.80666012, 2.031192713, -1.391828274, 0.01261233621, 0.01126738521, 0.002479635207, 0.04114830278, 6.600723459, 1.598492493, 1.717329535],
            ],
        );
    }

    #[test]
    fn models_and_texture() {
        let d = SoilData { sand: 30.0, silt: 30.0, clay: 40.0, bulk_density: Some(1.5), th33: Some(0.3), th1500: Some(0.1) };
        assert_eq!(d.best_model(), Some(5));
        assert_eq!(predict(RosettaVersion::V3, &d, Some(2), Estimate::Arithmetic).unwrap().model, 2);
        assert!(predict(RosettaVersion::V3, &SoilData::texture(30.0, 30.0, 30.0), None, Estimate::Arithmetic).is_err());
        assert!(predict(RosettaVersion::V3, &SoilData::texture(40.0, 40.0, 20.0), Some(3), Estimate::Arithmetic).is_err());
        assert_eq!(texture_class(40.0, 40.0, 20.0), "Loam");
        assert_eq!(texture_class(90.0, 5.0, 5.0), "Sand");
        assert_eq!(texture_class(20.0, 60.0, 20.0), "Silt loam");
        assert_eq!(texture_class(10.0, 40.0, 50.0), "Silty clay");
        assert_eq!(texture_class(30.0, 30.0, 40.0), "Clay");
        assert_eq!(texture_class(30.0, 35.0, 35.0), "Clay loam");
        assert_eq!(texture_class(60.0, 10.0, 30.0), "Sandy clay loam");
    }
}
