use alloc::{borrow::ToOwned, format, string::ToString, vec};
use core::{
    f64::math::ceil,
    ffi::{CStr, c_char, c_str},
    ptr, str,
};

use alloc::vec::Vec;
use elf::{ElfBytes, endian::NativeEndian};

use crate::{
    DEVICE,
    allocator::allocate,
    constants::{EXECUTE_ONLY, KERNEL_PAGE_TABLE, PAGE_SIZE, READ_ONLY, Sv48, WRITE_ONLY},
    error::Error,
    file::{self, FILES, FileType, exists, open},
    fs::sfs::{MAXIMUM_FILE_SIZE, flush_data_blocks, flush_inodes, read_inode},
    process::{
        self, CURRENT_PROCESS, Process, ProcessState, assign_process, map_code_pages,
        map_other_pages, prepare_first_time_execution,
    },
    scheduler::switch_to_scheduler_context,
    sh_end, sh_start,
    syscall::{self, stdout},
    traps::TrapFrame,
    vm::{SUPERVISOR, translate_virtual_address},
};

pub fn exit(trapframe: &TrapFrame) -> usize {
    let return_value = trapframe.a0;
    let current_process = unsafe { &mut **CURRENT_PROCESS.as_mut().unwrap() };

    current_process.state = ProcessState::Terminated {
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

pub fn execve(trapframe: &TrapFrame) -> usize {
    pub const ENOMEM: isize = 12;

    let process = unsafe { &mut **CURRENT_PROCESS.as_mut().unwrap() };

    assert!(
        trapframe.page_table == process.page_table,
        "TRAPFRAME OR PROCESS CONTROL BLOCK CORRUPT - 1"
    );

    assert!(
        trapframe.satp == Sv48 | (process.page_table >> 12),
        "TRAPFRAME OR PROCESS CONTROL BLOCK CORRUPT - 2"
    );

    unsafe { SUPERVISOR = true }

    let ptr = match translate_virtual_address(trapframe.page_table, trapframe.a0) {
        Ok(v) => v,
        Err(e) => panic!("ERROR WHILE TRANSLATING VA IN EXECVE: {}", e),
    } as *const c_char;

    unsafe { SUPERVISOR = false }

    let path = unsafe { c_str::CStr::from_ptr(ptr).to_str().unwrap() };

    if !exists(path).unwrap() {
        return -syscall::io::Error::ENOENT as usize;
    }

    let fd = match file::open(path, true, false, false, false, false, false) {
        Ok(v) => v,
        Err(e) => panic!("EXECVE: {}", e),
    };

    let mut mock_trapframe = TrapFrame::default();
    mock_trapframe.a0 = fd;

    let file = unsafe { FILES.get(&fd).unwrap() };

    let size: usize;

    if let FileType::INode {
        inode,
        offset: _,
        append: _,
    } = &*file.file_type.borrow()
    {
        size = inode.size.get();
    } else {
        panic!("FILE NOT INODE IN EXECVE");
    }

    let mut buffer = Vec::<u8>::with_capacity(size);
    mock_trapframe.a1 = buffer.as_mut_ptr().addr();
    mock_trapframe.page_table = unsafe { KERNEL_PAGE_TABLE };

    mock_trapframe.a2 = size;

    unsafe { SUPERVISOR = true }

    let read = syscall::io::read(&mock_trapframe);

    unsafe {
        SUPERVISOR = false;
        buffer.set_len(read);
    }

    let elf_data: ElfBytes<NativeEndian> =
        elf::ElfBytes::minimal_parse(buffer.as_slice()).expect("INIT FAILED - ELF ERROR");

    let program_headers = elf_data
        .segments()
        .expect("INIT FAILED - NO SEGMENTS")
        .iter();

    let mut page = 0;
    let mut max_code_page_end_va = 0;

    let old_heap_end = process.trapframe.as_ref().unwrap().heap_end.get();

    // if trapframe.a1 != 0 {
    //     let argv_ptr = translate_virtual_address(trapframe.page_table, trapframe.a1).unwrap()
    //         as *const *const c_char;

    //     // let mut argv = Vec::new();

    //     let mut i = 0;

    // loop {
    //     if unsafe { argv_ptr.offset(i) == ptr::null() } {
    //         break;
    //     }

    //     if let Ok(c_str) = translate_virtual_address(trapframe.page_table, unsafe {
    //         (*argv_ptr.offset(i)).addr()
    //     }) {
    //         let string = unsafe {
    //             CStr::from_ptr(c_str as *const c_char)
    //                 .to_str()
    //                 .unwrap()
    //                 .to_string()
    //         };
    //         argv.push(string);
    //         i += 1;
    //     } else {
    //         break;
    //     }
    // }
    // }

    if let Err(e) = process.prepare_for_execve().and_then(|page_table| {
        process.free_for_execve(page_table, old_heap_end);
        Ok(())
    }) {
        if e == Error::NoFreePage {
            return -ENOMEM as usize;
        } else {
            panic!("IN EXECVE: {}", e);
        }
    }

    for header in program_headers {
        let file_size = header.p_filesz as usize;
        let mem_size = header.p_memsz as usize;

        let num_pages = ceil(mem_size as f64 / PAGE_SIZE as f64) as usize;

        if num_pages == 0 {
            continue;
        }

        page = allocate(num_pages).expect("INIT FAILED - FAILED TO ALLOCATE PAGE FOR CODE");

        let offset = header.p_offset as usize;

        let va = header.p_vaddr as usize;
        let flags = header.p_flags;

        if va + num_pages * PAGE_SIZE > max_code_page_end_va {
            max_code_page_end_va = va + num_pages * PAGE_SIZE;
        }

        let mut permissions = 0;

        if flags & elf::abi::PF_R != 0 {
            permissions |= READ_ONLY;
        }

        if flags & elf::abi::PF_W != 0 {
            permissions |= WRITE_ONLY;
        }

        if flags & elf::abi::PF_X != 0 {
            permissions |= EXECUTE_ONLY;
        }

        let loadable = &buffer[offset..offset + file_size];

        map_code_pages(process.page_table, page, va, num_pages, permissions);
        process.size += num_pages * PAGE_SIZE;

        unsafe {
            ptr::copy_nonoverlapping(loadable.as_ptr(), page as *mut u8, file_size);
        }
    }

    if page == 0 {
        panic!("PANIC: INIT FAILED - ELF CONTAINS NO LOADABLE SEGMENT");
    }

    map_other_pages(process.page_table, max_code_page_end_va, process)
        .expect("INIT FAILED - ERROR WHILE MAPPING PAGES");

    let trapframe = process
        .trapframe
        .as_mut()
        .expect("TRAPFRAME NONE IN FIRST TIME EXECUTION");

    trapframe.sepc = elf_data.ehdr.e_entry as usize;

    process.context.sp = trapframe.kernel_stack + PAGE_SIZE;
    trapframe.brk.set(max_code_page_end_va);
    trapframe.heap_end.set(max_code_page_end_va);

    if let ProcessState::Running { cwd } = &process.state {
        process.state = ProcessState::Ready { cwd: cwd.clone() };
    }

    process.name = path.into();
    0
}
