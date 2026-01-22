use alloc::{format, string::ToString, vec};
use core::{
    ffi::{CStr, c_char, c_str},
    ptr::{self},
};

use alloc::vec::Vec;
use elf::{ElfBytes, endian::NativeEndian};

use crate::{
    DEVICE,
    constants::{EXECUTE_ONLY, PAGE_SIZE, READ_ONLY, Sv48, WRITE_ONLY},
    error::Error,
    file::{self, FILES, FileType, exists},
    fs::sfs::{flush_data_blocks, flush_inodes, read_inode_data},
    global_state::GlobalState,
    process::{self, ProcessState, map_code_pages, map_other_pages},
    scheduler::switch_to_scheduler_context,
    syscall::{self, stdout},
    traps::TrapFrame,
    vm::{SUPERVISOR, translate_virtual_address},
};

pub fn exit(state: &GlobalState, trapframe: &TrapFrame) -> usize {
    let return_value = trapframe.a0;
    let process = state.get_current_process().unwrap();
    let mut current_process = process.lock();

    current_process.process_state = ProcessState::Terminated {
        return_value: Ok(return_value as isize),
    };

    for fd in &current_process.fds {
        match unsafe { FILES.remove(&fd) } {
            Some(file) => match &*file.file_type.borrow() {
                FileType::INode {
                    inode,
                    offset: _,
                    append: _,
                } => {
                    flush_data_blocks(&DEVICE, true);
                    flush_inodes(&DEVICE).unwrap();

                    if inode.links.get() == 0 {
                        todo!();
                    }
                }
                _ => (),
            },
            None => {}
        }
    }

    let id = current_process.id;
    drop(current_process);
    process::wake_up(state, id);

    switch_to_scheduler_context(state);
    0
}

// pub fn fork(_: &TrapFrame) -> usize {
//     enum Error {
//         ENOMEM = 12,
//         EAGAIN = 11,
//     }

//     let current_process = unsafe { &mut **CURRENT_PROCESS.as_mut().unwrap() };
//     let child = current_process.clone();
//     let current_process = unsafe { &mut **CURRENT_PROCESS.as_mut().unwrap() };

//     match child {
//         Ok(child) => {
//             if let Some(children) = current_process.children.as_mut() {
//                 children.push(child);
//             } else {
//                 current_process.children = Some(vec![child]);
//             }
//             child.id
//         }
//         Err(e)
//             if matches!(
//                 e.downcast_ref().unwrap(),
//                 crate::error::Error::NoUnusedProcess
//             ) =>
//         {
//             -(Error::EAGAIN as isize) as usize
//         }
//         Err(e) if matches!(e.downcast_ref().unwrap(), crate::error::Error::NoFreePage) => {
//             -(Error::ENOMEM as isize) as usize
//         }
//         Err(e) => panic!("FORK: {}", e),
//     }
// }

// pub fn wait(trapframe: &TrapFrame) -> usize {
//     pub const ECHILD: usize = 10;
//     let current_process = unsafe { &mut **CURRENT_PROCESS.as_mut().unwrap() };

//     if let None = current_process.children {
//         return -(ECHILD as isize) as usize;
//     }

//     for child in current_process.children.as_ref().unwrap() {
//         if child.id == trapframe.a0 {
//             current_process.sleep(trapframe.a0);
//             return trapframe.a0;
//         }
//     }

//     return -(ECHILD as isize) as usize;
// }
