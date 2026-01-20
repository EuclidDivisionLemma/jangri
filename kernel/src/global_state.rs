use alloc::{collections::btree_map::BTreeMap, rc::Rc, sync::Arc};
use allocator::PageAllocator;
use anyhow::{Result, bail};
use spin::{Once, rwlock::RwLock};
use sync::{Lock, Mutex};

use crate::{
    constants::{KERNEL_END, RAM_STOP},
    fs::sfs::MemoryINode,
    process::{Process, ProcessState, are_interrupts_enabled},
    scheduler::Context,
};

static GLOBAL_STATE: Once<GlobalState> = Once::new();

pub struct GlobalState {
    allocator: Mutex<PageAllocator>,
    processes: RwLock<BTreeMap<usize, Arc<Mutex<Process>>>>,
    current_process: Mutex<Option<Arc<Mutex<Process>>>>,
    pub scheduler_context: Context,
}

impl GlobalState {
    pub fn initialise() -> &'static Self {
        let state = GlobalState {
            allocator: Mutex::new(
                PageAllocator::new(
                    &|_| bail!("Not yet implemented"),
                    unsafe { KERNEL_END },
                    RAM_STOP,
                ),
                1,
                || 0,
                riscv::interrupt::supervisor::enable,
                riscv::interrupt::supervisor::disable,
                are_interrupts_enabled,
            ),
            processes: RwLock::new(BTreeMap::new()),
            current_process: Mutex::new(
                None,
                1,
                || 0,
                riscv::interrupt::supervisor::enable,
                riscv::interrupt::supervisor::disable,
                are_interrupts_enabled,
            ),
            scheduler_context: Context::default(),
        };

        GLOBAL_STATE.call_once(|| state)
    }

    pub fn allocate(&self, size: usize) -> Result<usize> {
        let mut allocator = self.allocator.lock();
        allocator.allocate(size)
    }

    pub fn deallocate(&self, start: usize, size: usize) {
        let mut allocator = self.allocator.lock();
        allocator.deallocate(start, size);
    }

    pub fn get() -> &'static Self {
        GLOBAL_STATE.get().unwrap()
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
}

unsafe impl Send for GlobalState {}
unsafe impl Sync for GlobalState {}
