mod context;
mod switch;
mod task;

use crate::loader::get_num_app;
use crate::sync::UPSafeCell;
use crate::{config::*, loader::init_app_cx};
use context::TaskContext;
use lazy_static::lazy_static;
use switch::__switch;
use task::{TaskControlBlock, TaskStatus};

pub struct TaskManager {
    num_app: usize,
    inner: UPSafeCell<TaskManagerInner>,
}

struct TaskManagerInner {
    tasks: [TaskControlBlock; MAX_APP_NUM],
    current_task: usize,
}

lazy_static! {
    pub static ref TASK_MANAGER: TaskManager = {
        let num_app = get_num_app();
        let mut tasks = [TaskControlBlock {
            task_cx: TaskContext::zero_init(),
            task_status: TaskStatus::UnInit,
        }; MAX_APP_NUM];

        for i in 0..num_app {
            // 构造第一次进入的TaskContext
            // init_app_cx 返回内核栈上下文的地址，将这个地址通过goto_restore函数传入到sp寄存器
            // __switch函数ret以后会进入__restore函数
            tasks[i].task_cx = TaskContext::goto_restore(init_app_cx(i));
            tasks[i].task_status = TaskStatus::Ready;
        }

        TaskManager {
            num_app,
            inner: unsafe {
                UPSafeCell::new(TaskManagerInner{
                    tasks,
                    current_task: 0,
                })
            }
        }
    };
}

impl TaskManager {
    fn mark_current_suspended(&self) {
        let mut inner = self.inner.exclusive_access();
        let current = inner.current_task;
        // 切换到Ready态，代表任务暂停，但可以随时执行
        inner.tasks[current].task_status = TaskStatus::Ready;
    }

    fn mark_current_exit(&self) {
        let mut inner = self.inner.exclusive_access();
        let current = inner.current_task;
        // 将任务设置为Exited态，代表任务退出
        inner.tasks[current].task_status = TaskStatus::Exited;
    }

    fn run_next_task(&self) {
        // find函数返回一个Option类型的值，如果不为空，Some对其解包
        if let Some(next) = self.find_next_task() {
            let mut inner = self.inner.exclusive_access();
            let current = inner.current_task;
            // 设置下一个的状态为运行态
            inner.tasks[next].task_status = TaskStatus::Running;
            inner.current_task = next;

            // 获取任务上下文指针
            let current_task_cx_ptr = &mut inner.tasks[current].task_cx as *mut TaskContext;
            let next_task_cx_ptr = &mut inner.tasks[next].task_cx as *const TaskContext;

            // 此处必须手动drop inner
            // 所有权此时保留在当前的上下文上，切换出了将无法自动释放，因此需要手动释放。
            drop(inner);
            unsafe { __switch(current_task_cx_ptr, next_task_cx_ptr) };
        } else {
            panic!("All applications completed!");
        }
    }

    fn find_next_task(&self) -> Option<usize> {
        let inner = self.inner.exclusive_access();
        let current = inner.current_task;
        // 从current + 1 到 current + num_app + 1搜索一圈
        (current + 1..current + 1 + self.num_app)
            .map(|id| id % self.num_app)
            .find(|id| inner.tasks[*id].task_status == TaskStatus::Ready)
    }

    fn run_first_task(&self) -> ! {
        let mut inner = self.inner.exclusive_access();
        let task0 = &mut inner.tasks[0];
        task0.task_status = TaskStatus::Running;
        let next_task_cx_ptr = &task0.task_cx as *const TaskContext;

        drop(inner);
        let mut _unused = TaskContext::zero_init();
        unsafe {
            __switch(&mut _unused, next_task_cx_ptr);
        }
        panic!("Unreachable in run_first_task!");
    }

    fn get_current_task(&self) -> usize {
        let inner = self.inner.exclusive_access();
        inner.current_task
    }
}

pub fn suspend_current_and_run_next() {
    mark_current_suspended();
    run_next_task();
}

pub fn exit_current_and_run_next() {
    mark_current_exited();
    run_next_task();
}

fn mark_current_suspended() {
    TASK_MANAGER.mark_current_suspended();
}

fn mark_current_exited() {
    TASK_MANAGER.mark_current_exit();
}

fn run_next_task() {
    TASK_MANAGER.run_next_task();
}

pub fn run_first_task() {
    TASK_MANAGER.run_first_task();
}

// 这里需要知道当前是哪个app发起的syscall
pub fn get_current_task_id() -> usize {
    TASK_MANAGER.get_current_task()
}
