use alloc::{format, string::ToString};

use crate::{
    file::create_file,
    syscall::{ProcessState, stdout},
};
use core::ffi::{CStr, c_char};

use crate::{
    file::traverse_path, process::CURRENT_PROCESS, syscall::io::Error, traps::TrapFrame,
    vm::translate_virtual_address,
};

pub fn chdir(trapframe: &TrapFrame) -> usize {
    let path = unsafe {
        CStr::from_ptr(
            translate_virtual_address(trapframe.page_table, trapframe.a0).unwrap() as *const c_char,
        )
        .to_str()
        .unwrap()
    };

    let current_process = unsafe { &mut **CURRENT_PROCESS.as_mut().unwrap() };

    let inode = match traverse_path(path, false) {
        Ok(v) => v,
        Err(e) if matches!(e, crate::error::Error::NoSuchEntryInDirectory { name: _ }) => {
            return -Error::ENOTDIR as usize;
        }
        Err(e) if matches!(e, crate::error::Error::FileDoesNotExist { path: _ }) => {
            return -Error::ENOENT as usize;
        }
        Err(e) => panic!("IN CHDIR: {}", e),
    };

    if let ProcessState::Running { cwd: _ } = &current_process.state {
        current_process.state = ProcessState::Running { cwd: inode }
    } else if let ProcessState::Ready { cwd: _ } = &current_process.state {
        current_process.state = ProcessState::Ready { cwd: inode }
    }

    0
}

pub fn mkdir(trapframe: &TrapFrame) -> usize {
    let path = unsafe {
        CStr::from_ptr(
            translate_virtual_address(trapframe.page_table, trapframe.a0).unwrap() as *const c_char,
        )
        .to_str()
        .unwrap()
        .to_string()
    };

    match create_file(&path, crate::fs::sfs::InodeEntry::Directory) {
        Ok(_) => 0,
        Err(e) if matches!(e, crate::error::Error::FileAlreadyExists { path: _ }) => {
            -Error::EEXIST as usize
        }
        Err(e) if matches!(e, crate::error::Error::NotADirectory { name: _ }) => {
            -Error::ENOTDIR as usize
        }
        Err(e) if matches!(e, crate::error::Error::NoSuchEntryInDirectory { name: _ }) => {
            -Error::ENOENT as usize
        }
        Err(e) => panic!("IN MKDIR {}\n", e),
    }
}
