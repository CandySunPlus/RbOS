#![no_std]
#![no_main]

use user_lib::{
    get_time, println, sleep, task_info, TaskInfo, TaskStatus, SYSCALL_EXIT, SYSCALL_GET_TIME,
    SYSCALL_TASK_INFO, SYSCALL_WRITE, SYSCALL_YIELD,
};

#[no_mangle]
pub fn main() -> usize {
    let t1 = get_time() as usize;
    let info = TaskInfo::new();

    get_time();
    sleep(500);

    let t2 = get_time() as usize;

    assert_eq!(0, task_info(&info));

    let t3 = get_time() as usize;

    assert!(3 <= info.syscall_times[SYSCALL_GET_TIME]);
    assert_eq!(1, info.syscall_times[SYSCALL_TASK_INFO]);
    assert_eq!(0, info.syscall_times[SYSCALL_WRITE]);
    assert!(0 < info.syscall_times[SYSCALL_YIELD]);
    assert_eq!(0, info.syscall_times[SYSCALL_EXIT]);

    assert!(t2 - t1 <= info.time + 1);
    assert!(info.time < t3 - t1 + 100);
    assert_eq!(info.status, TaskStatus::Running);

    println!("string from task info test\n");
    let t4 = get_time() as usize;
    assert_eq!(0, task_info(&info));
    let t5 = get_time() as usize;
    assert!(5 <= info.syscall_times[SYSCALL_GET_TIME]);
    assert_eq!(2, info.syscall_times[SYSCALL_TASK_INFO]);
    assert_eq!(1, info.syscall_times[SYSCALL_WRITE]);
    assert!(0 < info.syscall_times[SYSCALL_YIELD]);
    assert_eq!(0, info.syscall_times[SYSCALL_EXIT]);
    assert!(t4 - t1 <= info.time + 1);
    assert!(info.time < t5 - t1 + 100);
    assert!(info.status == TaskStatus::Running);

    println!("Test task info OK!");

    0
}
