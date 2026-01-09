use crate::syscall::ProcessState;
use core::ffi::CStr;

use crate::{
    file::{exists, traverse_path},
    process::CURRENT_PROCESS,
    syscall::io::Error,
    traps::TrapFrame,
    vm::translate_virtual_address,
};

pub fn chdir(trapframe: &TrapFrame) -> usize {
    let path = unsafe {
        CStr::from_ptr(
            translate_virtual_address(trapframe.page_table, trapframe.a0).unwrap() as *const u8,
        )
        .to_str()
        .unwrap()
    };

    let current_process = unsafe { &mut **CURRENT_PROCESS.as_mut().unwrap() };

    if !exists(path).unwrap() {
        return -Error::ENOENT as usize;
    }

    let inode = match traverse_path(path, false) {
        Ok(v) => v,
        Err(e) if let crate::error::Error::NoSuchEntryInDirectory { name: _ } = e => {
            return -Error::ENOTDIR as usize;
        }
        Err(e) if let crate::error::Error::FileDoesNotExist { path: _ } = e => {
            return -Error::ENOENT as usize;
        }
        Err(e) => panic!("IN CHDIR: {}", e),
    };

    if let ProcessState::Running { cwd: _ } = &current_process.state {
        current_process.state = ProcessState::Running { cwd: inode }
    }

    0
}
