use serde::{Deserialize, Serialize};
use crate::error::HydrusError;
use crate::model::Project;
use crate::sim::Simulation;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum WaterField {
    Qr,
    Qs,
    Alpha,
    N,
    Ks,
    L,
    AlphaW,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SolMatField {
    BulkDensity,
    DispL,
    Frac,
    ThImmobile,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SolField {
    DiffW,
    DiffG,
    Ks,
    Nu,
    Beta,
    Henry,
    Omega,
    // Plain first-order decay in liquid/solid/gas phase (Fortran GamL/GamS/GamG,
    // Fit.in labels "SnkL1"/"SnkS1"/"SnkG1", no suffix) -> model.rs `mu_w`/`mu_s`/`mu_g`.
    MuW,
    MuS,
    MuG,
    // Chain-linked first-order decay that feeds the *next* solute in the chain
    // (Fortran GamL1/GamS1/GamG1, Fit.in labels "SnkL1'"/"SnkS1'"/"SnkG1'" WITH the
    // prime) -> model.rs `gam_w`/`gam_s`/`gam_g`. NOTE: earlier code named these
    // "MuWPrime"/"MuSPrime" but mapped them to mu0_w/mu0_s (zero-order production) —
    // that was wrong; the prime denotes chain decay, not zero-order production.
    GamW,
    GamS,
    GamG,
    // Zero-order production (Fortran xMuL/xMuS/xMuG, Fit.in labels "SnkL0"/"SnkS0"/
    // "SnkG0", trailing zero) -> model.rs `mu0_w`/`mu0_s`/`mu0_g`.
    Mu0W,
    Mu0S,
    Mu0G,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ParamKind {
    Water { mat: usize, field: WaterField },
    SoluteMaterial { mat: usize, field: SolMatField },
    SoluteSpecies { mat: usize, sol: usize, field: SolField },
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ObservationPoint {
    pub id: usize,
    pub time: f64,
    pub value: f64,
    pub q_type: i32,
    pub position: usize,
    pub weight: f64,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ParamSpec {
    pub name: String,
    pub initial_val: f64,
    pub is_fitted: bool,
    pub is_log_transformed: bool,
    pub min_val: f64,
    pub max_val: f64,
    pub kind: Option<ParamKind>,
}

impl ParamSpec {
    pub fn to_space(&self, val: f64) -> f64 {
        if self.is_log_transformed {
            val.max(1e-30).log10()
        } else {
            val
        }
    }

    pub fn from_space(&self, x: f64) -> f64 {
        if self.is_log_transformed {
            10.0_f64.powf(x)
        } else {
            x
        }
    }

    pub fn bounds_in_space(&self) -> (f64, f64) {
        if self.is_log_transformed {
            (self.min_val.max(1e-30).log10(), self.max_val.max(1e-30).log10())
        } else {
            (self.min_val, self.max_val)
        }
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ParameterStat {
    pub name: String,
    pub value: f64,
    pub se: f64,
    pub lower_95: f64,
    pub upper_95: f64,
}

#[derive(Clone, Debug, Default)]
pub struct InverseSettings {
    pub max_iter: usize,
    pub n_obs: usize,
    pub weight_option: i32,
    pub l_water_flow: bool,
    pub l_solute: bool,
    pub param_specs: Vec<ParamSpec>,
    pub observations: Vec<ObservationPoint>,
}

#[derive(Clone, Debug, Default)]
pub struct OptimizationState {
    pub settings: InverseSettings,
    pub current_ssq: f64,
    pub iterations_run: usize,
    pub converged: bool,
    pub simulated_values: Vec<f64>,
    pub r_squared: f64,
    pub parameters: Vec<ParameterStat>,
    pub correlation_matrix: Vec<Vec<f64>>,
}

impl OptimizationState {
    pub fn calculate_ssq(&self, simulated_values: &[f64]) -> f64 {
        self.settings
            .observations
            .iter()
            .zip(simulated_values.iter())
            .map(|(obs, &sim)| {
                let residual = obs.value - sim;
                obs.weight * residual * residual
            })
            .sum()
    }

    pub fn get_param_by_spec(&self, prj: &Project, spec: &ParamSpec, _spec_idx: usize) -> f64 {
        if let Some(ref kind) = spec.kind {
            return match kind {
                ParamKind::Water { mat, field } => {
                    let m = prj.water.materials.get(*mat).cloned().unwrap_or_default();
                    match field {
                        WaterField::Qr => m.qr,
                        WaterField::Qs => m.qs,
                        WaterField::Alpha => m.alpha,
                        WaterField::N => m.n,
                        WaterField::Ks => m.ks,
                        WaterField::L => m.l,
                        WaterField::AlphaW => m.alpha_w,
                    }
                }
                ParamKind::SoluteMaterial { mat, field } => {
                    let m = prj.solute.materials.get(*mat).cloned().unwrap_or_default();
                    match field {
                        SolMatField::BulkDensity => m.bulk_density,
                        SolMatField::DispL => m.disp_l,
                        SolMatField::Frac => m.frac,
                        SolMatField::ThImmobile => m.th_immobile,
                    }
                }
                ParamKind::SoluteSpecies { mat, sol, field } => {
                    match field {
                        SolField::DiffW => prj.solute.species.get(*sol).map(|s| s.diff_w).unwrap_or(0.0),
                        SolField::DiffG => prj.solute.species.get(*sol).map(|s| s.diff_g).unwrap_or(0.0),
                        other => {
                            let pm = prj.solute.species.get(*sol)
                                .and_then(|s| s.per_material.get(*mat))
                                .cloned()
                                .unwrap_or_default();
                            match other {
                                SolField::Ks => pm.ks,
                                SolField::Nu => pm.nu,
                                SolField::Beta => pm.beta,
                                SolField::Henry => pm.henry,
                                SolField::Omega => pm.omega,
                                SolField::MuW => pm.mu_w,
                                SolField::MuS => pm.mu_s,
                                SolField::MuG => pm.mu_g,
                                SolField::GamW => pm.gam_w,
                                SolField::GamS => pm.gam_s,
                                SolField::GamG => pm.gam_g,
                                SolField::Mu0W => pm.mu0_w,
                                SolField::Mu0S => pm.mu0_s,
                                SolField::Mu0G => pm.mu0_g,
                                SolField::DiffW | SolField::DiffG => unreachable!(),
                            }
                        }
                    }
                }
            };
        }
        spec.initial_val
    }

    pub fn set_param_by_spec(&self, prj: &mut Project, spec: &ParamSpec, _spec_idx: usize, val: f64) {
        if let Some(ref kind) = spec.kind {
            match kind {
                ParamKind::Water { mat, field } => {
                    if let Some(m) = prj.water.materials.get_mut(*mat) {
                        match field {
                            WaterField::Qr => m.qr = val,
                            WaterField::Qs => m.qs = val,
                            WaterField::Alpha => m.alpha = val,
                            WaterField::N => m.n = val,
                            WaterField::Ks => m.ks = val,
                            WaterField::L => m.l = val,
                            WaterField::AlphaW => m.alpha_w = val,
                        }
                    }
                }
                ParamKind::SoluteMaterial { mat, field } => {
                    if let Some(m) = prj.solute.materials.get_mut(*mat) {
                        match field {
                            SolMatField::BulkDensity => m.bulk_density = val,
                            SolMatField::DispL => m.disp_l = val,
                            SolMatField::Frac => m.frac = val,
                            SolMatField::ThImmobile => m.th_immobile = val,
                        }
                    }
                }
                ParamKind::SoluteSpecies { mat, sol, field } => {
                    match field {
                        SolField::DiffW => {
                            if let Some(sp) = prj.solute.species.get_mut(*sol) {
                                sp.diff_w = val;
                            }
                        }
                        SolField::DiffG => {
                            if let Some(sp) = prj.solute.species.get_mut(*sol) {
                                sp.diff_g = val;
                            }
                        }
                        other => {
                            if let Some(sp) = prj.solute.species.get_mut(*sol) {
                                if let Some(pm) = sp.per_material.get_mut(*mat) {
                                    match other {
                                        SolField::Ks => pm.ks = val,
                                        SolField::Nu => pm.nu = val,
                                        SolField::Beta => pm.beta = val,
                                        SolField::Henry => pm.henry = val,
                                        SolField::Omega => pm.omega = val,
                                        SolField::MuW => pm.mu_w = val,
                                        SolField::MuS => pm.mu_s = val,
                                        SolField::MuG => pm.mu_g = val,
                                        SolField::GamW => pm.gam_w = val,
                                        SolField::GamS => pm.gam_s = val,
                                        SolField::GamG => pm.gam_g = val,
                                        SolField::Mu0W => pm.mu0_w = val,
                                        SolField::Mu0S => pm.mu0_s = val,
                                        SolField::Mu0G => pm.mu0_g = val,
                                        SolField::DiffW | SolField::DiffG => unreachable!(),
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    pub fn get_active_specs(&self) -> Vec<ParamSpec> {
        self.settings
            .param_specs
            .iter()
            .filter(|s| s.is_fitted)
            .cloned()
            .collect()
    }

    pub fn compute_statistics(&mut self, prj: &Project, simulated_vals: &[f64]) {
        let n = self.settings.observations.len();
        let active_specs: Vec<ParamSpec> = self.get_active_specs();
        let p = active_specs.len();

        if n <= p || p == 0 || simulated_vals.len() != n {
            return;
        }

        // 1. Pearson R^2
        let obs_values: Vec<f64> = self.settings.observations.iter().map(|o| o.value).collect();
        let y_mean: f64 = obs_values.iter().sum::<f64>() / n as f64;
        let f_mean: f64 = simulated_vals.iter().sum::<f64>() / n as f64;

        let mut cov_yf = 0.0;
        let mut var_y = 0.0;
        let mut var_f = 0.0;

        for (i, &y) in obs_values.iter().enumerate() {
            let f = simulated_vals[i];
            let dy = y - y_mean;
            let df = f - f_mean;
            cov_yf += dy * df;
            var_y += dy * dy;
            var_f += df * df;
        }

        let denom = var_y * var_f;
        self.r_squared = if denom > 1e-12 {
            (cov_yf * cov_yf) / denom
        } else {
            0.0
        };

        // 2. Jacobian Matrix FJAC(I,J) in optimization space
        let mut j_matrix = vec![vec![0.0; p]; n];
        for j in 0..p {
            let spec = &active_specs[j];
            let real_val = self.get_param_by_spec(prj, spec, j);
            let x0 = spec.to_space(real_val);
            let (lo, hi) = spec.bounds_in_space();

            let dx = if spec.is_log_transformed {
                0.005_f64
            } else {
                (0.005 * real_val.abs().max(1.0)).max(1e-4)
            };

            let (test_x, actual_dx) = if x0 + dx > hi {
                (x0 - dx, -dx)
            } else if x0 - dx < lo {
                (x0 + dx, dx)
            } else {
                (x0 + dx, dx)
            };

            let mut perturbed_prj = prj.clone();
            self.set_param_by_spec(&mut perturbed_prj, spec, j, spec.from_space(test_x));

            if let Ok(mut sim_p) = Simulation::new(perturbed_prj.clone()) {
                sim_p.run(|_| true);
                for (i, obs) in self.settings.observations.iter().enumerate() {
                    let val_p = extract_simulated_quantity(&sim_p, &perturbed_prj, obs.time, obs.position, obs.q_type);
                    j_matrix[i][j] = (val_p - simulated_vals[i]) / actual_dx;
                }
            }
        }

        // 3. Normal equations coefficient matrix: A = J^T * W * J
        let mut a = vec![vec![0.0; p]; p];
        for r in 0..p {
            for c in 0..p {
                let mut sum = 0.0;
                for i in 0..n {
                    sum += self.settings.observations[i].weight * j_matrix[i][r] * j_matrix[i][c];
                }
                a[r][c] = sum;
            }
        }

        // 4. Covariance matrix: s^2 = SSQ / (N - P)
        let s2 = (self.current_ssq / (n - p) as f64).max(1e-12);
        let cov = invert_matrix(&a, s2);

        // 5. Correlation matrix: R_jk = Cov(j,k) / sqrt(Cov(j,j) * Cov(k,k))
        let mut corr = vec![vec![1.0; p]; p];
        for r in 0..p {
            for c in 0..p {
                if r != c && cov[r][r] > 0.0 && cov[c][c] > 0.0 {
                    corr[r][c] = (cov[r][c] / (cov[r][r].sqrt() * cov[c][c].sqrt())).clamp(-1.0, 1.0);
                }
            }
        }
        self.correlation_matrix = corr;

        // 6. Parameter statistics: Delta method for log-transformed parameters
        let t_val = 1.96;
        self.parameters.clear();
        for j in 0..p {
            let spec = &active_specs[j];
            let val = self.get_param_by_spec(prj, spec, j);
            let se_space = if cov[j][j] > 0.0 { cov[j][j].sqrt() } else { 0.0 };
            let se = if spec.is_log_transformed {
                se_space * val * std::f64::consts::LN_10
            } else {
                se_space
            };

            self.parameters.push(ParameterStat {
                name: spec.name.clone(),
                value: val,
                se,
                lower_95: val - t_val * se,
                upper_95: val + t_val * se,
            });
        }
    }

    pub fn optimize(&mut self, prj: &mut Project) -> Result<(), HydrusError> {
        let max_it = self.settings.max_iter.max(1);
        let n = self.settings.observations.len();
        let active_specs = self.get_active_specs();
        let p = active_specs.len();

        if n <= p || p == 0 {
            return Ok(());
        }
        for (j, spec) in active_specs.iter().enumerate() {
            self.set_param_by_spec(prj, spec, j, spec.initial_val);
        }

        // Ensure POS = 0 profile print times exist
        for obs in &self.settings.observations {
            if obs.position == 0 && !prj.time.print_times.iter().any(|&pt| (pt - obs.time).abs() < 1e-4) {
                prj.time.print_times.push(obs.time);
            }
        }
        prj.time.print_times.sort_by(|a, b| a.partial_cmp(b).unwrap());

        // Grouped weight normalization matching the verified reference landscape
        if self.settings.weight_option == 2 && !self.settings.observations.is_empty() {
            use std::collections::HashMap;

            let mut group_sums: HashMap<i32, f64> = HashMap::new();
            let mut group_counts: HashMap<i32, usize> = HashMap::new();

            for obs in &self.settings.observations {
                if obs.q_type > 0 {
                    *group_sums.entry(obs.q_type).or_insert(0.0) += obs.value.abs();
                    *group_counts.entry(obs.q_type).or_insert(0) += 1;
                }
            }

            for obs in &mut self.settings.observations {
                if obs.q_type > 0 {
                    if let (Some(&sum), Some(&count)) = (group_sums.get(&obs.q_type), group_counts.get(&obs.q_type)) {
                        if count > 0 && sum > 1e-12 {
                            let mean_val = sum / count as f64;
                            obs.weight /= (count as f64) * mean_val;
                        }
                    }
                }
            }
        }

        let mut flambda: f64 = 0.001;

        // Baseline forward run
        let mut sim = Simulation::new(prj.clone())?;
        sim.run(|_| true);
        let sim_vals: Vec<f64> = self.settings.observations.iter()
            .map(|obs| extract_simulated_quantity(&sim, prj, obs.time, obs.position, obs.q_type))
            .collect();

        let mut current_ssq = self.calculate_ssq(&sim_vals);
        self.current_ssq = current_ssq;
        self.simulated_values = sim_vals;

        for iter in 0..max_it {
            self.iterations_run = iter + 1;

            // Current values in optimization space
            let mut current_x: Vec<f64> = vec![0.0; p];
            for j in 0..p {
                let real = self.get_param_by_spec(prj, &active_specs[j], j);
                current_x[j] = active_specs[j].to_space(real);
            }

            // 1. Compute Jacobian in optimization space
            let mut j_mat: Vec<Vec<f64>> = vec![vec![0.0; p]; n];
            for j in 0..p {
                let spec = &active_specs[j];
                let (lo, hi) = spec.bounds_in_space();
                let x0 = current_x[j];

                let dx = if spec.is_log_transformed {
                    0.01_f64
                } else {
                    let real = spec.from_space(x0);
                    (0.01 * real.abs().max(1.0)).max(1e-3)
                };

                let (test_x, actual_dx) = if x0 + dx > hi {
                    (x0 - dx, -dx)
                } else if x0 - dx < lo {
                    (x0 + dx, dx)
                } else {
                    (x0 + dx, dx)
                };

                let mut perturbed = prj.clone();
                self.set_param_by_spec(&mut perturbed, spec, j, spec.from_space(test_x));

                if let Ok(mut sim_p) = Simulation::new(perturbed.clone()) {
                    sim_p.run(|_| true);
                    for (i, obs) in self.settings.observations.iter().enumerate() {
                        let val_p = extract_simulated_quantity(&sim_p, &perturbed, obs.time, obs.position, obs.q_type);
                        j_mat[i][j] = (val_p - self.simulated_values[i]) / actual_dx;
                    }
                }
            }

            // 2. Normal equations: A = J^T * W * J, B = J^T * W * (Y - Y_hat)
            let mut a: Vec<Vec<f64>> = vec![vec![0.0; p]; p];
            let mut b: Vec<f64> = vec![0.0; p];
            for i in 0..n {
                let w = self.settings.observations[i].weight;
                let res = self.settings.observations[i].value - self.simulated_values[i];
                for r in 0..p {
                    b[r] += w * j_mat[i][r] * res;
                    for c in 0..p {
                        a[r][c] += w * j_mat[i][r] * j_mat[i][c];
                    }
                }
            }

            // 3. Marquardt diagonal scaling
            let mut c_diag: Vec<f64> = vec![0.0; p];
            for j in 0..p {
                c_diag[j] = if a[j][j] > 1e-20 { a[j][j].sqrt() } else { 1.0 };
            }

            // 4. Inner Marquardt trial loop
            let mut step_accepted = false;
            for _trial in 0..15 {
                let mut a_norm: Vec<Vec<f64>> = vec![vec![0.0; p]; p];
                let mut b_norm: Vec<f64> = vec![0.0; p];
                for r in 0..p {
                    b_norm[r] = b[r] / c_diag[r];
                    for c in 0..p {
                        a_norm[r][c] = a[r][c] / (c_diag[r] * c_diag[c]);
                    }
                    a_norm[r][r] += flambda;
                }

                let dx_norm = solve_linear_system(&a_norm, &b_norm);
                if dx_norm.is_empty() {
                    flambda = (flambda * 10.0).min(1e10);
                    continue;
                }

                let mut dx: Vec<f64> = vec![0.0; p];
                for j in 0..p {
                    dx[j] = dx_norm[j] / c_diag[j];
                }

                // Trust-region rescale: shrink the WHOLE step by one factor so every
                // parameter stays in bounds, rather than clamping each component on its
                // own (which changes the step's direction, not just its length).
                let mut scale = 1.0_f64;
                for j in 0..p {
                    let (lo, hi) = active_specs[j].bounds_in_space();
                    let raw_x = current_x[j] + dx[j];
                    if raw_x > hi && dx[j] > 1e-12 {
                        scale = scale.min(((hi - current_x[j]) / dx[j]).max(0.0));
                    } else if raw_x < lo && dx[j] < -1e-12 {
                        scale = scale.min(((lo - current_x[j]) / dx[j]).max(0.0));
                    }
                }
                let any_clamped = scale < 0.999;
                if scale < 1e-6 {
                    // Already sitting on the wall and the LM direction points further
                    // out of bounds: no feasible step to try. Shrink the trust region
                    // (grow lambda) instead of wasting a forward run on a ~zero step.
                    flambda = (flambda * 10.0).min(1e10);
                    continue;
                }
                let applied: Vec<f64> = dx.iter().map(|d| d * scale).collect();

                let mut candidate_prj = prj.clone();
                for j in 0..p {
                    let spec = &active_specs[j];
                    let (lo, hi) = spec.bounds_in_space();
                    let new_x = (current_x[j] + applied[j]).clamp(lo, hi);
                    self.set_param_by_spec(&mut candidate_prj, spec, j, spec.from_space(new_x));
                }

                if let Ok(mut cand_sim) = Simulation::new(candidate_prj.clone()) {
                    cand_sim.run(|_| true);
                    let cand_vals: Vec<f64> = self.settings.observations.iter()
                        .map(|obs| extract_simulated_quantity(&cand_sim, &candidate_prj, obs.time, obs.position, obs.q_type))
                        .collect();
                    let cand_ssq = self.calculate_ssq(&cand_vals);

                    if cand_ssq < current_ssq {
                        flambda = if any_clamped {
                            (flambda * 2.0).min(1e10)
                        } else {
                            (flambda / 1.5).max(1e-4)
                        };
                        current_ssq = cand_ssq;
                        self.current_ssq = current_ssq;
                        self.simulated_values = cand_vals;
                        *prj = candidate_prj;
                        step_accepted = true;

                        let max_change = applied.iter().map(|d| d.abs()).fold(0.0, f64::max);
                        if max_change < 1e-4 && iter >= 3 {
                            self.converged = true;
                        }
                        break;
                    } else {
                        flambda = (flambda * 10.0).min(1e10);
                    }
                } else {
                    flambda = (flambda * 10.0).min(1e10);
                }
            }

            if self.converged || !step_accepted {
                break;
            }
        }

        self.compute_statistics(prj, &self.simulated_values.clone());
        Ok(())
    }
}

fn solve_linear_system(a: &[Vec<f64>], b: &[f64]) -> Vec<f64> {
    let n = b.len();
    if n == 0 {
        return Vec::new();
    }
    if n == 1 {
        if a[0][0].abs() > 1e-18 {
            return vec![b[0] / a[0][0]];
        }
        return Vec::new();
    }

    let mut aug = vec![vec![0.0; n + 1]; n];
    for i in 0..n {
        for j in 0..n {
            aug[i][j] = a[i][j];
        }
        aug[i][n] = b[i];
    }

    for col in 0..n {
        let mut max_row = col;
        let mut max_val = aug[col][col].abs();
        for row in (col + 1)..n {
            if aug[row][col].abs() > max_val {
                max_val = aug[row][col].abs();
                max_row = row;
            }
        }

        if max_val < 1e-18 {
            return Vec::new();
        }

        aug.swap(col, max_row);

        let pivot = aug[col][col];
        for j in col..=n {
            aug[col][j] /= pivot;
        }

        for row in 0..n {
            if row != col {
                let factor = aug[row][col];
                if factor.abs() > 1e-18 {
                    for j in col..=n {
                        aug[row][j] -= factor * aug[col][j];
                    }
                }
            }
        }
    }

    let mut x = vec![0.0; n];
    for i in 0..n {
        x[i] = aug[i][n];
    }
    x
}

fn invert_matrix(a: &[Vec<f64>], scale: f64) -> Vec<Vec<f64>> {
    let n = a.len();
    if n == 0 {
        return Vec::new();
    }
    if n == 1 {
        let inv00 = if a[0][0].abs() > 1e-18 { (1.0 / a[0][0]) * scale } else { 0.0 };
        return vec![vec![inv00]];
    }

    let mut aug = vec![vec![0.0; 2 * n]; n];
    for i in 0..n {
        for j in 0..n {
            aug[i][j] = a[i][j];
        }
        aug[i][n + i] = 1.0;
    }

    for col in 0..n {
        let mut max_row = col;
        let mut max_val = aug[col][col].abs();
        for row in (col + 1)..n {
            if aug[row][col].abs() > max_val {
                max_val = aug[row][col].abs();
                max_row = row;
            }
        }

        if max_val < 1e-18 {
            return vec![vec![0.0; n]; n];
        }

        aug.swap(col, max_row);

        let pivot = aug[col][col];
        for j in 0..(2 * n) {
            aug[col][j] /= pivot;
        }

        for row in 0..n {
            if row != col {
                let factor = aug[row][col];
                if factor.abs() > 1e-18 {
                    for j in 0..(2 * n) {
                        aug[row][j] -= factor * aug[col][j];
                    }
                }
            }
        }
    }

    let mut inv = vec![vec![0.0; n]; n];
    for i in 0..n {
        for j in 0..n {
            inv[i][j] = aug[i][n + j] * scale;
        }
    }
    inv
}

fn extract_simulated_quantity(sim: &Simulation, prj: &Project, time: f64, node_pos: usize, q_type: i32) -> f64 {
    let mat_idx = node_pos.saturating_sub(1);

    // Type 5: Retention point theta(h)
    if q_type == 5 {
        if let Some(mat) = prj.water.materials.get(mat_idx).or_else(|| prj.water.materials.first()) {
            let h = -time.abs();
            let ah = (mat.alpha * h.abs()).powf(mat.n);
            let m = 1.0 - 1.0 / mat.n;
            let theta = mat.qr + (mat.qs - mat.qr) / (1.0 + ah).powf(m);
            return theta;
        }
    }

    // Type 6: Conductivity point K(h)
    if q_type == 6 {
        if let Some(mat) = prj.water.materials.get(mat_idx).or_else(|| prj.water.materials.first()) {
            let h = -time.abs();
            let ah = (mat.alpha * h.abs()).powf(mat.n);
            let m = 1.0 - 1.0 / mat.n;
            let se = 1.0 / (1.0 + ah).powf(m);
            let k = mat.ks * se.powf(mat.l) * (1.0 - (1.0 - se.powf(1.0 / m)).powf(m)).powi(2);
            return k;
        }
    }

    // 1. Cumulative boundary fluxes (Types <= 0): Pos 1 = Top, Pos 2 = Bottom
    if q_type <= 0 {
        if !sim.res.tlevel.is_empty() {
            let tl_list = &sim.res.tlevel;
            let val_at = |idx: usize| -> f64 {
                let tl = &tl_list[idx];
                if node_pos == 1 { tl.cum_v_top } else { tl.cum_v_bot }
            };

            if time <= tl_list.first().unwrap().t {
                return val_at(0);
            }
            if time >= tl_list.last().unwrap().t {
                return val_at(tl_list.len() - 1);
            }

            let mut left = 0;
            for (k, tl) in tl_list.iter().enumerate() {
                if tl.t <= time {
                    left = k;
                } else {
                    break;
                }
            }
            let right = (left + 1).min(tl_list.len() - 1);
            let t0 = tl_list[left].t;
            let t1 = tl_list[right].t;
            let v0 = val_at(left);
            let v1 = val_at(right);

            return if (t1 - t0).abs() > 1e-12 {
                v0 + (time - t0) / (t1 - t0) * (v1 - v0)
            } else {
                v0
            };
        }
    } else if q_type == 3 {
        // Instantaneous (non-cumulative) boundary flux: Pos 1 = vTop, Pos 2 = vBot.
        // Confirmed against FITTEST2's Fit.out: Type=3/Pos=2 observations track vBot's
        // magnitude decaying over time, distinct from Type<=0's cumulative growth.
        if !sim.res.tlevel.is_empty() {
            let tl_list = &sim.res.tlevel;
            let val_at = |idx: usize| -> f64 {
                let tl = &tl_list[idx];
                if node_pos == 1 { tl.v_top } else { tl.v_bot }
            };

            if time <= tl_list.first().unwrap().t {
                return val_at(0);
            }
            if time >= tl_list.last().unwrap().t {
                return val_at(tl_list.len() - 1);
            }

            let mut left = 0;
            for (k, tl) in tl_list.iter().enumerate() {
                if tl.t <= time {
                    left = k;
                } else {
                    break;
                }
            }
            let right = (left + 1).min(tl_list.len() - 1);
            let t0 = tl_list[left].t;
            let t1 = tl_list[right].t;
            let v0 = val_at(left);
            let v1 = val_at(right);

            return if (t1 - t0).abs() > 1e-12 {
                v0 + (time - t0) / (t1 - t0) * (v1 - v0)
            } else {
                v0
            };
        }
    } else if q_type == 7 {
        if let Some(tl) = sim.res.tlevel.iter().min_by(|a, b| (a.t - time).abs().partial_cmp(&(b.t - time).abs()).unwrap()) {
            return tl.v_top;
        }
    }

    // 2. Solute concentrations (Type 4)
    if q_type == 4 {
        if node_pos == 0 {
            if let Some(profile) = sim.res.profiles.iter().min_by(|a, b| (a.t - time).abs().partial_cmp(&(b.t - time).abs()).unwrap()) {
                let n_nodes = profile.nodes.len();
                let mut total_mass = 0.0;
                let sp = prj.solute.species.first();
                for i in 0..n_nodes.saturating_sub(1) {
                    let dx = (sim.x[i + 1] - sim.x[i]).abs();
                    let m = sim.mat.get(i).copied().unwrap_or(0);
                    let ro = prj.solute.materials.get(m).map(|sm| sm.bulk_density).unwrap_or(1.0);
                    let frac = prj.solute.materials.get(m).map(|sm| sm.frac).unwrap_or(1.0);
                    let kd = sp.and_then(|s| s.per_material.get(m)).map(|pm| pm.ks).unwrap_or(0.0);

                    let c_i = profile.nodes[i].conc.first().copied().unwrap_or(0.0);
                    let c_next = profile.nodes[i + 1].conc.first().copied().unwrap_or(0.0);
                    let th_i = profile.nodes[i].theta;
                    let th_next = profile.nodes[i + 1].theta;

                    let sk_i = profile.nodes[i].sorb.first().copied().unwrap_or(0.0);
                    let sk_next = profile.nodes[i + 1].sorb.first().copied().unwrap_or(0.0);

                    let m_i = th_i * c_i + ro * (frac * kd * c_i + sk_i);
                    let m_next = th_next * c_next + ro * (frac * kd * c_next + sk_next);

                    total_mass += dx * (m_i + m_next) / 2.0;
                }
                return total_mass;
            }
        }

        if !sim.res.obs.is_empty() {
            let obs_idx = node_pos.saturating_sub(1);
            let obs_list = &sim.res.obs;

            if time <= obs_list.first().unwrap().t {
                return obs_list.first().unwrap().points.get(obs_idx).and_then(|p| p.conc.first().copied()).unwrap_or(0.0);
            }
            if time >= obs_list.last().unwrap().t {
                return obs_list.last().unwrap().points.get(obs_idx).and_then(|p| p.conc.first().copied()).unwrap_or(0.0);
            }

            let mut left = 0;
            for (k, snap) in obs_list.iter().enumerate() {
                if snap.t <= time {
                    left = k;
                } else {
                    break;
                }
            }
            let right = (left + 1).min(obs_list.len() - 1);
            let t0 = obs_list[left].t;
            let t1 = obs_list[right].t;
            let c0 = obs_list[left].points.get(obs_idx).and_then(|p| p.conc.first().copied()).unwrap_or(0.0);
            let c1 = obs_list[right].points.get(obs_idx).and_then(|p| p.conc.first().copied()).unwrap_or(0.0);

            return if (t1 - t0).abs() > 1e-12 {
                c0 + (time - t0) / (t1 - t0) * (c1 - c0)
            } else {
                c0
            };
        }
    }

    // 3. Water flow state at observation nodes (Type 1 = h, Type 2 = theta) with linear time interpolation
    if (q_type == 1 || q_type == 2) && !sim.res.obs.is_empty() {
        let obs_idx = node_pos.saturating_sub(1);
        let obs_list = &sim.res.obs;

        let val_at = |idx: usize| -> f64 {
            if let Some(pt) = obs_list[idx].points.get(obs_idx) {
                if q_type == 1 { pt.h } else { pt.theta }
            } else {
                0.0
            }
        };

        if time <= obs_list.first().unwrap().t {
            return val_at(0);
        }
        if time >= obs_list.last().unwrap().t {
            return val_at(obs_list.len() - 1);
        }

        let mut left = 0;
        for (k, snap) in obs_list.iter().enumerate() {
            if snap.t <= time {
                left = k;
            } else {
                break;
            }
        }
        let right = (left + 1).min(obs_list.len() - 1);
        let t0 = obs_list[left].t;
        let t1 = obs_list[right].t;
        let v0 = val_at(left);
        let v1 = val_at(right);

        return if (t1 - t0).abs() > 1e-12 {
            v0 + (time - t0) / (t1 - t0) * (v1 - v0)
        } else {
            v0
        };
    }

    // 4. Fallback: Profile records
    let node_idx = node_pos.saturating_sub(1);
    if let Some(profile) = sim.res.profiles.iter().min_by(|a, b| (a.t - time).abs().partial_cmp(&(b.t - time).abs()).unwrap()) {
        if let Some(node) = profile.nodes.get(node_idx) {
            return match q_type {
                1 => node.h,
                2 => node.theta,
                4 => node.conc.first().copied().unwrap_or(0.0),
                _ => node.h,
            };
        }
    }

    0.0
}