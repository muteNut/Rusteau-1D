//! Profile discretization from fixed points, regridding with interpolation, adding and removing materials.

use rusteau_core::grid::generate_depths;
use rusteau_core::*;

fn fp(depth: f64, above: f64, below: f64) -> FixedPoint {
    FixedPoint { depth, size_above: above, size_below: below }
}

#[test]
fn uniform_fixed_points_give_equidistant_nodes() {
    let z = generate_depths(&[FixedPoint::new(0.0), FixedPoint::new(60.0)], 60.0, 61).unwrap();
    assert_eq!(z.len(), 61);
    for (i, zi) in z.iter().enumerate() {
        assert!((zi - i as f64).abs() < 1e-9, "node {i}: {zi}");
    }
    // the ends are added when missing
    let z = generate_depths(&[], 10.0, 11).unwrap();
    assert_eq!(z.len(), 11);
    assert!((z[10] - 10.0).abs() < 1e-12 && z[0] == 0.0);
}

#[test]
fn fixed_points_are_nodes_and_element_sizes_follow_the_relative_sizes() {
    let fixed = [fp(0.0, 1.0, 1.0), fp(20.0, 1.0, 1.0), fp(100.0, 5.0, 5.0)];
    let z = generate_depths(&fixed, 100.0, 51).unwrap();
    assert_eq!(z.len(), 51);
    assert!(z.windows(2).all(|w| w[1] > w[0]));
    assert!(z.iter().any(|&x| (x - 20.0).abs() < 1e-9), "20 is a node");
    assert!((z[50] - 100.0).abs() < 1e-12);
    // elements just below 20 are about 1/5 of those at the bottom
    let k = z.iter().position(|&x| (x - 20.0).abs() < 1e-9).unwrap();
    let top = z[k + 1] - z[k];
    let bot = z[50] - z[49];
    assert!((bot / top - 5.0).abs() < 0.2, "ratio {}", bot / top);
    // too few nodes for the fixed points
    assert!(generate_depths(&fixed, 100.0, 2).is_err());
}

#[test]
fn regrid_interpolates_values_and_keeps_observation_depths() {
    let mut p = Profile::default();
    p.nodes = (0..11).map(|i| ProfileNode::new(i as f64 * 10.0, -100.0 - i as f64 * 10.0, if i < 5 { 1 } else { 2 })).collect();
    for n in p.nodes.iter_mut() {
        n.temp = 10.0 + n.depth / 10.0;
        n.conc = vec![n.depth];
    }
    p.observation_nodes = vec![3, 8]; // depths 20 and 70
    let z: Vec<f64> = (0..101).map(|i| i as f64).collect();
    p.regrid(&z);
    assert_eq!(p.nodes.len(), 101);
    for n in &p.nodes {
        assert!((n.h - (-100.0 - n.depth)).abs() < 1e-9);
        assert!((n.temp - (10.0 + n.depth / 10.0)).abs() < 1e-9);
        assert!((n.conc[0] - n.depth).abs() < 1e-9);
    }
    // material boundary between the old nodes at 40 (mat 1) and 50 (mat 2): half-way
    assert_eq!(p.nodes[45].mat, 1);
    assert_eq!(p.nodes[46].mat, 2);
    assert_eq!(p.observation_nodes, vec![21, 71]);
    // values below the old bottom are those of the bottom node
    p.regrid(&[0.0, 50.0, 100.0, 120.0]);
    assert!((p.nodes[3].h - p.nodes[2].h).abs() < 1e-12);
}

#[test]
fn materials_are_added_and_removed_with_their_parameters() {
    let mut prj = examples::infiltration_evaporation();
    let nm = prj.water.materials.len();
    assert!(nm >= 2);
    let optm_len = |p: &Project| match &p.root.stress {
        RootStress::Feddes { p_optm, .. } => p_optm.len(),
        _ => 0,
    };
    let n_optm = optm_len(&prj);
    let k = prj.add_material_copy(0);
    assert_eq!(k, nm);
    assert_eq!(prj.water.materials.len(), nm + 1);
    assert_eq!(prj.water.materials[k].ks, prj.water.materials[0].ks);
    assert_eq!(optm_len(&prj), if n_optm > 0 { n_optm + 1 } else { 0 });
    // nodes of material 2 move to material 1, material 3 becomes 2
    prj.profile.nodes.last_mut().unwrap().mat = 3;
    let before: Vec<usize> = prj.profile.nodes.iter().map(|n| n.mat).collect();
    prj.remove_material(1);
    assert_eq!(prj.water.materials.len(), nm);
    assert_eq!(optm_len(&prj), n_optm);
    for (b, n) in before.iter().zip(&prj.profile.nodes) {
        let want = match b {
            1 | 2 => 1,
            3 => 2,
            x => *x,
        };
        assert_eq!(n.mat, want);
    }
    // the last material cannot be removed
    let mut one = Project::default();
    one.remove_material(0);
    assert_eq!(one.water.materials.len(), 1);
}
