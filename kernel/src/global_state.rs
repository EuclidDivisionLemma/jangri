use alloc::format;
use alloc::{boxed::Box, collections::btree_map::BTreeMap, rc::Rc, sync::Arc};
use allocator::{ALLOC, PageAllocator};
use anyhow::{Result, bail};
use hal::vm::constants::PAGE_SIZE;
use hal::vm::{PageTable, VirtualMemory};
use hal::{Hal, vm::PageTableEntry};
use riscv_arch::Riscv;

use crate::constants::END_OF_KERNEL_TEXT;
use crate::syscall::stdout;
use crate::{
    ARCH, Mutex, PAGE_TABLE_ENTRY, RwLock,
    constants::{KERNEL_END, RAM_STOP},
    fs::sfs::MemoryINode,
    process::{Process, ProcessState, are_interrupts_enabled},
    scheduler::Context,
    traps::initialise_global_state_for_trap_handlers,
};

pub struct GlobalState {
    allocator: Arc<Mutex<PageAllocator>>,
    processes: RwLock<BTreeMap<usize, Arc<Mutex<Process>>>>,
    current_process: Mutex<Option<Arc<Mutex<Process>>>>,
    pub scheduler_context: Context,
    arch: Mutex<ARCH>,
}

unsafe impl Send for GlobalState {}
unsafe impl Sync for GlobalState {}

impl GlobalState {
    pub fn initialise() -> &'static Self {
        let allocator0 = Arc::new(Mutex::new(PageAllocator::new(
            &|_| bail!("Not yet implemented"),
            unsafe { KERNEL_END },
            RAM_STOP,
        )));

        let allocator1 = allocator0.clone();

        let state = GlobalState {
            allocator: allocator1.clone(),
            processes: RwLock::new(BTreeMap::new()),
            current_process: Mutex::new(None),
            scheduler_context: Context::default(),
            arch: Mutex::new(ARCH {
                allocate: Arc::new(move |size| {
                    let mut allocator = allocator0.lock();
                    assert!(size.is_power_of_two() && size >= PAGE_SIZE);

                    allocator.allocate(size)
                }),
                deallocate: Arc::new(move |addr, size| {
                    let mut allocator = allocator1.lock();
                    allocator.deallocate(addr, size)
                }),
            }),
        };

        initialise_global_state_for_trap_handlers(state)
    }

    pub fn allocate(&self, size: usize) -> Result<usize> {
        let mut allocator = self.allocator.lock();
        allocator.allocate(size)
    }

    pub fn deallocate(&self, start: usize, size: usize) {
        let mut allocator = self.allocator.lock();
        allocator.deallocate(start, size);
    }

    pub fn add_process(&self, id: usize, process: Arc<Mutex<Process>>) {
        let mut processes = self.processes.write();
        processes.insert(id, process);
    }

    pub fn get_current_process(&self) -> Option<Arc<Mutex<Process>>> {
        self.current_process
            .lock()
            .as_ref()
            .and_then(|v| Some(v.clone()))
    }

    pub fn set_current_process(&self, process: Arc<Mutex<Process>>) {
        let mut current_process = self.current_process.lock();
        *current_process = Some(process);
    }

    pub fn find_ready_process(&self) -> Option<(usize, Rc<MemoryINode>)> {
        let processes = self.processes.read();

        for (pid, process) in processes.iter() {
            let process = process.lock();

            if let ProcessState::Ready { cwd } = &process.process_state {
                return Some((*pid, cwd.clone()));
            }
        }

        None
    }

    pub fn find_sleeping_process(&self, sleep_on: usize) -> Option<(usize, Rc<MemoryINode>)> {
        let processes = self.processes.read();

        for (pid, process) in processes.iter() {
            let process = process.lock();

            if let ProcessState::Sleeping { cwd, sleep_on: s } = &process.process_state
                && *s == sleep_on
            {
                return Some((*pid, cwd.clone()));
            }
        }

        None
    }

    pub fn get_process(&self, pid: usize) -> Option<Arc<Mutex<Process>>> {
        let processes = self.processes.read();
        processes.get(&pid).and_then(|v| Some(v.clone()))
    }

    pub fn map(
        &self,
        page_table: usize,
        va: usize,
        pa: usize,
        size: usize,
        read: bool,
        write: bool,
        execute: bool,
        user: bool,
    ) -> Result<()> {
        let arch = self.arch.lock();
        arch.map(
            page_table as *mut PageTable<PAGE_TABLE_ENTRY>,
            va,
            pa,
            size,
            read,
            write,
            execute,
            user,
        )?;

        Ok(())
    }
}
