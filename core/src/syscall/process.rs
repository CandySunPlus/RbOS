use crate::batch::run_next_app;
use crate::println;

pub fn sys_exit(exit_code: i32) -> isize {
    println!("[kernel] Application exit with code {}", exit_code);
    run_next_app()
}
