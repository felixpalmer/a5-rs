use a5::traversal::triple_cells::{triple_children, triple_parent};
use serde::Deserialize;
use std::fs;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Fixture {
    hilbert_res: u32,
    triple: [i32; 3],
    children: Vec<[i32; 3]>,
}

#[derive(Deserialize)]
struct Fixtures {
    cases: Vec<Fixture>,
}

fn load() -> Fixtures {
    let content = fs::read_to_string("tests/fixtures/traversal/triple-hierarchy.json")
        .expect("Could not read triple-hierarchy.json");
    serde_json::from_str(&content).expect("Could not parse triple-hierarchy.json")
}

#[test]
fn test_triple_children() {
    for f in load().cases {
        let [x, y, z] = f.triple;
        let mut out: Vec<[i32; 5]> = Vec::new();
        triple_children([0, 0, x, y, z], (1i32 << f.hilbert_res) - 1, &mut out);
        let mut children: Vec<[i32; 3]> = out.iter().map(|c| [c[2], c[3], c[4]]).collect();
        children.sort();
        assert_eq!(children, f.children, "{:?} @ {}", f.triple, f.hilbert_res);
    }
}

#[test]
fn test_triple_parent() {
    for f in load().cases {
        for [x, y, z] in &f.children {
            let p = triple_parent([0, 0, *x, *y, *z], (1i32 << f.hilbert_res) - 1);
            assert_eq!(
                [p[2], p[3], p[4]],
                f.triple,
                "{:?} @ {}",
                [x, y, z],
                f.hilbert_res + 1
            );
        }
    }
}
