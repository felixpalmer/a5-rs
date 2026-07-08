// A5
// SPDX-License-Identifier: Apache-2.0
// Copyright (c) A5 contributors

// The turtle alphabet on the integer (a,b) triangular lattice (basis u=(√3/4,1/4),
// v=(0,1/2)). Draw symbols {E e S s U u D d T t} are unit segments; `+`/`-` are 60°
// turns. Each symbol also carries the 3 corners of the triangular cell it hosts.
// Everything here is exact integer — √3 only enters when (a,b) is later mapped to
// A5 triple coordinates (in the lsystem module).

/// A point on the integer (a,b) triangular lattice.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AB {
    pub a: i32,
    pub b: i32,
}

impl AB {
    pub fn new(a: i32, b: i32) -> Self {
        Self { a, b }
    }
}

pub fn add(p: AB, q: AB) -> AB {
    AB::new(p.a + q.a, p.b + q.b)
}

/// 180° rotation
pub fn neg(p: AB) -> AB {
    AB::new(-p.a, -p.b)
}

/// 60° CCW, order 6
fn rot60(p: AB) -> AB {
    AB::new(-p.b, p.a + p.b)
}

pub fn rot_times(p: AB, n: i32) -> AB {
    let mut r = p;
    let k = n.rem_euclid(6);
    for _ in 0..k {
        r = rot60(r);
    }
    r
}

/// Step vector of each draw symbol at heading 0. Lowercase = same step, cell hosted
/// on the other side (see host_offsets).
fn base(sym: char) -> Option<AB> {
    let ab = match sym {
        'E' | 'e' => AB::new(4, 0),
        'S' | 's' => AB::new(4, -2),
        'U' | 'u' => AB::new(0, 2),
        'D' | 'd' => AB::new(0, -2),
        'T' | 't' => AB::new(-4, 0),
        _ => return None,
    };
    Some(ab)
}

/// Whether a symbol is a draw terminal.
pub fn is_draw(sym: char) -> bool {
    base(sym).is_some()
}

/// The 3 corner offsets (heading 0, from the segment start) of the cell each symbol hosts.
fn host_offsets(sym: char) -> [AB; 3] {
    match sym {
        'E' => [AB::new(0, 0), AB::new(4, 0), AB::new(4, -4)],
        'e' => [AB::new(0, 0), AB::new(4, 0), AB::new(0, 4)],
        'S' => [AB::new(0, 0), AB::new(4, 0), AB::new(4, -4)],
        's' => [AB::new(4, -2), AB::new(0, 2), AB::new(0, -2)],
        'U' => [AB::new(0, 2), AB::new(0, -2), AB::new(4, -2)],
        'u' => [AB::new(0, 0), AB::new(0, 4), AB::new(-4, 4)],
        'D' => [AB::new(0, 2), AB::new(0, -2), AB::new(4, -2)],
        'd' => [AB::new(0, 0), AB::new(0, -4), AB::new(-4, 0)],
        'T' => [AB::new(0, -4), AB::new(-4, 0), AB::new(-4, -4)],
        't' => [AB::new(-4, 4), AB::new(0, 0), AB::new(0, 4)],
        _ => panic!("lsystem: no host offsets for symbol {}", sym),
    }
}

/// The 3 (a,b) corners of the cell hosted by `sym`, drawn from `from` at `heading`.
pub fn host_corners(sym: char, from: AB, heading: i32) -> [AB; 3] {
    let o = host_offsets(sym);
    [
        add(from, rot_times(o[0], heading)),
        add(from, rot_times(o[1], heading)),
        add(from, rot_times(o[2], heading)),
    ]
}

/// The corner SUM (= 3·centroid, an exact integer) of that cell.
pub fn host_sum(sym: char, from: AB, heading: i32) -> AB {
    let [p, q, r] = host_corners(sym, from, heading);
    AB::new(p.a + q.a + r.a, p.b + q.b + r.b)
}

/// Final turtle state after walking a draw string.
pub struct TurtleState {
    pub pos: AB,
    pub heading: i32,
}

/// Walk a draw string (draw symbols + `+`/`-` turns) from (pos, heading). Calls
/// `on_draw(sym, from, heading)` for each draw symbol (before advancing). Returns the
/// final turtle state.
pub fn walk<F: FnMut(char, AB, i32)>(
    s: &str,
    pos: AB,
    heading: i32,
    mut on_draw: F,
) -> TurtleState {
    let mut p = pos;
    let mut h = heading.rem_euclid(6);
    for ch in s.chars() {
        if ch == '+' {
            h = (h + 1) % 6;
            continue;
        }
        if ch == '-' {
            h = (h + 5) % 6;
            continue;
        }
        let step = match base(ch) {
            Some(b) => b,
            None => continue,
        };
        on_draw(ch, p, h);
        p = add(p, rot_times(step, h));
    }
    TurtleState { pos: p, heading: h }
}

/// Net (a,b) displacement + net heading of a draw string, from origin at heading 0.
pub fn net_of(s: &str) -> (AB, i32) {
    let end = walk(s, AB::new(0, 0), 0, |_, _, _| {});
    (end.pos, end.heading)
}
