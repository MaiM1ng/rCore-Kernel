use riscv::register::sstatus::{self, Sstatus, SPP};

#[repr(C)]
#[derive(Debug)]
/// trap context structure containing sstatus, sepc and register
pub struct TrapContext {
    /// general-purpose register
    pub x: [usize; 32],
    /// supervisor status register
    pub sstatus: Sstatus,
    /// supervisor exception program counter
    pub sepc: usize,
    /// kernel satp
    pub kernel_satp: usize,
    /// kernel sp
    pub kernel_sp: usize,
    /// entry of trap handler
    pub trap_handler: usize,
}

impl TrapContext {
    /// set sp
    pub fn set_sp(&mut self, sp: usize) {
        self.x[2] = sp;
    }

    /// init the trap context of an application
    pub fn app_init_context(
        entry: usize,
        sp: usize,
        kernel_satp: usize,
        kernel_sp: usize,
        trap_handler: usize,
    ) -> Self {
        let mut sstatus = sstatus::read();
        // 设置返回地址
        sstatus.set_spp(SPP::User);
        let mut cx = Self {
            x: [0; 32],
            sstatus,
            sepc: entry,
            kernel_satp,
            kernel_sp,
            trap_handler,
        };

        cx.set_sp(sp);

        cx
    }
}
