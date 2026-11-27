//! The thread-group info SGPR of compute programs (`TG_SIZE_EN`), run on the
//! host: each thread stores what it holds.

use super::host::{Dispatch, Host, Root, View};
use super::{HostShader, lower};
use crate::native::{
    ComputeIo, HwStage, NativeProgram, ResourceType, RootParam, RootType, StageIo, UsedResource,
};
use crate::{io, ir, resource, structure};

const THREADS: u32 = 128;

/// Five user SGPRs (four constants, a UAV table at s4), then the
/// thread-group info in s5 (`TG_SIZE_EN`, bit 10 of `COMPUTE_PGM_RSRC2`).
fn native(wave64: bool) -> NativeProgram {
    let mov = |dst: u32, src: u32| 0x7E00_0200 | dst << 17 | src;
    let smov = |dst: u32, src: u32| 0xBE80_0300 | dst << 16 | src;
    let words = [
        smov(12, 5), // s_mov_b32 s12, s5 (the info, before s5 is the address's top)
        smov(5, 128),
        0x3D << 26 | 0x02 << 18 | 8 << 6 | 2,
        125 << 25,   // s_load_dwordx4 s[8:11], s[4:5], 0
        0x340E_0082, // v_lshlrev_b32 v7, 2, v0 (output byte offset)
        mov(6, 12),  // v_mov_b32 v6, s12
        0xBF8C_0000, // s_waitcnt 0
        0x38 << 26 | 0x1C << 18 | 1 << 12,
        0x80 << 24 | 2 << 16 | 6 << 8 | 7, // buffer_store_dword v6, v7, s[8:11], 0 offen
        0xBF81_0000,
    ];
    NativeProgram {
        stage: HwStage::Cs,
        wave64,
        vgprs: 8,
        layout: vec![
            RootParam { root: RootType::Constants, table_offset_dwords: 0, sgprs: 4, base_register: 0, user_data: 0, slot: 0 },
            RootParam { root: RootType::Table, table_offset_dwords: 0, sgprs: 1, base_register: 0, user_data: 4, slot: 1 },
        ],
        used: vec![UsedResource {
            kind: ResourceType::Uav,
            root: RootType::Table,
            table_offset_dwords: 0,
            lower_bound: 0,
            stride: 0,
            dwords: 4,
            user_data: 4,
            slot: 1,
        }],
        io: StageIo::Cs(ComputeIo { rsrc2: 5 << 1 | 1 << 10, threads: [THREADS, 1, 1] }),
        code: words.iter().flat_map(|w| w.to_le_bytes()).collect(),
    }
}

fn lowered(native: &NativeProgram, host_wave: u32) -> HostShader {
    let p = ir::build(&native.code, native.wave64, native.vgprs).expect("ir");
    let st = structure::structure(&p).expect("structure");
    let res = resource::track(&p, &native.layout, native.stage, Some(&st)).expect("resources");
    let io = io::raise(&p, native).expect("io");
    let was = lower::TEST_HOST_WAVE.replace(Some(host_wave));
    let result = lower::lower(&lower::Parts {
        native,
        p: &p,
        st: &st,
        io: &io,
        res: &res,
        inputs: Vec::new(),
        outputs: Vec::new(),
        prims: Vec::new(),
        mesh_stride: 0,
    });
    lower::TEST_HOST_WAVE.set(was);
    let result = result.expect("the thread-group info lowers");
    HostShader {
        stage: HwStage::Cs,
        dxil: result.dxil,
        user_data: result.user_data,
        samplers: result.samplers,
        static_samplers: Vec::new(),
        root_cbvs: result.root_cbvs,
        root_srvs: result.root_srvs,
        root_uavs: result.root_uavs,
        root_indexed_views: res.root_indexed_views,
        half_views: result.half_views.clone(),
    }
}

/// Each thread holds its console wave's index in bits 6-11 and the group's
/// wave count in bits 0-5, for console waves of 32 and 64.
#[test]
fn the_thread_group_info_holds_the_wave_index_and_count() {
    let _one = super::valu_tests::ONE_AT_A_TIME.lock().unwrap_or_else(|e| e.into_inner());
    let mut ran = 0;
    for (backend, warp) in [("hardware", false), ("WARP", true)] {
        let Ok(host) = Host::new(warp) else {
            eprintln!("no SM 6.6 {backend}; skipped");
            continue;
        };
        for wave64 in [false, true] {
            let wave = if wave64 { 64 } else { 32 };
            let shader = lowered(&native(wave64), wave);
            let Ok((root, pso)) = host.compute_pipeline(&shader) else {
                eprintln!("{backend}: no pipeline for console waves of {wave}; skipped");
                continue;
            };
            let ud = vec![0; shader.user_data_dwords() as usize];
            let out = host
                .dispatch(
                    &root,
                    &pso,
                    &Dispatch { roots: vec![(0, Root::Constants(ud))], views: &[View::RawUav(THREADS)], samplers: &[], groups: [1, 1, 1] },
                )
                .expect("dispatch");
            for t in 0..THREADS {
                let want = ((t / wave) << 6) | (THREADS / wave);
                assert_eq!(out[0][t as usize], want, "{backend} wave64={wave64} thread {t}");
            }
            ran += 1;
        }
    }
    eprintln!("ran on {ran} backend and wave combinations");
}
