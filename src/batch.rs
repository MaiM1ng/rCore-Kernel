use crate::trap::TrapContext;
use crate::{sbi::shutdown, sync::UPSafeCell};
use core::arch::asm;
use lazy_static::*;

const MAX_APP_NUM: usize = 16;
const APP_BASE_ADDRESS: usize = 0x80400000;
const APP_SIZE_LIMIT: usize = 0x20000;

// Stack
const USER_STACK_SIZE: usize = 4096 * 2;
const KERNEL_STACK_SIZE: usize = 4096 * 2;

#[repr(align(4096))]
struct KernelStack {
    data: [u8; KERNEL_STACK_SIZE],
}

#[repr(align(4096))]
struct UserStack {
    data: [u8; USER_STACK_SIZE],
}

// 静态全局变量 bss section
static KERNEL_STACK: KernelStack = KernelStack {
    data: [0; KERNEL_STACK_SIZE],
};

static USER_STACK: UserStack = UserStack {
    data: [0; USER_STACK_SIZE],
};

impl UserStack {
    // 获取栈顶
    fn get_sp(&self) -> usize {
        self.data.as_ptr() as usize + USER_STACK_SIZE
    }
}

impl KernelStack {
    // 获取栈顶
    fn get_sp(&self) -> usize {
        self.data.as_ptr() as usize + KERNEL_STACK_SIZE
    }

    pub fn push_context(&self, cx: TrapContext) -> &'static mut TrapContext {
        let cx_ptr = (self.get_sp() - core::mem::size_of::<TrapContext>()) as *mut TrapContext;
        unsafe {
            *cx_ptr = cx;
        }
        unsafe { cx_ptr.as_mut().unwrap() }
    }
}

struct AppManager {
    num_app: usize,
    current_app: usize,
    app_start: [usize; MAX_APP_NUM + 1],
}

lazy_static! {
    static ref APP_MANAGER: UPSafeCell<AppManager> = unsafe {
        UPSafeCell::new({
            // 定义符号
            extern "C" {
                fn _num_app();
            }

            // 转换为指针
            let num_app_ptr = _num_app as usize as *const usize;
            // 读取num_app
            let num_app = num_app_ptr.read_volatile();
            // 声明list
            let mut app_start: [usize; MAX_APP_NUM + 1] = [0; MAX_APP_NUM + 1];
            // 从num_app_ptr + 1的位置开始读取 num_app + 1个元素
            // 实际上也读取了最后一个app的end地址
            let app_start_raw: &[usize] =
                core::slice::from_raw_parts(num_app_ptr.add(1), num_app + 1);

            app_start[0..=num_app].copy_from_slice(app_start_raw);

            AppManager {
                num_app,
                current_app: 0,
                app_start,
            }
        })
    };
}

// 实现AppManager 方法
impl AppManager {
    pub fn print_app_info(&self) {
        println!("[Kernel] num_app = {}", self.num_app);
        for i in 0..self.num_app {
            println!(
                "[Kernel] app_{} [{:#x} {:#x})",
                i,
                self.app_start[i],
                self.app_start[i + 1]
            );
        }
    }

    unsafe fn load_app(&self, app_id: usize) {
        if app_id >= self.num_app {
            println!("[Kernel] All Application completed!");
            shutdown(false);
        }

        println!("[Kernel] Loading app_{}", app_id);

        // 清空
        core::slice::from_raw_parts_mut(APP_BASE_ADDRESS as *mut u8, APP_SIZE_LIMIT).fill(0);

        let app_src = core::slice::from_raw_parts(
            self.app_start[app_id] as *const u8,
            self.app_start[app_id + 1] - self.app_start[app_id],
        );

        let app_dst = core::slice::from_raw_parts_mut(APP_BASE_ADDRESS as *mut u8, app_src.len());
        app_dst.copy_from_slice(app_src);

        // invalid i-cache
        asm!("fence.i");
    }

    pub fn get_current_app(&self) -> usize {
        self.current_app
    }

    pub fn move_to_next_app(&mut self) {
        self.current_app += 1;
    }
}

pub fn init() {
    print_app_info();
}

pub fn print_app_info() {
    APP_MANAGER.exclusive_access().print_app_info();
}

pub fn run_next_app() -> ! {
    // 构造异常返回
    let mut app_manager = APP_MANAGER.exclusive_access();
    let current_app = app_manager.get_current_app();

    unsafe {
        app_manager.load_app(current_app);
    }
    app_manager.move_to_next_app();

    drop(app_manager);

    extern "C" {
        fn __restore(cx_addr: usize);
    }

    unsafe {
        // push_context的返回值是一个地址，指向此时的Kernel Stack，但是此时的Kernel
        // Stack上存在着一个构建好的应用上下文
        __restore(KERNEL_STACK.push_context(TrapContext::app_init_context(
            APP_BASE_ADDRESS,
            USER_STACK.get_sp(),
        )) as *const _ as usize);
    }

    panic!("Unreachable in batch::run_current_app!");
}

pub fn get_user_stack_sp_space() -> (usize, usize) {
    (USER_STACK.get_sp(), USER_STACK.get_sp() + USER_STACK_SIZE)
}

pub fn get_app_address_space() -> (usize, usize) {
    (APP_BASE_ADDRESS, APP_BASE_ADDRESS + APP_SIZE_LIMIT)
}
