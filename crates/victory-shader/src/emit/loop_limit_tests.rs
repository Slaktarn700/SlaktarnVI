//! The iteration limit of a loop the console jumps back to without a test (`s_branch`):
//! kept at the loop's exit branch ([`super::lower::limit_exits`]), so a loop that ends
//! counts exactly as on the console, and one that never meets its exit leaves through that
//! exit after the limit (here with the sum of its first 4,096 counts) instead of running on.

use std::collections::HashSet;

use super::host::{Dispatch, Host, Root, View};
use super::wave64_tests::{lowered, native_of};
use crate::ir;

/// A wave32 program of 32 threads: each sums the counts `0..` until the scalar count
/// reaches `end`, then stores its sum. The loop's test is at its top; its back edge is an
/// `s_branch`, as the console's compiler lays out a rotated loop.
fn counting_loop(end: u32) -> Vec<u32> {
    let mut words = vec![
        0x7E02_0280, // v_mov_b32 v1, 0
        0xBE90_0380, // s_mov_b32 s16, 0
    ];
    // LOOP: s_cmp_ge_u32 s16, end (an inline constant up to 64, else a literal)
    let literal = end > 64;
    if literal {
        words.extend([0xBF09_FF10, end]);
    } else {
        words.push(0xBF09_0010 | (128 + end) << 8);
    }
    let back = -(4 + i32::from(literal)) - 1; // to LOOP, from after the s_branch
    words.extend([
        0xBF85_0003,                          // s_cbranch_scc1 EXIT
        0x4A02_0210,                          // v_add_nc_u32 v1, s16, v1
        0x8110_8110,                          // s_add_i32 s16, s16, 1
        0xBF82_0000 | (back as u32 & 0xFFFF), // s_branch LOOP
        0x3404_0082,                          // EXIT: v_lshlrev_b32 v2, 2, v0
        0x38 << 26 | 0x1C << 18 | 1 << 12,
        0x80 << 24 | 2 << 16 | 1 << 8 | 2, // buffer_store_dword v1, v2, s[8:11], 0 offen
    ]);
    words
}

/// Each of the 32 threads' stored value, or `None` without a hardware device.
fn run(end: u32) -> Option<Vec<u32>> {
    let native = native_of(&counting_loop(end), false, 32);
    let p = ir::build(&native.code, native.wave64, native.vgprs).expect("ir::build");
    assert!(
        p.blocks.iter().flat_map(|b| &b.insts).any(|i| i.name() == "s_branch"),
        "the back edge is a jump"
    );
    let shader = lowered(&native, &p, &HashSet::new()).expect("lower");
    let _one = super::valu_tests::ONE_AT_A_TIME.lock().unwrap_or_else(|e| e.into_inner());
    let Ok(host) = Host::new(false) else {
        eprintln!("no SM 6.6 hardware device; not run");
        return None;
    };
    let (root, pso) = host.compute_pipeline(&shader).expect("pipeline");
    let ud = vec![0u32; shader.user_data_dwords().max(1) as usize];
    let out = host
        .dispatch(
            &root,
            &pso,
            &Dispatch {
                roots: vec![(0, Root::Constants(ud))],
                views: &[View::RawUav(32)],
                samplers: &[],
                groups: [1, 1, 1],
            },
        )
        .unwrap_or_else(|e| panic!("dispatch failed: {e}"));
    Some(out[0][..32].to_vec())
}

#[test]
fn a_loop_with_a_jump_back_counts_as_on_the_console() {
    let Some(out) = run(10) else { return };
    assert_eq!(out, vec![45; 32], "0 + 1 + ... + 9 in every thread");
}

#[test]
fn a_loop_that_never_meets_its_exit_leaves_it_after_the_limit() {
    // A compute loop's limit is 4,096 iterations (`LOOP_LIMIT`): the lane leaves through the
    // loop's exit with the sum of the counts it ran, and stores it.
    let Some(out) = run(100_000) else { return };
    assert_eq!(out, vec![4096 * 4095 / 2; 32]);
}
