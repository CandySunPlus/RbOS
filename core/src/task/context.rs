#[derive(Clone, Copy, Default)]
#[repr(C)]
pub struct TaskContext {
    ra: usize,
    sp: usize,
    s: [usize; 12],
}

impl TaskContext {
    pub fn zero_init() -> Self {
        Self::default()
    }

    pub fn goto_restore(kstack_ptr: usize) -> Self {
        extern "C" {
            fn __restore();
        }

        Self {
            ra: __restore as usize,
            sp: kstack_ptr,
            s: Default::default(),
        }
    }
}
