use crate::config::CLOCK_FREQ;
use crate::sbi::set_timer;
use riscv::register::time;

// 1s之内的时间片数量
const TICKS_PER_SEC: usize = 100;
const MICRO_PER_SEC: usize = 1_000_000;

pub fn get_time() -> usize {
    time::read()
}

pub fn set_next_trigger() {
    // 设置下一次触发时钟中断的时间，即一个时间片所需的cycles
    set_timer(get_time() + CLOCK_FREQ / TICKS_PER_SEC);
}

pub fn get_time_us() -> usize {
    time::read() / (CLOCK_FREQ / MICRO_PER_SEC)
}
