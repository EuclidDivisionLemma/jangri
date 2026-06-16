use alloc::collections::linked_list::LinkedList;
use alloc::{collections::btree_map::BTreeMap, sync::Arc};
use allocator::PageAllocator;
use hal::Hal;
use hal::constants::PAGE_SIZE;
use hal::error::Result;
use hal::interrupts::InterruptHandling;
use hal::vm::{PageTable, VirtualMemory};

use crate::traps::initialise_global_state_for_trap_handlers;
use crate::{
    ARCH, Mutex, PAGE_TABLE_ENTRY, RwLock,
    constants::{KERNEL_END, RAM_STOP},
    process::{Process, ProcessState},
    scheduler::Context,
};

pub struct GlobalState {
    allocator: Arc<Mutex<PageAllocator>>,
    processes: RwLock<BTreeMap<usize, Arc<Mutex<Process>>>>,
    pids: Mutex<LinkedList<usize>>,
    current_process: Mutex<Option<Arc<Mutex<Process>>>>,
    pub scheduler_context: Context,
    arch: Mutex<ARCH>,
}

unsafe impl Send for GlobalState {}
unsafe impl Sync for GlobalState {}

impl GlobalState {
    pub fn initialise() -> &'static Self {
        let allocator0 = Arc::new(Mutex::new(PageAllocator::new(
            &|_| todo!("Page eviction is not implemented. It's a hobby OS afterall!"),
            unsafe { KERNEL_END },
            RAM_STOP,
        )));

        let allocator1 = allocator0.clone();

        let state = GlobalState {
            allocator: allocator1.clone(),
            processes: RwLock::new(BTreeMap::new()),
            pids: Mutex::new(LinkedList::new()),
            current_process: Mutex::new(None),
            scheduler_context: Context::default(),
            arch: Mutex::new(ARCH::new(
                Arc::new(move |size| {
                    let mut allocator = allocator0.lock();
                    assert!(size.is_power_of_two() && size >= PAGE_SIZE);

                    allocator.allocate(size)
                }),
                Arc::new(move |addr, size| {
                    let mut allocator = allocator1.lock();
                    allocator.deallocate(addr, size)
                }),
            )),
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

        let mut pids = self.pids.lock();
        pids.push_back(id);
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

    pub fn find_ready_process(&self) -> Option<usize> {
        let processes = self.processes.read();
        let mut pids = self.pids.lock();

        for _ in 0..pids.len() {
            let pid = pids.pop_front();

            match pid {
                Some(pid) => {
                    pids.push_back(pid);

                    let process = processes.get(&pid).unwrap();
                    let process = process.lock();

                    if let ProcessState::Ready = process.process_state {
                        return Some(pid);
                    }
                }
                None => return None,
            }
        }

        None
    }

    pub fn find_sleeping_process(&self, sleep_on: usize) -> Option<usize> {
        let processes = self.processes.read();

        for (pid, process) in processes.iter() {
            let process = process.lock();

            if let ProcessState::Sleeping { sleep_on: s } = &process.process_state
                && *s == sleep_on
            {
                return Some(*pid);
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

    pub unsafe fn enable_interrupts(&self) {
        unsafe {
            ARCH::enable_interrupts();
        }
    }

    pub fn disable_interrupts(&self) {
        ARCH::disable_interrupts();
    }

    pub fn cleanup_page_table(&self, page_table: usize) -> Result<()> {
        let arch = self.arch.lock();
        arch.clean_up_page_table(page_table as *mut PageTable<PAGE_TABLE_ENTRY>)
    }

    pub fn va2pa(&self, page_table: usize, va: usize) -> Result<usize> {
        let arch = self.arch.lock();
        arch.va2pa(page_table as *mut PageTable<PAGE_TABLE_ENTRY>, va)
    }

    #[allow(unused)]
    pub fn unmap(
        &self,
        page_table: usize,
        va: usize,
        num_pages: usize,
        deallocate: bool,
    ) -> Result<()> {
        let arch = self.arch.lock();
        arch.unmap(
            page_table as *mut PageTable<PAGE_TABLE_ENTRY>,
            va,
            num_pages,
            deallocate,
        )
    }
}
