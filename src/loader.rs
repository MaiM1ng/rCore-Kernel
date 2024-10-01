use crate::config::*;
use crate::trap::TrapContext;
use core::arch::asm;

use crate::sbi::shutdown;
use crate::sync::UPSafeCell;
use lazy_static::*;

// 当作为数组中的元素是，需要实现copy和clone trait， UserStack同理
#[repr(align(4096))]
#[derive(Clone, Copy)]
struct KernelStack {
    data: [u8; KERNEL_STACK_SIZE],
}

#[repr(align(4096))]
#[derive(Clone, Copy)]
struct UserStack {
    data: [u8; USER_STACK_SIZE],
}

// 相比于Batch System
// 多任务要为每个app维护内核栈和用户栈
static KERNEL_STACK: [KernelStack; MAX_APP_NUM] = [KernelStack {
    data: [0; KERNEL_STACK_SIZE],
}; MAX_APP_NUM];

static USER_STACK: [UserStack; MAX_APP_NUM] = [UserStack {
    data: [0; USER_STACK_SIZE],
}; MAX_APP_NUM];

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

    // 相应的 返回值从Batch System发生了变化
    pub fn push_context(&self, cx: TrapContext) -> usize {
        // 在栈上分配一块空间
        let cx_ptr = (self.get_sp() - core::mem::size_of::<TrapContext>()) as *mut TrapContext;
        unsafe {
            // 数据复制到栈上
            *cx_ptr = cx;
        }
        cx_ptr as usize
    }
}

// batch System
struct AppManger {
    num_app: usize,
    current_app: usize,
}

impl AppManger {
    pub fn get_current_app(&self) -> usize {
        self.current_app
    }

    pub fn move_to_next_app(&mut self) {
        self.current_app += 1;
    }
}

lazy_static! {
    static ref APP_MANAGER: UPSafeCell<AppManger> = unsafe {
        UPSafeCell::new({
            extern "C" {
                fn _num_app();
            }

            let num_app_ptr = _num_app as usize as *const usize;
            let num_app = num_app_ptr.read_volatile();

            AppManger {
                num_app: num_app,
                current_app: 0,
            }
        })
    };
}

// 计算app i在内存中相应的位置
fn get_base_i(app_id: usize) -> usize {
    APP_BASE_ADDRESS + app_id * APP_SIZE_LIMIT
}

// 获取app数量
pub fn get_num_app() -> usize {
    extern "C" {
        fn _num_app();
    }
    unsafe { (_num_app as usize as *const usize).read_volatile() }
}

pub fn load_apps() {
    extern "C" {
        fn _num_app();
    }
    let num_app_ptr = _num_app as usize as *const usize;
    let num_app = get_num_app();
    println!("[Kernel] load {} apps", num_app);

    let app_start = unsafe {
        // 这里包括最后一个app的end
        core::slice::from_raw_parts(num_app_ptr.add(1), num_app + 1)
    };

    for (i, &app) in app_start.iter().enumerate() {
        println!("[Kernel] app_{} image at 0x{:x}", i, app);
    }

    // load apps
    for i in 0..num_app {
        let base_i = get_base_i(i);

        println!(
            "[Kernel] Load app_{} User Stack {:x}, Kernel Stack {:x} Base: {:x}",
            i,
            USER_STACK[i].get_sp(),
            KERNEL_STACK[i].get_sp(),
            base_i
        );

        // clear region
        // 迭代器
        (base_i..base_i + APP_SIZE_LIMIT)
            .for_each(|addr| unsafe { (addr as *mut u8).write_volatile(0) });

        let src = unsafe {
            core::slice::from_raw_parts(app_start[i] as *const u8, app_start[i + 1] - app_start[i])
        };

        let dst = unsafe { core::slice::from_raw_parts_mut(base_i as *mut u8, src.len()) };

        dst.copy_from_slice(src);
    }

    // 清空ICache
    // 正常来说，多道程序的时候没有冲刷icache的必要
    // 但是isa手册规定，最好清空一下
    // 同时可以清空prefetch icache和清空dcache 脏的cacheline
    unsafe {
        asm!("fence.i");
    }
}

pub fn init_app_cx(app_id: usize) -> usize {
    println!(
        "[Kernel] Run app_{} at 0x{:x}, Kernel Stack : {:x}, User Stack : {:x}",
        app_id,
        get_base_i(app_id),
        KERNEL_STACK[app_id].get_sp(),
        USER_STACK[app_id].get_sp()
    );
    KERNEL_STACK[app_id].push_context(TrapContext::app_init_context(
        get_base_i(app_id),
        USER_STACK[app_id].get_sp(),
    ))
}

pub fn run_next_app() -> ! {
    let mut app_manger = APP_MANAGER.exclusive_access();
    let current_app = app_manger.get_current_app();

    app_manger.move_to_next_app();

    if current_app >= app_manger.num_app {
        println!("[Kernel] All application completed!");
        shutdown(false);
    }

    drop(app_manger);

    extern "C" {
        fn __restore(cx_addr: usize);
    }

    unsafe {
        let ctx = init_app_cx(current_app);
        __restore(ctx);
    }

    panic!("unreachable run_next_app")
}

// 这里需要知道当前是哪个app发起的syscall
// 由于执行run_next_app函数后，current_app++了， 这里需要-1
pub fn get_current_app_id() -> usize {
    let app_manager = APP_MANAGER.exclusive_access();
    let app_id = app_manager.get_current_app();
    drop(app_manager);
    app_id - 1
}

pub fn get_user_stack_sp_space(app_id: usize) -> (usize, usize) {
    (
        USER_STACK[app_id].get_sp() - USER_STACK_SIZE,
        USER_STACK[app_id].get_sp(),
    )
}

pub fn get_app_address_space(app_id: usize) -> (usize, usize) {
    (get_base_i(app_id), get_base_i(app_id) + APP_SIZE_LIMIT)
}
