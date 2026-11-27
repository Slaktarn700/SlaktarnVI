//! A call whose access violations fail it instead of ending the process.
//!
//! The stack dump walks other threads' stacks with the system's unwinder
//! (`RtlLookupFunctionEntry`, `RtlVirtualUnwind`). Under Wine a suspended
//! thread's frames can lead the unwinder to read through a garbage pointer
//! (a thread inside one of Wine's system calls), and Wine's unwinder has no
//! guard: twice the first scheduled dump of a run ended the process inside
//! `RtlVirtualUnwind` (2026-11-26). [`guarded`] runs such a step so that an
//! access violation on this thread while it runs comes back as `false`.
//!
//! How: [`victory_guarded_call`] keeps every register the callee must give
//! back (rbx, rbp, rdi, rsi, r12-r15, xmm6-xmm15) on its own frame and the
//! frame's stack pointer in [`VICTORY_GUARD_RSP`], then calls. A vectored
//! handler, first in the process and acting only for the thread that is in a
//! guarded call, sends a fault to [`victory_guarded_call_failed`], which
//! takes that stack pointer back, restores the registers and returns 0. What
//! the callee had begun is left as it was: only calls that change nothing of
//! ours on the way (the unwind of a copy of a context) belong here.

use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};

use windows_sys::Win32::Foundation::{EXCEPTION_ACCESS_VIOLATION, NTSTATUS};
use windows_sys::Win32::System::Diagnostics::Debug::{AddVectoredExceptionHandler, EXCEPTION_POINTERS};
use windows_sys::Win32::System::Threading::GetCurrentThreadId;

/// The stack pointer of the guarded call's frame (one guarded call at a time:
/// [`GUARDED_THREAD`] says whose).
#[unsafe(no_mangle)]
static VICTORY_GUARD_RSP: AtomicU64 = AtomicU64::new(0);

/// The thread in a guarded call, 0 if none.
static GUARDED_THREAD: AtomicU32 = AtomicU32::new(0);

/// Faults taken by guarded calls since the start.
pub(crate) static GUARD_FAULTS: AtomicU64 = AtomicU64::new(0);

std::arch::global_asm!(
    ".globl victory_guarded_call",
    ".globl victory_guarded_call_failed",
    // rcx the function, rdx its argument; eax 1 when it returned, 0 when it
    // faulted. The frame: eight pushes, then 200 bytes (32 of shadow space,
    // 160 for xmm6-xmm15, 8 to keep the stack 16-byte aligned).
    "victory_guarded_call:",
    "push rbx",
    "push rbp",
    "push rsi",
    "push rdi",
    "push r12",
    "push r13",
    "push r14",
    "push r15",
    "sub rsp, 200",
    "movaps xmmword ptr [rsp + 32], xmm6",
    "movaps xmmword ptr [rsp + 48], xmm7",
    "movaps xmmword ptr [rsp + 64], xmm8",
    "movaps xmmword ptr [rsp + 80], xmm9",
    "movaps xmmword ptr [rsp + 96], xmm10",
    "movaps xmmword ptr [rsp + 112], xmm11",
    "movaps xmmword ptr [rsp + 128], xmm12",
    "movaps xmmword ptr [rsp + 144], xmm13",
    "movaps xmmword ptr [rsp + 160], xmm14",
    "movaps xmmword ptr [rsp + 176], xmm15",
    "mov qword ptr [rip + {rsp}], rsp",
    "mov rax, rcx",
    "mov rcx, rdx",
    "call rax",
    "mov eax, 1",
    "jmp 2f",
    "victory_guarded_call_failed:",
    "mov rsp, qword ptr [rip + {rsp}]",
    "xor eax, eax",
    "2:",
    "movaps xmm6, xmmword ptr [rsp + 32]",
    "movaps xmm7, xmmword ptr [rsp + 48]",
    "movaps xmm8, xmmword ptr [rsp + 64]",
    "movaps xmm9, xmmword ptr [rsp + 80]",
    "movaps xmm10, xmmword ptr [rsp + 96]",
    "movaps xmm11, xmmword ptr [rsp + 112]",
    "movaps xmm12, xmmword ptr [rsp + 128]",
    "movaps xmm13, xmmword ptr [rsp + 144]",
    "movaps xmm14, xmmword ptr [rsp + 160]",
    "movaps xmm15, xmmword ptr [rsp + 176]",
    "add rsp, 200",
    "pop r15",
    "pop r14",
    "pop r13",
    "pop r12",
    "pop rdi",
    "pop rsi",
    "pop rbp",
    "pop rbx",
    "ret",
    rsp = sym VICTORY_GUARD_RSP,
);

unsafe extern "system" {
    fn victory_guarded_call(f: unsafe extern "system" fn(*mut core::ffi::c_void), arg: *mut core::ffi::c_void) -> u32;
    fn victory_guarded_call_failed();
}

/// First in the process: a fault of the thread that is in a guarded call
/// goes to the call's failing exit.
unsafe extern "system" fn handler(info: *mut EXCEPTION_POINTERS) -> i32 {
    const CONTINUE_EXECUTION: i32 = -1;
    const CONTINUE_SEARCH: i32 = 0;
    // SAFETY: a plain query.
    let me = unsafe { GetCurrentThreadId() };
    if me == 0 || GUARDED_THREAD.load(Ordering::Acquire) != me {
        return CONTINUE_SEARCH;
    }
    // SAFETY: the system passes the exception being dispatched.
    let (record, context) = unsafe { (&*(*info).ExceptionRecord, &mut *(*info).ContextRecord) };
    if record.ExceptionCode != EXCEPTION_ACCESS_VIOLATION as NTSTATUS {
        return CONTINUE_SEARCH;
    }
    GUARD_FAULTS.fetch_add(1, Ordering::Relaxed);
    context.Rip = victory_guarded_call_failed as *const () as usize as u64;
    CONTINUE_EXECUTION
}

fn install() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    // SAFETY: the handler only reads the exception and writes its context.
    ONCE.call_once(|| unsafe {
        AddVectoredExceptionHandler(1, Some(handler));
    });
}

/// Runs `f`; `false` if it ended in an access violation (then what it had
/// begun stays as it was). One guarded call at a time in the process.
pub(crate) fn guarded(f: &mut dyn FnMut()) -> bool {
    unsafe extern "system" fn call(arg: *mut core::ffi::c_void) {
        // SAFETY: `arg` is the `&mut dyn FnMut()` below, alive for the call.
        let f = unsafe { &mut *arg.cast::<&mut dyn FnMut()>() };
        f();
    }
    install();
    // SAFETY: a plain query.
    let me = unsafe { GetCurrentThreadId() };
    if GUARDED_THREAD.compare_exchange(0, me, Ordering::AcqRel, Ordering::Acquire).is_err() {
        return false;
    }
    let mut f = f;
    // SAFETY: the trampoline calls `call` with a pointer to `f`, which lives
    // across the call; a fault comes back as 0 with our registers restored.
    let done = unsafe { victory_guarded_call(call, (&raw mut f).cast()) } != 0;
    GUARDED_THREAD.store(0, Ordering::Release);
    done
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A read of address 0x10 inside a guarded call fails the call; the
    /// thread goes on with its registers as they were, and later calls work.
    #[test]
    fn a_fault_in_a_guarded_call_fails_it() {
        let before = GUARD_FAULTS.load(Ordering::Relaxed);
        let mut ran = 0u64;
        assert!(guarded(&mut || ran += 1));
        assert_eq!(ran, 1);
        let kept = std::hint::black_box(0x1234_5678_u64);
        let failed = guarded(&mut || {
            ran += 10;
            // SAFETY (deliberately not): a read of an unmapped address, the
            // fault the guard is for.
            unsafe {
                std::arch::asm!("mov {x}, qword ptr [{p}]", p = in(reg) 0x10usize, x = out(reg) _);
            }
            ran += 100;
        });
        assert!(!failed);
        assert_eq!(ran, 11);
        assert_eq!(std::hint::black_box(kept), 0x1234_5678);
        assert_eq!(GUARD_FAULTS.load(Ordering::Relaxed), before + 1);
        // Floating point registers the call kept are as they were.
        let x = std::hint::black_box(1.5f64) * std::hint::black_box(2.0);
        assert!(guarded(&mut || ran += 1000));
        assert_eq!(ran, 1011);
        assert_eq!(x, 3.0);
    }
}
