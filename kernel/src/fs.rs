use core::ptr;

use crate::constants::KERNEL_PAGE_TABLE;
use crate::drivers::Storage;
use crate::file::{self, create_file};
use crate::fs::sfs::{
    DirectoryEntry, FILE_NAME_SIZE, InodeEntry, flush_data_blocks, read_inode, write_inode,
    write_inode_data,
};
use crate::global_state::GlobalState;
use crate::sfs::allocate_inode;
use crate::traps::TrapFrame;
use crate::vm::SUPERVISOR;
use crate::{DEVICE, INIT, syscall};
pub mod caching;
pub mod sfs;

pub fn initialise(state: &GlobalState) {
    sfs::initialise(&DEVICE);
    initialise_root(&DEVICE);
    initialise_devices(state);

    file::open(state, "/dev/stdin", true, false, false, false, false, false).unwrap();
    file::open(
        state,
        "/dev/stdout",
        false,
        true,
        false,
        false,
        false,
        false,
    )
    .unwrap();
    file::open(
        state,
        "/dev/stderr",
        false,
        true,
        false,
        false,
        false,
        false,
    )
    .unwrap();

    // Copy sh.elf to /sh
    let fd = file::open(state, "sh", true, true, true, true, false, false).unwrap();

    let mut mock_traptrame = TrapFrame::default();
    mock_traptrame.a0 = fd;
    mock_traptrame.a1 = INIT.as_ptr().addr();
    mock_traptrame.page_table = unsafe { KERNEL_PAGE_TABLE };
    mock_traptrame.a2 = INIT.len();

    unsafe {
        SUPERVISOR = true;
    }

    syscall::io::write(state, &mock_traptrame);
    syscall::io::close(state, &mock_traptrame);

    unsafe {
        SUPERVISOR = false;
    }
}

pub fn initialise_root(device: &'static dyn Storage) {
    let inum = allocate_inode(device).expect("ROOT INITIALISATION ERROR - INODE ALLOCATION FAILED");
    let inode = read_inode(inum, device);
    inode.size.set(2 * size_of::<DirectoryEntry>());

    inode.entry.set(InodeEntry::Directory);
    inode.links.set(1);

    let mut current: [u8; FILE_NAME_SIZE] = [0; FILE_NAME_SIZE];
    let mut parent: [u8; FILE_NAME_SIZE] = [0; FILE_NAME_SIZE];

    unsafe {
        ptr::copy_nonoverlapping(
            &raw const ".".as_bytes()[..] as *const u8,
            current.as_mut_ptr(),
            ".".len(),
        );

        ptr::copy_nonoverlapping(
            &raw const "..".as_bytes()[..] as *const u8,
            parent.as_mut_ptr(),
            "..".len(),
        );
    }

    let current = DirectoryEntry {
        name: current,
        inum: inum.get(),
    };
    let parent = DirectoryEntry {
        name: parent,
        inum: inum.get(),
    };

    let mut buffer = [0; 2 * size_of::<DirectoryEntry>()];

    unsafe {
        ptr::copy_nonoverlapping(
            &raw const current as *const u8,
            buffer[0..size_of::<DirectoryEntry>()].as_mut_ptr(),
            size_of::<DirectoryEntry>(),
        );

        ptr::copy_nonoverlapping(
            &raw const parent as *const u8,
            buffer[size_of::<DirectoryEntry>()..].as_mut_ptr(),
            size_of::<DirectoryEntry>(),
        );
    }
    write_inode(inode.clone(), device, true)
        .expect("ERROR WHILE INITIALISING ROOT - INODE WRITE FAILED");

    write_inode_data(&inode, 0, buffer.to_vec(), device)
        .expect("ERROR WHILE INITIALISING ROOT - INODE DATA WRITE FAILED");

    flush_data_blocks(device, true);

    inode.needs_write.set(true);
}

pub fn initialise_devices(state: &GlobalState) {
    create_file(state, "/dev", InodeEntry::Directory)
        .expect("ERROR WHILE INITIALISING DEVICES - /dev CREATION FAILED");

    create_file(state, "/dev/stdin", InodeEntry::Device)
        .expect("ERROR WHILE INITIALISING DEVICES - /dev/stdin CREATION FAILED");

    create_file(state, "/dev/stdout", InodeEntry::Device)
        .expect("ERROR WHILE INITIALISING DEVICES - /dev/stdout CREATION FAILED");

    create_file(state, "/dev/stderr", InodeEntry::Device)
        .expect("ERROR WHILE INITIALISING DEVICES - /dev/stderr CREATION FAILED");
}
