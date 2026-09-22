//! Run in a disposable process: loaded DLLs receive THREAD_ATTACH on a 64KiB stack.
//! Build: rustc dll_thread_smoke.rs -o target/dll-thread-smoke.exe
use std::{ffi::c_void, os::windows::ffi::OsStrExt};
#[link(name = "kernel32")]
unsafe extern "system" {
    fn LoadLibraryW(path: *const u16) -> *mut c_void;
    fn CreateThread(attributes: *const c_void, stack_size: usize,
        start: unsafe extern "system" fn(*mut c_void) -> u32, parameter: *mut c_void,
        flags: u32, id: *mut u32) -> *mut c_void;
    fn WaitForSingleObject(handle: *mut c_void, timeout: u32) -> u32;
    fn GetExitCodeThread(handle: *mut c_void, code: *mut u32) -> i32;
    fn CloseHandle(handle: *mut c_void) -> i32;
}
unsafe extern "system" fn worker(_: *mut c_void) -> u32 { 42 }
fn main() {
    let path = std::env::args_os().nth(1).expect("DLL path required");
    let wide: Vec<_> = path.encode_wide().chain(Some(0)).collect();
    unsafe {
        assert!(!LoadLibraryW(wide.as_ptr()).is_null(), "DLL load failed");
        let thread = CreateThread(std::ptr::null(), 65536, worker,
            std::ptr::null_mut(), 0x10000, std::ptr::null_mut());
        assert!(!thread.is_null(), "thread creation failed");
        assert_eq!(WaitForSingleObject(thread, 10000), 0, "thread timed out");
        let mut code = 0;
        assert_ne!(GetExitCodeThread(thread, &mut code), 0);
        assert_eq!(code, 42);
        CloseHandle(thread);
    }
    // No hot unload: the DLL can still own its initialization worker.
    println!("PASS: DLL thread attach/detach on 64KiB reserved stack");
}
