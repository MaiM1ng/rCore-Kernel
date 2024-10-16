// 实现Clon和Copy特型
// 按照C语言内存布局，供switch函数调用
#[derive(Clone, Copy)]
#[repr(C)]
pub struct TaskContext {
    // TODO: 为什么要保存ra
    ra: usize,
    sp: usize,
    // s寄存器在rv调用约定中是被调用者保存的。
    // 由于__switch函数是自己编写的，rust编译器不会对__switch内的函数进行保存
    // 当task管理器调用__switch进行任务切换的时候，在调用前，编译器会对t寄存器进行保存，因此不用处理t寄存器
    // 进入到__switch以后，s寄存器需要在__switch中手动保存。
    // 相当于手动对s寄存器做了一次保存，而t寄存器已经被报错了，而且对于__switch函数来说
    // 也没有使用t寄存器，所以也无需保存
    s: [usize; 12],
}

impl TaskContext {
    pub fn zero_init() -> Self {
        Self {
            ra: 0,
            sp: 0,
            s: [0; 12],
        }
    }

    pub fn goto_restore(kstack_ptr: usize) -> Self {
        extern "C" {
            fn __restore();
        }

        Self {
            // ra作为返回值
            ra: __restore as usize,
            sp: kstack_ptr,
            s: [0; 12],
        }
    }
}
