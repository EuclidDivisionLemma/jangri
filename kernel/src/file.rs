use core::{
    array,
    cell::{Cell, RefCell},
    num::NonZeroUsize,
};

use alloc::{
    collections::btree_map::BTreeMap,
    format,
    rc::Rc,
    slice,
    string::{String, ToString},
    vec::Vec,
};
use anyhow::{Result, bail};

use crate::{
    DEVICE,
    constants::ROOT_INODE,
    error::Error,
    fs::sfs::{
        DirectoryEntry, DiskINode, InodeEntry, MemoryINode, allocate_inode, read_inode,
        read_inode_data, write_inode, write_inode_data,
    },
    global_state::GlobalState,
    pipe::Pipe,
    process::ProcessState,
};

pub const STDIN: usize = 0;
pub const STDOUT: usize = 1;
pub const STDERR: usize = 2;

pub static mut FILES: BTreeMap<usize, Rc<File>> = BTreeMap::new();
static mut FD: usize = 0;

pub enum FileType {
    Pipe(Rc<Pipe>),
    INode {
        inode: Rc<MemoryINode>,
        offset: usize,
        append: bool,
    },
    Device {
        inode: Rc<MemoryINode>,
        major: u8,
    },
    Free,
}

pub struct File {
    pub file_type: RefCell<FileType>,
    pub readable: RefCell<bool>,
    pub writeable: RefCell<bool>,
    pub fd: RefCell<usize>,
}

pub fn allocate_file() -> Rc<File> {
    let file = Rc::new(File {
        file_type: RefCell::new(FileType::Free),
        readable: RefCell::new(false),
        writeable: RefCell::new(false),
        fd: unsafe {
            FD += 1;
            RefCell::new(FD - 1)
        },
    });
    unsafe {
        FILES.insert(*file.fd.borrow(), file.clone());
    }

    file
}

/// Returns the sub-path with the first component removed along with the first component.
/// For example, if the passed path is `/home/foo`, the function returns `("/foo", "home")`.
pub fn next_path_element(path: &str) -> (String, String) {
    if path == "/" {
        return ("".into(), "/".into());
    }
    let mut path = match path.strip_prefix("/") {
        Some(v) => v,
        None => path,
    }
    .to_string();

    path.push('/');

    let mut components = Vec::new();
    let mut s = String::new();

    for ch in path.chars() {
        if ch == '/' {
            if !s.is_empty() {
                components.push(s);
                s = "".into();
            }
        } else {
            s.push(ch);
        }
    }

    (components[1..].join("/"), components[0].to_string())
}

/// Takes an inode, and a name, searches if the directory that is associated with inode contains any
/// members (files or directories) that go by the name. If so, returns it, else None.
///
/// For example, to get the inode of the file `home` in `/home/bar`, the call would be
/// `directory_search(home_inode, "bar")`, where `home_inode` is the inode of the file `home`.
pub fn search_in_directory(inode: &Rc<MemoryINode>, name: &str) -> Result<Rc<MemoryINode>> {
    if inode.entry.get() != InodeEntry::Directory {
        bail!(Error::NotADirectory { name: name.into() });
    } else {
        for i in 0..inode.size.get() / size_of::<DirectoryEntry>() {
            let bytes = &read_inode_data(
                inode,
                i * size_of::<DirectoryEntry>(),
                size_of::<DirectoryEntry>(),
                false,
                &DEVICE,
            )?[..];
            let entry = unsafe { &*(bytes.as_ptr() as *const DirectoryEntry) };

            if str::from_utf8(
                &entry
                    .name
                    .iter()
                    .filter_map(|ch| if *ch != 0 { Some(*ch) } else { None })
                    .collect::<Vec<u8>>()
                    .as_slice(),
            )
            .expect("INVALID UTF-8 NAME IN search_in_directory")
                == name
                && entry.inum != 0
            {
                return Ok(read_inode(NonZeroUsize::new(entry.inum).unwrap(), &DEVICE));
            }
        }
    }

    bail!(Error::NoSuchEntryInDirectory {
        name: name.to_string(),
    })
}

/// Traverses a path and returns the inode of the last component.
/// For example, if the path is `/home/bar`, returns the inode of `bar`
pub fn traverse_path(state: &GlobalState, path: &str, parent: bool) -> Result<Rc<MemoryINode>> {
    let mut inode: Rc<MemoryINode>;

    if path == "/" {
        return Ok(read_inode(ROOT_INODE, &DEVICE));
    }

    if path.chars().collect::<Vec<char>>()[0] == '/' {
        inode = read_inode(ROOT_INODE, &DEVICE);
    } else {
        inode = if let Some(process) = state.get_current_process() {
            let process = process.lock();

            match &process.process_state {
                ProcessState::Ready { cwd }
                | ProcessState::Running { cwd }
                | ProcessState::Sleeping { cwd, sleep_on: _ } => cwd.clone(),
                _ => panic!("OTHER STATE IN TRAVERSE_PATH"),
            }
        } else {
            read_inode(ROOT_INODE, &DEVICE)
        };
    }

    let mut path = path.to_string();
    let mut name: String;

    loop {
        (path, name) = next_path_element(&path);

        if path == "" && parent == true {
            break;
        }

        inode = search_in_directory(&inode, &name)?;

        if path == "" {
            break;
        }
    }

    Ok(inode)
}

pub fn exists(state: &GlobalState, path: &str) -> Result<bool> {
    match traverse_path(state, path, false) {
        Ok(_) => Ok(true),
        Err(e)
            if matches!(
                e.downcast_ref().unwrap(),
                Error::NoSuchEntryInDirectory { name: _ }
            ) =>
        {
            Ok(false)
        }
        Err(e) => Err(e),
    }
}

pub fn create_file(state: &GlobalState, path: &str, kind: InodeEntry) -> Result<NonZeroUsize> {
    if exists(state, path)? {
        bail!(Error::FileAlreadyExists {
            path: path.to_string(),
        });
    }

    let parent_inode = traverse_path(state, path, true)?;
    let parent = read_inode(parent_inode.inum, &DEVICE);

    if parent.entry.get() != InodeEntry::Directory {
        bail!(Error::NotADirectory {
            name: format!("One of the parents not a directory {:?}", path),
        });
    }

    let last = path.split("/").last().ok_or(Error::InvalidPath)?;
    let name = last.strip_prefix("/").unwrap_or(last).as_bytes();
    let inum_of_file = allocate_inode(&DEVICE)?;

    let entry = DirectoryEntry {
        name: array::from_fn(|i| if i < name.len() { name[i] } else { 0 }),
        inum: inum_of_file.get(),
    };

    write_inode_data(
        &parent,
        parent.size.get(),
        unsafe {
            slice::from_raw_parts(&raw const entry as *const u8, size_of::<DirectoryEntry>())
                .to_vec()
        },
        &DEVICE,
    )?;

    let mut file_inode = MemoryINode::default();
    file_inode.entry.set(kind);
    file_inode.links.set(1);
    file_inode.inum = inum_of_file;

    write_inode(Rc::new(file_inode), &DEVICE, false)?;

    Ok(inum_of_file)
}

pub fn open(
    state: &GlobalState,
    path: &str,
    readable: bool,
    writeable: bool,
    create: bool,
    excl: bool,
    truncate: bool,
    append: bool,
) -> Result<usize> {
    if exists(state, path)? & create & excl {
        bail!(Error::FileAlreadyExists {
            path: path.to_string(),
        });
    }

    let inode = match traverse_path(state, path, false) {
        Ok(inode) => inode,
        Err(e) => {
            if let Error::NoSuchEntryInDirectory { name: _ } = e.downcast_ref().unwrap()
                && create
            {
                read_inode(create_file(state, path, InodeEntry::File)?, &DEVICE)
            } else {
                return Err(e);
            }
        }
    };

    let size = inode.size.get();

    let file_type = match inode.entry.get() {
        InodeEntry::File | InodeEntry::Directory => FileType::INode {
            inode: inode.clone(),
            offset: if append & writeable { size } else { 0 },
            append,
        },
        InodeEntry::Device => FileType::Device {
            inode: inode.clone(),
            major: inode.major.get(),
        },
        InodeEntry::SymLink => todo!(),
        InodeEntry::None => {
            bail!(Error::FreeInode {
                inode: DiskINode::from(inode.as_ref()),
            });
        }
    };

    let file = allocate_file();

    if let Some(current_process) = state.get_current_process() {
        let mut current_process = current_process.lock();
        current_process.fds.push(*file.fd.borrow());
    }
    *file.file_type.borrow_mut() = file_type;
    *file.readable.borrow_mut() = readable;
    *file.writeable.borrow_mut() = writeable;

    if truncate {
        inode.size.set(0);
    }

    Ok(*file.fd.borrow())
}
