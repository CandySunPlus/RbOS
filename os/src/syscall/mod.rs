use self::fs::{sys_close, sys_open, sys_read, sys_write};
use self::process::{
    sys_exec, sys_exit, sys_fork, sys_get_pid, sys_get_time, sys_mmap, sys_munmap, sys_sbrk,
    sys_set_priority, sys_spawn, sys_task_info, sys_waitpid, sys_yield,
};
use crate::task::current_task;

mod fs;
mod process;

const SYSCALL_OPEN: usize = 56;
const SYSCALL_CLOSE: usize = 57;
const SYSCALL_READ: usize = 63;
const SYSCALL_WRITE: usize = 64;
const SYSCALL_EXIT: usize = 93;
const SYSCALL_YIELD: usize = 124;
const SYSCALL_SET_PRIORITY: usize = 140;
const SYSCALL_GET_TIME: usize = 169;
const SYSCALL_GET_PID: usize = 172;
const SYSCALL_SBRK: usize = 214;
const SYSCALL_MUNMAP: usize = 215;
const SYSCALL_FORK: usize = 220;
const SYSCALL_EXEC: usize = 221;
const SYSCALL_MMAP: usize = 222;
const SYSCALL_WAITPID: usize = 260;
const SYSCALL_SPAWN: usize = 400;
const SYSCALL_TASK_INFO: usize = 410;

pub fn syscall(syscall_id: usize, args: [usize; 3]) -> isize {
    current_task().unwrap().record_syscall_times(syscall_id);
    match syscall_id {
        SYSCALL_OPEN => sys_open(args[0] as _, args[1] as _),
        SYSCALL_CLOSE => sys_close(args[0] as _),
        SYSCALL_READ => sys_read(args[0], args[1] as _, args[2]),
        SYSCALL_WRITE => sys_write(args[0], args[1] as _, args[2]),
        SYSCALL_EXIT => sys_exit(args[0] as _),
        SYSCALL_YIELD => sys_yield(),
        SYSCALL_SET_PRIORITY => sys_set_priority(args[0] as _),
        SYSCALL_GET_TIME => sys_get_time(args[0] as _, args[1]),
        SYSCALL_GET_PID => sys_get_pid(),
        SYSCALL_SBRK => sys_sbrk(args[0] as _),
        SYSCALL_MUNMAP => sys_munmap(args[0], args[1]),
        SYSCALL_MMAP => sys_mmap(args[0], args[1], args[2]),
        SYSCALL_FORK => sys_fork(),
        SYSCALL_EXEC => sys_exec(args[0] as _),
        SYSCALL_WAITPID => sys_waitpid(args[0] as _, args[1] as _),
        SYSCALL_SPAWN => sys_spawn(args[0] as _),
        SYSCALL_TASK_INFO => sys_task_info(args[0] as *mut _),
        syscall_id => unreachable!("Unsupported syscall {}", syscall_id),
    }
}
