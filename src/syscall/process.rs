use crate::loader::run_next_app;

pub fn sys_exit(xstate: i32) -> ! {
    println!("[Kernel] Application exited with code {}", xstate);
    run_next_app()
}
