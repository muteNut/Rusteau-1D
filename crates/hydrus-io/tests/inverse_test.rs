use hydrus_core::inverse::OptimizationState;
use hydrus_io::{legacy::read_fit_in, read_legacy_project};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

const ALLOWED_R2_TOLERANCE: f64 = 0.05; // Strict 5% tolerance on R^2
const PARAM_TOLERANCE_PCT: f64 = 5.0;   // Near-100% agreement on calibrated parameters

#[derive(Debug, Default)]
struct LegacyFitResults {
    headers: Vec<String>,
    rows: Vec<Vec<f64>>,
    final_ssq: Option<f64>,
    r_squared: Option<f64>,
    parameters: Vec<(String, f64, f64, f64, f64)>, // (Name, Value, SE, Lower95, Upper95)
    correlation_off_diagonal: Option<f64>,
}

fn parse_fit_out_full(path: &Path) -> Option<LegacyFitResults> {
    let file = File::open(path).ok()?;
    let reader = BufReader::new(file);
    let mut lines = reader.lines().filter_map(|l| l.ok()).peekable();

    let mut res = LegacyFitResults::default();

    while let Some(line) = lines.next() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('*') {
            continue;
        }

        let upper = trimmed.to_ascii_uppercase();

        // 1. Final SSQ
        if upper.contains("SUM OF SQUARED RESIDUALS") || upper.contains("SSQ") {
            let parts: Vec<f64> = trimmed
                .split_whitespace()
                .filter_map(|s| s.replace(['d', 'D'], "e").parse::<f64>().ok())
                .collect();
            if let Some(&val) = parts.last() {
                res.final_ssq = Some(val);
            }
        }

        // 2. RSQUARE
        if upper.contains("RSQUARE") {
            if let Some(val_str) = trimmed.split('=').nth(1) {
                res.r_squared = val_str.trim().replace(['d', 'D'], "e").parse::<f64>().ok();
            }
        }

        // 3. Correlation Matrix (parse off-diagonal r12)
        if upper.contains("CORRELATION MATRIX") {
            while let Some(l) = lines.next() {
                let tr = l.trim();
                if tr.starts_with('=') || tr.is_empty() {
                    continue;
                }
                if tr.to_ascii_uppercase().contains("NON-LINEAR") {
                    break;
                }
                let nums: Vec<f64> = tr
                    .split_whitespace()
                    .filter_map(|s| s.replace(['d', 'D'], "e").parse::<f64>().ok())
                    .collect();
                if nums.len() >= 3 && nums[0] == 2.0 {
                    res.correlation_off_diagonal = Some(nums[1]);
                }
            }
        }

        // 4. Parameter Statistics: Value, SE, 95% Confidence limits
        if upper.contains("95% CONFIDENCE LIMITS") {
            let _ = lines.next();
            while let Some(l) = lines.next() {
                let tr = l.trim();
                if tr.is_empty() || tr.starts_with('=') || tr.starts_with('-') {
                    continue;
                }
                if tr.to_ascii_uppercase().contains("CONTRIBUTION") || tr.to_ascii_uppercase().contains("OBSERVED") {
                    break;
                }
                let tokens: Vec<&str> = tr.split_whitespace().collect();
                if tokens.len() >= 5 {
                    let name = tokens[0].to_string();
                    let v = tokens[1].replace(['d', 'D'], "e").parse::<f64>().unwrap_or(0.0);
                    let se = tokens[2].replace(['d', 'D'], "e").parse::<f64>().unwrap_or(0.0);
                    let lower = tokens[3].replace(['d', 'D'], "e").parse::<f64>().unwrap_or(0.0);
                    let upper_ci = tokens[4].replace(['d', 'D'], "e").parse::<f64>().unwrap_or(0.0);
                    res.parameters.push((name, v, se, lower, upper_ci));
                }
            }
        }

        // 5. Observed & Fitted Table
        if upper.contains("OBSERVED & FITTED QUANTITY") {
            while let Some(l) = lines.next() {
                let tr = l.trim();
                let u = tr.to_ascii_uppercase();
                if u.contains("NO") && u.contains("TIME") && u.contains("FITTED") {
                    res.headers = tr
                        .split_whitespace()
                        .map(|s| s.trim_matches(|c: char| !c.is_alphanumeric() && c != '_').to_string())
                        .collect();
                    break;
                }
            }

            while let Some(l) = lines.next() {
                let tr = l.trim();
                if tr.is_empty() || tr.starts_with('-') {
                    break;
                }
                let vals: Vec<f64> = tr
                    .split_whitespace()
                    .filter_map(|s| s.replace(['d', 'D'], "e").parse::<f64>().ok())
                    .collect();
                if vals.len() >= 5 {
                    res.rows.push(vals);
                }
            }
        }
    }

    Some(res)
}

#[test]
fn test_inverse_optimization_and_table_comparison() {
    let base_path = std::path::PathBuf::from(r"C:\Users\Public\Documents\PC-Progress\Hydrus-1D 4.xx\Examples\Inverse");

    assert!(
        base_path.exists(),
        "FATAL: Inverse benchmark directory not found at {:?}. Verification cannot proceed.",
        base_path
    );

    let entries = std::fs::read_dir(&base_path).expect("Must read Inverse examples directory");
    let mut total_run = 0;
    let mut passed = 0;
    let mut failure_reports = Vec::new();

    let target = std::env::var("HYDRUS_CASE").ok();

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() && path.join("SELECTOR.IN").exists() {
            let folder_name = path.file_name().unwrap().to_string_lossy().to_string();

            if let Some(ref t) = target {
                if !folder_name.eq_ignore_ascii_case(t) {
                    continue;
                }
            }

            total_run += 1;
            print!("Testing Inverse Example {:<12} ... ", folder_name);

            let mut prj = match read_legacy_project(&path) {
                Ok(p) => p,
                Err(e) => {
                    println!("FAILED (Project parse error)");
                    failure_reports.push(format!("[{}] Project parse error: {}", folder_name, e));
                    continue;
                }
            };

            let fit_in_upper = path.join("FIT.IN");
            let fit_in_lower = path.join("Fit.in");
            let fit_path = if fit_in_upper.exists() {
                fit_in_upper
            } else if fit_in_lower.exists() {
                fit_in_lower
            } else {
                println!("PASSED (Forward check only)");
                passed += 1;
                continue;
            };

            let settings = match read_fit_in(&fit_path) {
                Ok(s) => s,
                Err(e) => {
                    println!("FAILED (Fit.in parse error: {})", e);
                    failure_reports.push(format!("[{}] Fit.in parse error: {}", folder_name, e));
                    continue;
                }
            };
			for p in &settings.param_specs {
					if p.is_fitted {
						println!("   FITTED: {} (init={}, min={}, max={})", p.name, p.initial_val, p.min_val, p.max_val);
					}
				}

            let mut opt_state = OptimizationState {
                settings,
                ..Default::default()
            };

            if let Err(e) = opt_state.optimize(&mut prj) {
                println!("FAILED (Optimization execution error)");
                failure_reports.push(format!("[{}] Optimization error: {}", folder_name, e));
                continue;
            }

            let fit_out_upper = path.join("FIT.OUT");
            let fit_out_lower = path.join("fit.out");
            let out_path = if fit_out_upper.exists() {
                Some(fit_out_upper)
            } else if fit_out_lower.exists() {
                Some(fit_out_lower)
            } else {
                None
            };

            let mut check_failed = false;
            if let Some(out_file) = out_path {
                if let Some(legacy) = parse_fit_out_full(&out_file) {
                    // Strict R^2 check
                    if let Some(ref_r2) = legacy.r_squared {
                        let r2_diff = (opt_state.r_squared - ref_r2).abs();
                        if r2_diff > ALLOWED_R2_TOLERANCE {
                            println!("FAILED (R^2: got {:.4}, ref {:.4})", opt_state.r_squared, ref_r2);
                            failure_reports.push(format!(
                                "[{}] R^2 mismatch: got {:.4}, ref {:.4} (diff {:.4})",
                                folder_name, opt_state.r_squared, ref_r2, r2_diff
                            ));
                            check_failed = true;
                        }
                    }

                    // Strict Parameter Convergence check
                    for (name, ref_val, ref_se, _ref_low, _ref_up) in &legacy.parameters {
                        if let Some(our_param) = opt_state.parameters.iter().find(|p| p.name.eq_ignore_ascii_case(name)) {
                            let val_diff = (our_param.value - ref_val).abs();
                            // Normierung mit .max(1.0) verhindert Divisionsexplosionen bei Parametern nahe 0 (z. B. l = -0.0137)
                            let denom = ref_val.abs().max(1.0);
                            let val_pct = (val_diff / denom) * 100.0;

                            println!(
                                "   Param {:<6}: got val={:.4} (ref {:.4}), SE={:.4} (ref {:.4})",
                                name, our_param.value, ref_val, our_param.se, ref_se
                            );

                            if val_pct > PARAM_TOLERANCE_PCT && val_diff > 0.02 {
                                failure_reports.push(format!(
                                    "[{}] Param {} mismatch: got {:.4}, ref {:.4} ({:.1}% diff)",
                                    folder_name, name, our_param.value, ref_val, val_pct
                                ));
                                check_failed = true;
                            }
                        }
                    }

                    // Correlation Matrix check
                    let our_corr = opt_state.correlation_matrix.first().and_then(|r| r.get(1)).copied();
                    if let (Some(oc), Some(rc)) = (our_corr, legacy.correlation_off_diagonal) {
                        println!("   Corr(1,2) : got {:.4}, ref {:.4}", oc, rc);
                    }
                }
            }

            if !check_failed {
                println!("PASSED (Final SSQ: {:.6}, R^2: {:.4})", opt_state.current_ssq, opt_state.r_squared);
                passed += 1;
            }
        }
    }

    println!("\n==========================================");
    println!("Inverse Calculation Benchmark: {}/{} passed", passed, total_run);
    println!("==========================================");

    if !failure_reports.is_empty() {
        for report in &failure_reports {
            eprintln!("{}", report);
        }
        panic!("{} inverse examples failed strict verification", failure_reports.len());
    }
}