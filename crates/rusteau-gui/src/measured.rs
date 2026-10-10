//! Measured data shown with the results (overlay): read from a text file with two numeric columns or taken from the
//! observations of a HYDRUS `Fit.in`.

use rusteau_core::inverse::InverseSettings;
use std::path::Path;

/// Plot a measured data set belongs to.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Target {
    /// Observation-node time series: variable index of the observation page (0 h, 1 θ, 2 T, 3 flux, 4 + j
    /// concentration j) and observation node index (0-based in the list of observation nodes)
    Obs { var: usize, obs: usize },
    /// Profile at time `t`: variable index of the profile page; points are (value, depth)
    Profile { var: usize, t: f64 },
    /// Boundary-flux page group (0 fluxes, 1 cumulative fluxes, 2 heads, ...); points are (time, value)
    Flux { group: usize },
    /// Retention data θ(h): points are (h, θ)
    Retention,
    /// Conductivity data K(h): points are (h, K)
    Conductivity,
}

#[derive(Clone, Debug)]
pub struct Measured {
    pub name: String,
    pub target: Target,
    pub pts: Vec<[f64; 2]>,
    pub show: bool,
}

/// Reads the first two numbers of every line (separated by blanks, tabs, commas or semicolons; decimal point or, with
/// semicolons as separators, decimal comma). Lines without two numbers (headers, comments) are skipped.
pub fn read_xy(path: &Path) -> Result<Vec<[f64; 2]>, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut pts = vec![];
    for line in text.lines() {
        let semi = line.contains(';');
        let nums: Vec<f64> = line
            .split(|c: char| c.is_whitespace() || c == ';' || (!semi && c == ','))
            .filter(|s| !s.is_empty())
            .map(|s| if semi { s.replace(',', ".") } else { s.to_string() })
            .map_while(|s| s.parse::<f64>().ok())
            .collect();
        if nums.len() >= 2 && nums[0].is_finite() && nums[1].is_finite() {
            pts.push([nums[0], nums[1]]);
        }
    }
    if pts.is_empty() {
        return Err(format!("{}: no lines with two numbers", path.display()));
    }
    Ok(pts)
}

/// Data sets of the observations in a `Fit.in`: type 1 / 2 / 4 at observation node `position` (h, θ, concentration of
/// solute 1), 0 / 3 cumulative / actual boundary flux (position 1 top, 2 bottom), 5 / 6 retention and conductivity data.
pub fn from_fit(s: &InverseSettings) -> Vec<Measured> {
    let mut out: Vec<Measured> = vec![];
    for o in &s.observations {
        let p = o.pos;
        let (target, name) = match o.kind {
            1 if p > 0 => (Target::Obs { var: 0, obs: p as usize - 1 }, format!("Fit.in h, obs. node {p}")),
            2 if p > 0 => (Target::Obs { var: 1, obs: p as usize - 1 }, format!("Fit.in θ, obs. node {p}")),
            4 if p > 0 => (Target::Obs { var: 4, obs: p as usize - 1 }, format!("Fit.in c, obs. node {p}")),
            0 => (Target::Flux { group: 1 }, format!("Fit.in cumulative flux {}", if p == 2 { "bottom" } else { "top" })),
            3 => (Target::Flux { group: 0 }, format!("Fit.in flux {}", if p == 2 { "bottom" } else { "top" })),
            5 => (Target::Retention, "Fit.in θ(h)".to_string()),
            6 => (Target::Conductivity, "Fit.in K(h)".to_string()),
            _ => continue,
        };
        match out.iter_mut().find(|m| m.name == name) {
            Some(m) => m.pts.push([o.x, o.value]),
            None => out.push(Measured { name, target, pts: vec![[o.x, o.value]], show: true }),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn two_column_text_is_read_with_any_separator() {
        let dir = std::env::temp_dir().join(format!("rusteau_measured_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let p = dir.join("a.csv");
        std::fs::write(&p, "time,h\n# comment\n1, -10\n2,-20.5,7\n3\t-30\n").unwrap();
        assert_eq!(read_xy(&p).unwrap(), vec![[1.0, -10.0], [2.0, -20.5], [3.0, -30.0]]);
        std::fs::write(&p, "t;theta\n1,5;0,25\n").unwrap();
        assert_eq!(read_xy(&p).unwrap(), vec![[1.5, 0.25]]);
        std::fs::write(&p, "nothing here\n").unwrap();
        assert!(read_xy(&p).is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
