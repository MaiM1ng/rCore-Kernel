use crate::config::CLOCK_FREQ;
use crate::sbi::set_timer;
use riscv::register::time;

// 1s之内的时间片数量
const TICKS_PER_SEC: usize = 100;
const MICRO_PER_SEC: usize = 1_000_000;
const MSEC_PER_SEC: usize = 1000;

pub fn get_time() -> usize {
    time::read()
}

pub fn set_next_trigger() {
    // 设置下一次触发时钟中断的时间，即一个时间片所需的cycles
    set_timer(get_time() + CLOCK_FREQ / TICKS_PER_SEC);
}

pub fn get_time_us() -> usize {
    assert_eq!(MICRO_PER_SEC, 1000000);
    assert_eq!(CLOCK_FREQ, 12500000);
    // println!("MICRO {} CLOCK FREQ {} fuck", MICRO_PER_SEC, CLOCK_FREQ);
    time::read() * MICRO_PER_SEC / CLOCK_FREQ
}

pub fn get_time_ms() -> usize {
    time::read() * MSEC_PER_SEC / CLOCK_FREQ
}
