use crate::{
    DEVICE,
    error::Error,
    file::{FILES, FileType},
    fs::sfs::{flush_data_blocks, flush_inodes},
    process::{CURRENT_PROCESS, ProcessState, assign_process},
    scheduler::switch_to_scheduler_context,
    syscall,
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

    switch_to_scheduler_context();
    0
}

pub fn fork(trapframe: &TrapFrame) -> usize {
    enum Error {
        ENOMEM = 12,
        EAGAIN = 11,
    }

    let current_process = unsafe { &mut **CURRENT_PROCESS.as_mut().unwrap() };

    match current_process.clone() {
        Ok(child) => child.id,
        Err(e) if e == crate::error::Error::NoUnusedProcess => -(Error::EAGAIN as isize) as usize,
        Err(e) if e == crate::error::Error::NoFreePage => -(Error::ENOMEM as isize) as usize,
        Err(e) => panic!("FORK: {}", e),
    }
}
