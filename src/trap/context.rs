use riscv::register::sstatus::{self, Sstatus, SPP};

#[repr(C)]
pub struct TrapContext {
    // gp regs
    pub x: [usize; 32],
    // 嵌套 trap
    pub sstatus: Sstatus,
    pub sepc: usize,
}

impl TrapContext {
    pub fn set_sp(&mut self, sp: usize) {
        self.x[2] = sp;
    }

    pub fn app_init_context(entry: usize, sp: usize) -> Self {
        // spp表示进入到s模式之前的特权等级
        // spp: 0 U模式
        // spp: 1 S模式
        let mut sstatus = sstatus::read();
        sstatus.set_spp(SPP::User);
        let mut cx = Self {
            x: [0; 32],
            sstatus,
            sepc: entry,
        };

        cx.set_sp(sp);

        cx
    }
}
