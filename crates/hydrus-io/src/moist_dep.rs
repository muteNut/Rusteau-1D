//! Parser and serializer for HYDRUS-1D MoistDep.in files.

use hydrus_core::model::{MoistDepTable, Project};
use hydrus_core::HydrusError;
use std::fs::File;
use std::io::{BufRead, BufReader, Write};
use std::path::Path;

pub fn read_moist_dep<P: AsRef<Path>>(path: P, prj: &mut Project) -> Result<(), HydrusError> {
    let file = File::open(path).map_err(|e| HydrusError::Io(e.to_string()))?;
    let reader = BufReader::new(file);
    let mut lines = reader.lines().filter_map(|l| l.ok());

    // Skip main header line
    lines.next();

    let n_mat = prj.water.materials.len();
    let n_sol = prj.n_solutes();
    // MoistDepIn: per material a header line; per solute a header line followed by 13 rows of
    // (ReacMin0, Theta0, Theta1, Theta2, Theta3, ReacMin1). Stored material-major.
    let mut tables = Vec::with_capacity(n_mat * n_sol);
    for _ in 0..n_mat {
        lines.next(); // "Material M"
        for _ in 0..n_sol {
            lines.next(); // "Solute j"
            let mut tab = MoistDepTable::default();
            for _ in 0..13 {
                let row = lines.next().ok_or_else(|| HydrusError::Parse("MoistDep.in: unexpected end of file".into()))?;
                let vals: Vec<f64> = row.split_whitespace().filter_map(|s| s.parse().ok()).collect();
                if vals.len() < 6 {
                    return Err(HydrusError::Parse(format!("MoistDep.in: expected 6 values, found '{}'", row.trim())));
                }
                tab.react.push([vals[0], vals[1], vals[2], vals[3], vals[4], vals[5]]);
            }
            tables.push(tab);
        }
    }

    prj.solute.moist_tables = tables;
    Ok(())
}

pub fn write_moist_dep<P: AsRef<Path>>(path: P, prj: &Project) -> Result<(), HydrusError> {
    let mut file = File::create(path).map_err(|e| HydrusError::Io(e.to_string()))?;
    let io = |e: std::io::Error| HydrusError::Io(e.to_string());
    writeln!(file, "Heading: Reaction rate dependence on water content").map_err(io)?;
    let n_sol = prj.solute.species.len();
    for (m_idx, _) in prj.water.materials.iter().enumerate() {
        writeln!(file, "Material {}", m_idx + 1).map_err(io)?;
        for j_idx in 0..n_sol {
            writeln!(file, "Solute {}", j_idx + 1).map_err(io)?;
            let tab = prj.solute.moist_tables.get(m_idx * n_sol + j_idx);
            for r in 0..13 {
                let p = tab.and_then(|t| t.react.get(r)).copied().unwrap_or([1.0, 0.0, 0.0, 0.0, 0.0, 1.0]);
                writeln!(file, "{:10.4} {:10.4} {:10.4} {:10.4} {:10.4} {:10.4}", p[0], p[1], p[2], p[3], p[4], p[5]).map_err(io)?;
            }
        }
    }
    Ok(())
}
