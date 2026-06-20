use core::num::NonZero;

pub const NUM_MULTIPLES_PER_BITMAP: usize = 128;
pub const NUM_BITMAPS: usize = 2560 / NUM_MULTIPLES_PER_BITMAP;

pub struct Bitmap {
    inner: [u128; NUM_BITMAPS],
}

impl Bitmap {
    pub const fn new() -> Self {
        Self {
            inner: [0u128; NUM_BITMAPS],
        }
    }

    pub fn mark_available(&mut self, multiple: usize) {
        assert!(multiple != 0);
        let multiple = multiple - 1;
        let index = multiple / NUM_MULTIPLES_PER_BITMAP;
        let offset = multiple % NUM_MULTIPLES_PER_BITMAP;
        let mut bitmap = *self.inner.get(index).expect("Multiple must be <= 2560");
        bitmap = bitmap | ((1u128 << 127) >> offset);
        *self.inner.get_mut(index).unwrap() = bitmap;
    }

    #[allow(dead_code)]
    pub fn is_available(&self, multiple: usize) -> bool {
        assert!(multiple != 0);
        let multiple = multiple - 1;
        let index = multiple / NUM_MULTIPLES_PER_BITMAP;
        let offset = multiple % NUM_MULTIPLES_PER_BITMAP;
        let bitmap = *self.inner.get(index).expect("Multiple must be <= 2560");
        if bitmap & ((1u128 << 127) >> offset) == 0 {
            false
        } else {
            true
        }
    }

    pub fn mark_unavailable(&mut self, multiple: usize) {
        assert!(multiple != 0);
        let multiple = multiple - 1;
        let index = multiple / NUM_MULTIPLES_PER_BITMAP;
        let offset = multiple % NUM_MULTIPLES_PER_BITMAP;
        let mut bitmap = *self.inner.get(index).expect("Multiple must be <= 2560");
        bitmap = bitmap & !((1u128 << 127) >> offset);
        *self.inner.get_mut(index).unwrap() = bitmap;
    }

    pub fn first_available(&self, multiple: usize) -> Option<usize> {
        assert!(multiple != 0);
        let multiple = multiple - 1;
        let index = multiple / NUM_MULTIPLES_PER_BITMAP;
        let mut offset = multiple % NUM_MULTIPLES_PER_BITMAP;
        for i in index..NUM_BITMAPS {
            if let Some(v) = NonZero::new(*self.inner.get(i).unwrap()) {
                for j in offset..128 {
                    if v.get() & ((1 << 127) >> j) != 0 {
                        return Some((i * NUM_MULTIPLES_PER_BITMAP + j) + 1);
                    }
                }
            }
            offset = 0;
        }

        None
    }
}
