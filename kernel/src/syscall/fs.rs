use alloc::{format, string::ToString};

use crate::{
    file::create_file,
    global_state::GlobalState,
    syscall::{ProcessState, stdout},
};
use core::ffi::{CStr, c_char};

use crate::{
    file::traverse_path, syscall::io::Error, traps::TrapFrame, vm::translate_virtual_address,
};

pub fn chdir(state: &GlobalState, trapframe: &TrapFrame) -> usize {
    let process = state.get_current_process().unwrap();
    let mut current_process = process.lock();

    let path = unsafe {
        CStr::from_ptr(
            translate_virtual_address(state, trapframe.page_table, trapframe.a0).unwrap()
                as *const c_char,
        )
        .to_str()
        .unwrap()
    };

    let inode = match traverse_path(state, path, false) {
        Ok(v) => v,
        Err(e)
            if matches!(
                e.downcast_ref().unwrap(),
                crate::error::Error::NoSuchEntryInDirectory { name: _ }
            ) =>
        {
            return -Error::ENOTDIR as usize;
        }
        Err(e)
            if matches!(
                e.downcast_ref().unwrap(),
                crate::error::Error::FileDoesNotExist { path: _ }
            ) =>
        {
            return -Error::ENOENT as usize;
        }
        Err(e) => panic!("IN CHDIR: {}", e),
    };

    if let ProcessState::Running { cwd: _ } = &current_process.process_state {
        current_process.process_state = ProcessState::Running { cwd: inode }
    } else if let ProcessState::Ready { cwd: _ } = &current_process.process_state {
        current_process.process_state = ProcessState::Ready { cwd: inode }
    }

    0
}

pub fn mkdir(state: &GlobalState, trapframe: &TrapFrame) -> usize {
    let process = state.get_current_process().unwrap();
    let current_process = process.lock();

    let state = current_process.global_state;

    let path = unsafe {
        CStr::from_ptr(
            translate_virtual_address(state, trapframe.page_table, trapframe.a0).unwrap()
                as *const c_char,
        )
        .to_str()
        .unwrap()
        .to_string()
    };

    match create_file(state, &path, crate::fs::sfs::InodeEntry::Directory) {
        Ok(_) => 0,
        Err(e)
            if matches!(
                e.downcast_ref().unwrap(),
                crate::error::Error::FileAlreadyExists { path: _ }
            ) =>
        {
            -Error::EEXIST as usize
        }
        Err(e)
            if matches!(
                e.downcast_ref().unwrap(),
                crate::error::Error::NotADirectory { name: _ }
            ) =>
        {
            -Error::ENOTDIR as usize
        }
        Err(e)
            if matches!(
                e.downcast_ref().unwrap(),
                crate::error::Error::NoSuchEntryInDirectory { name: _ }
            ) =>
        {
            -Error::ENOENT as usize
        }
        Err(e) => panic!("IN MKDIR {}\n", e),
    }
}
