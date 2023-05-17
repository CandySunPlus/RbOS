use self::fs::{sys_read, sys_write};
use self::process::{
    sys_exit, sys_get_time, sys_mmap, sys_munmap, sys_sbrk, sys_task_info, sys_yield,
};
use crate::task::inc_syscall_times;

mod fs;
mod process;

const SYSCALL_READ: usize = 63;
const SYSCALL_WRITE: usize = 64;
const SYSCALL_EXIT: usize = 93;
const SYSCALL_YIELD: usize = 124;
const SYSCALL_GET_TIME: usize = 169;
const SYSCALL_SBRK: usize = 214;
const SYSCALL_MUNMAP: usize = 215;
const SYSCALL_FORK: usize = 220;
const SYSCALL_EXEC: usize = 221;
const SYSCALL_MMAP: usize = 222;
const SYSCALL_WAITPID: usize = 260;
const SYSCALL_TASK_INFO: usize = 410;

pub fn syscall(syscall_id: usize, args: [usize; 3]) -> isize {
    inc_syscall_times(syscall_id);
    match syscall_id {
        SYSCALL_READ => sys_read(args[0], args[1] as *const u8, args[2]),
        SYSCALL_WRITE => sys_write(args[0], args[1] as *const u8, args[2]),
        SYSCALL_EXIT => sys_exit(args[0] as i32),
        SYSCALL_YIELD => sys_yield(),
        SYSCALL_GET_TIME => sys_get_time(args[0] as *mut _, args[1]),
        SYSCALL_SBRK => sys_sbrk(args[0] as i32),
        SYSCALL_MUNMAP => sys_munmap(args[0], args[1]),
        SYSCALL_MMAP => sys_mmap(args[0], args[1], args[2]),
        SYSCALL_TASK_INFO => sys_task_info(args[0] as *mut _),
        syscall_id => unreachable!("Unsupported syscall {}", syscall_id),
    }
}
