use alloc::vec;

use crate::{
    DEVICE,
    file::{FILES, FileType},
    fs::sfs::{flush_data_blocks, flush_inodes},
    process::{self, CURRENT_PROCESS, ProcessState},
    scheduler::switch_to_scheduler_context,
    traps::TrapFrame,
};

pub fn exit(trapframe: &TrapFrame) -> usize {
    let return_value = trapframe.a0;
    let current_process = unsafe { &mut **CURRENT_PROCESS.as_mut().unwrap() };

    current_process.state = ProcessState::Terminated {
        return_value: return_value as isize,
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

    process::wake_up(current_process.id);

    switch_to_scheduler_context();
    0
}

pub fn fork(trapframe: &TrapFrame) -> usize {
    enum Error {
        ENOMEM = 12,
        EAGAIN = 11,
    }

    let current_process = unsafe { &mut **CURRENT_PROCESS.as_mut().unwrap() };
    let child = current_process.clone();
    let current_process = unsafe { &mut **CURRENT_PROCESS.as_mut().unwrap() };

    match child {
        Ok(child) => {
            if let Some(children) = current_process.children.as_mut() {
                children.push(child);
            } else {
                current_process.children = Some(vec![child]);
            }
            child.id
        }
        Err(e) if e == crate::error::Error::NoUnusedProcess => -(Error::EAGAIN as isize) as usize,
        Err(e) if e == crate::error::Error::NoFreePage => -(Error::ENOMEM as isize) as usize,
        Err(e) => panic!("FORK: {}", e),
    }
}

pub fn wait(trapframe: &TrapFrame) -> usize {
    pub const ECHILD: usize = 10;
    let current_process = unsafe { &mut **CURRENT_PROCESS.as_mut().unwrap() };

    if let None = current_process.children {
        return -(ECHILD as isize) as usize;
    }

    for child in current_process.children.as_ref().unwrap() {
        if child.id == trapframe.a0 {
            current_process.sleep(trapframe.a0);
            return trapframe.a0;
        }
    }

    return -(ECHILD as isize) as usize;
}
