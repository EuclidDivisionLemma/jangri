use core::{
    fmt::Debug,
    ops::{Add, RangeBounds},
};

use alloc::{
    collections::{
        btree_map::{self, BTreeMap},
        vec_deque::VecDeque,
    },
    vec::Vec,
};

use {
    crate::error::{Error, Result},
    crate::sfs::{BLOCK_SIZE, DATA_CACHE, LRU_CACHE_CAPACITY},
};

#[derive(Debug, Clone)]
pub struct Interval {
    pub start: usize,
    pub end: usize,
    pub data: Vec<u8>,
    pub needs_write: bool,
}

impl Interval {
    pub fn new(start: usize, end: usize, data: Vec<u8>, needs_write: bool) -> Result<Self> {
        if data.len() == (end - start) * BLOCK_SIZE {
            Ok(Self {
                start,
                end,
                data,
                needs_write,
            })
        } else {
            Err(Error::IntervalsNotConsecutive)
        }
    }
}

impl Add for Interval {
    type Output = Self;

    fn add(mut self, mut rhs: Self) -> Self::Output {
        // the intervals are of form [a, b) and [b, c)

        assert!(
            self.end == rhs.start,
            "Intervals not consecutive: {:?} and {:?}",
            self,
            rhs
        );

        let mut buf = Vec::with_capacity(self.data.len() + rhs.data.len());
        buf.append(&mut self.data);
        buf.append(&mut rhs.data);
        self.data = buf;
        self.end = rhs.end;

        self
    }
}

pub fn coalesce<'a>(
    mut interval: Interval,
    on_preceding_merge: Option<&'a mut dyn FnMut(usize)>,
    on_succeeding_merge: Option<&'a mut dyn FnMut(usize)>,
) -> Interval {
    let block = interval.start;

    if let Some(key) = unsafe { DATA_CACHE.range(..block).next_back() }
        && let Some(predecessor) = unsafe { DATA_CACHE.get_mut(key) }
        && predecessor.end == predecessor.start
        && predecessor.needs_write
    {
        let predessor_start = predecessor.start;

        interval = predecessor.clone() + interval;

        unsafe {
            DATA_CACHE.remove(&predessor_start);
        }

        if let Some(f) = on_preceding_merge {
            f(predecessor.start);
        }
    }

    if let Some(key) = unsafe { DATA_CACHE.range(block..).next_back() }
        && let Some(successor) = unsafe { DATA_CACHE.get_mut(key) }
        && interval.end == successor.start
        && successor.needs_write
    {
        let sucessor_start = successor.start;

        interval = interval + successor.clone();

        unsafe {
            DATA_CACHE.remove(&sucessor_start);
        }

        if let Some(f) = on_succeeding_merge {
            f(successor.start);
        }
    }

    interval
}

pub struct Lru<K: Ord + Copy, V> {
    intervals: BTreeMap<K, V>,
    lru: VecDeque<K>,
}

impl<K: Ord + Copy, V> Lru<K, V> {
    pub const fn new() -> Self {
        Self {
            intervals: BTreeMap::new(),
            lru: VecDeque::new(),
        }
    }

    pub fn insert(&mut self, key: K, value: V) {
        // If the key value pair is present, just update the value and move to front
        if let Some(v) = self.intervals.get_mut(&key) {
            *v = value;
            self.move_to_front_if_necessary(&key);
        }
        // if not present insert them
        else {
            // if cache is full, evict to make some space
            if self.lru.len() > LRU_CACHE_CAPACITY
                && let Some(key) = self.lru.pop_back()
            {
                // note that `key` here is the one that was popped, its scope ends
                self.intervals.remove(&key);
            }

            self.intervals.insert(key, value); // the key here is the argument passed
            self.lru.push_front(key);
        }
    }

    pub fn get(&mut self, key: &K) -> Option<&V> {
        self.move_to_front_if_necessary(key);
        self.intervals.get(key)
    }

    pub fn get_mut(&mut self, key: &K) -> Option<&mut V> {
        self.move_to_front_if_necessary(key);
        self.intervals.get_mut(key)
    }

    pub fn range<'a, T: RangeBounds<K>>(&'a self, range: T) -> impl DoubleEndedIterator<Item = &K> {
        self.intervals.range(range).map(|(k, _)| k)
    }

    pub fn remove(&mut self, key: &K) {
        self.intervals.remove(key);

        for (i, k) in self.lru.iter().enumerate() {
            if *k == *key {
                self.lru.remove(i);
                return;
            }
        }
    }

    fn move_to_front_if_necessary(&mut self, key: &K) {
        if !self.lru.contains(key) {
            return;
        }

        if let Some(k) = self.lru.front()
            && *k == *key
        {
            return;
        }

        let mut index_to_be_removed = 0;
        let mut remove = false;

        for (i, k) in self.lru.iter().enumerate() {
            if *k == *key {
                index_to_be_removed = i;
                remove = true;
                break;
            }
        }

        if remove {
            self.lru.remove(index_to_be_removed);
            self.lru.push_front(*key);
        }
    }
}

impl<'a, K: Copy + Ord, V> IntoIterator for &'a Lru<K, V> {
    type Item = (&'a K, &'a V);

    type IntoIter = btree_map::Iter<'a, K, V>;

    fn into_iter(self) -> Self::IntoIter {
        self.intervals.iter()
    }
}

impl<'a, K: Copy + Ord, V> IntoIterator for &'a mut Lru<K, V> {
    type Item = (&'a K, &'a mut V);

    type IntoIter = btree_map::IterMut<'a, K, V>;

    fn into_iter(self) -> Self::IntoIter {
        self.intervals.iter_mut()
    }
}
