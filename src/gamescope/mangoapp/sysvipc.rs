use libc::{c_char, c_void, perror, size_t};
use std::ffi::CString;

pub const IPC_CREAT: i32 = libc::IPC_CREAT;

pub fn ftok(path: &str, proj_id: i32) -> i32 {
    let c_str = CString::new(path).unwrap();
    let c_path: *const c_char = c_str.as_ptr() as *const c_char;

    let ret = unsafe { libc::ftok(c_path, proj_id) };
    if ret == -1 {
        let c_str = CString::new("ftok").unwrap();
        let c_path: *const c_char = c_str.as_ptr() as *const c_char;
        unsafe { perror(c_path) };
    }

    ret
}

pub fn msgget(key: i32, msgflg: i32) -> i32 {
    unsafe { libc::msgget(key, msgflg) }
}

pub fn msgrcv(msqid: i32, msgp: &mut [u8], size: usize, msgtyp: i64, msgflg: i32) -> usize {
    let msgp = msgp.as_ptr() as *mut c_void;
    let msgsz = size as size_t;
    unsafe { libc::msgrcv(msqid, msgp, msgsz, msgtyp, msgflg) as usize }
}
