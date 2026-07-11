// A5
// SPDX-License-Identifier: Apache-2.0
// Copyright (c) A5 contributors

// The A5 curve's L-system grammar: the motif rules and the string operations that
// expand them. This file is the source of truth for the curve's definition and is
// purely symbolic — the turtle geometry that interprets the symbols lives in
// turtle.rs, and the compilation to descent tables in tables.rs.
//
// 7 self-referential motifs over the alphabet {A B C M P Q R} (+ their lowercase
// reverses) + the draw terminals {E e S s U u D d T t} + the 60° turns +/-.
// A LOWERCASE motif is its uppercase counterpart REVERSED, generated automatically
// by `reverse_motif` — so only the 7 uppercase rules below need to be authored.

use std::collections::HashMap;

/// Each motif's production rule (the 7 authored motifs).
pub fn rules() -> HashMap<char, String> {
    [
        ('A', "PQAB"),
        ('B', "B+++PQ---A"),
        ('C', "P---RMb+++"),
        ('M', "qQ+++C---b"),
        ('P', "PpB---B+++"),
        ('Q', "PQ---Cb+++"),
        ('R', "b+++a---qQ"),
    ]
    .iter()
    .map(|(k, v)| (*k, v.to_string()))
    .collect()
}

/// Each motif's leaf draw symbol — the terminal it renders as at the base case.
pub fn draws() -> HashMap<char, String> {
    [
        ('A', "E"),
        ('B', "+e-"),
        ('C', "-e+"),
        ('M', "T"),
        ('P', "S"),
        ('Q', "D"),
        ('R', "+++D---"),
    ]
    .iter()
    .map(|(k, v)| (*k, v.to_string()))
    .collect()
}

fn swap_case(c: char) -> char {
    if c.is_ascii_lowercase() {
        c.to_ascii_uppercase()
    } else {
        c.to_ascii_lowercase()
    }
}

/// The reverse of a motif/draw string — traced end to start. Uniform transform:
/// reverse the order, swap the case of every letter (uppercase<->lowercase =
/// forward<->reverse partner), and flip every `+`/`-`. This is how the lowercase
/// motifs are derived from the authored uppercase rules.
pub fn reverse_motif(s: &str) -> String {
    s.chars()
        .rev()
        .map(|c| match c {
            '+' => '-',
            '-' => '+',
            _ => swap_case(c),
        })
        .collect()
}

/// One expansion pass over `str`: replace each symbol using `table` (rules or
/// draws). A lowercase motif whose uppercase is in `table` expands to that rule
/// REVERSED; turns and unknown symbols pass through unchanged.
pub fn expand_once(s: &str, table: &HashMap<char, String>) -> String {
    let mut out = String::new();
    for ch in s.chars() {
        let up = ch.to_ascii_uppercase();
        if let Some(rule) = table.get(&ch) {
            out.push_str(rule);
        } else if ch != up {
            if let Some(rule) = table.get(&up) {
                out.push_str(&reverse_motif(rule));
            } else {
                out.push(ch);
            }
        } else {
            out.push(ch);
        }
    }
    out
}
