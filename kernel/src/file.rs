use core::{array, cell::Cell, num::NonZeroUsize};

use alloc::{
    format,
    rc::Rc,
    slice,
    string::{String, ToString},
    vec::Vec,
};

use crate::{
    DEVICE,
    constants::ROOT_INODE,
    error::{Error, Result},
    fs::sfs::{
        DirectoryEntry, InodeEntry, MemoryINode, allocate_inode, read_inode, read_inode_data,
        write_inode, write_inode_data,
    },
    pipe::Pipe,
    process::CURRENT_PROCESS,
};

pub static mut FILES: Vec<Rc<File>> = Vec::new();

pub enum FileType {
    Pipe(Rc<Pipe>),
    INode {
        inode: Rc<MemoryINode>,
        offset: usize,
    },
    Device {
        inode: Rc<MemoryINode>,
        major: u8,
    },
    Free,
}

pub struct File {
    pub file_type: Cell<FileType>,
    pub readable: Cell<bool>,
    pub writeable: Cell<bool>,
    pub fd: usize,
}

pub fn allocate_file() -> Rc<File> {
    let file = Rc::new(File {
        file_type: Cell::new(FileType::Free),
        readable: Cell::new(false),
        writeable: Cell::new(false),
        fd: unsafe { FILES.len() },
    });
    unsafe {
        FILES.push(file.clone());
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
        panic!("INODE NOT THAT OF A DIRECTORY: IN directory_search");
    } else {
        for i in 0..inode.size.get() / size_of::<DirectoryEntry>() {
            let bytes = &read_inode_data(
                inode,
                i * size_of::<DirectoryEntry>(),
                size_of::<DirectoryEntry>(),
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
                && entry.inum.get() != 0
            {
                return Ok(read_inode(entry.inum, &DEVICE));
            }
        }
    }

    Err(Error::NoSuchEntryInDirectory {
        name: name.to_string(),
    })
}

/// Traverses a path and returns the inode of the last component.
/// For example, if the path is `/home/bar`, returns the inode of `bar`
pub fn traverse_path(path: &str, parent: bool) -> Result<Rc<MemoryINode>> {
    let mut inode: Rc<MemoryINode>;

    if path == "/" {
        return Ok(read_inode(ROOT_INODE, &DEVICE));
    }

    if path.chars().collect::<Vec<char>>()[0] == '/' {
        inode = read_inode(ROOT_INODE, &DEVICE);
    } else {
        inode = if let Some(process) = unsafe { CURRENT_PROCESS.as_ref() } {
            process.cwd.clone()
        } else {
            read_inode(ROOT_INODE, &DEVICE)
        };
    }

    loop {
        let (path, name) = next_path_element(path);

        if path == "" && parent == true {
            break;
        }

        let next = search_in_directory(&inode, &name);

        match next {
            Ok(v) => inode = v,
            Err(e) => return Err(e),
        }

        if path == "" {
            break;
        }
    }

    Ok(inode)
}

pub fn exists(path: &str) -> Result<bool> {
    match traverse_path(path, false) {
        Ok(_) => Ok(true),
        Err(e) if let Error::NoSuchEntryInDirectory { name: _ } = e => Ok(false),
        Err(e) => Err(e),
    }
}

pub fn create_fs_file(path: &str, is_directory: bool) -> Result<NonZeroUsize> {
    if exists(path)? {
        return Err(Error::FileAlreadyExists {
            path: path.to_string(),
        });
    }

    let parent_inode = traverse_path(path, true)?;
    let parent = read_inode(parent_inode.inum, &DEVICE);

    if parent.entry.get() != InodeEntry::Directory {
        return Err(Error::NotADirectory {
            name: format!("One of the parents not a directory {:?}", path),
        });
    }

    let last = path.split("/").last().ok_or(Error::InvalidPath)?;
    let name = last.strip_prefix("/").unwrap_or(last).as_bytes();
    let inum_of_file = allocate_inode(&DEVICE)?;

    let entry = DirectoryEntry {
        name: array::from_fn(|i| if i < name.len() { name[i] } else { 0 }),
        inum: inum_of_file,
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
    if is_directory {
        file_inode.entry.set(InodeEntry::Directory);
    } else {
        file_inode.entry.set(InodeEntry::File);
    }

    file_inode.inum = inum_of_file;

    write_inode(Rc::new(file_inode), &DEVICE, false)?;

    Ok(inum_of_file)
}
