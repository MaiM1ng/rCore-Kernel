.altmacro

// 参数: n
// 64位 sd
.macro SAVE_GP n
  sd x\n, \n*8(sp)
.endm
.macro LOAD_GP n
  ld x\n, \n*8(sp)
.endm
  .section .text
  .global __alltraps
  .global __restore
  .align 2
__alltraps:
  // swap sp, sscratch
  // sp -> user stack
  // sscratch -> kernel stack
  csrrw sp, sscratch, sp
  // now 
  // sp -> kernel stack
  // sscratch -> user stack
 
  // 32 regs + sp + sstatus
  // allocate TrapContext on Kernel Stack
  addi sp, sp, -34 * 8
 
  // x0 == 0
  sd x1, 1 * 8(sp)
  // skip 2
  sd x3, 3 * 8(sp)
  // skip 4

  .set n, 5
  .rept 27
    SAVE_GP %n
    .set n, n + 1
  .endr

  // 此时 t寄存器已被保存 可以使用t寄存器保存CSR
  // 这一步以后才可以保存sp

  csrr t0, sstatus
  csrr t1, sepc
  sd t0, 32*8(sp)
  sd t1, 33*8(sp)

  // save user kernel
  csrr t2, sscratch
  // save to x2(sp)
  sd t2, 2 * 8(sp)
 
  // trap_handler args: TrapContext addr : sp
  mv a0, sp

  call trap_handler

// trap_handler在这里返回
// 返回值为 a0
// 马上进入__restore

// 特殊情况: 第一次执行__restore
// 如果是第一次执行__restore，push_context函数在Kernel Stack上分配了一个应用程序上下文
// 并且返回这个上下文的首地址，那么此时返回值a0就是这个地址，也就是返回值a0指向了Kernel stack
// 且在init_context过程中，上下文中的sp = UserKernel
// 第一次sscratch并不是user stack
// 恢复完以后 sscratch 指向了user stack, 然后__restore会交换sp = user stack， sscratch = Kernel stack。

__restore:
  # mv sp, a0
  # now sp -> kernel stack
  # sscratch -> user stack
  # 如果是第一次指向，那么此时sscratch的值未定义
  ld t0, 32 * 8(sp)
  ld t1, 33 * 8(sp)
  ld t2, 2  * 8(sp)

  # 先使用t寄存器恢复CSR，然后恢复t寄存器

  csrw sstatus, t0
  csrw sepc, t1
  // 此时 sscratch 的值指向user stack
  csrw sscratch, t2

  ld x1, 1 * 8(sp)
  ld x3, 3 * 8(sp)
  .set n, 5
  .rept 27
    LOAD_GP %n
    .set n, n + 1
  .endr

  addi sp, sp, 34 * 8
  
  csrrw sp, sscratch, sp
  # now sp -> user stack
  # sscratch -> kernel stack
  sret
