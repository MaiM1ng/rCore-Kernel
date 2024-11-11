.altmacro
.macro SAVE_GP n
  sd x\n, \n*8(sp)
.endm
.macro LOAD_GP n
  ld x\n, \n*8(sp)
.endm
  .section .text.trampoline
  .global __alltraps
  .global __restore
  .align 2
__alltraps:
  # 交换之前
  # sscratch: trap context的位置
  # sp: user stack
  csrrw sp, sscratch, sp
  # 交换以后
  # sscratch: user stack
  # sp: trap context的位置
  sd x1, 1*8(sp)
  # 此时user stack的指针在sscratch寄存器上
  # 但是保存sscratch需要先保存t寄存器
  sd x3, 3*8(sp)

  .set n, 5
  .rept 27
    SAVE_GP %n
    .set n, n + 1
  .endr
  
  csrr t0, sstatus
  csrr t1, sepc
  sd t0, 32*8(sp)
  sd t1, 33*8(sp)
  # 读取用户栈保存到TrapContext上
  csrr t2, sscratch
  sd t2, 2*8(sp)

  # load kernel_trap into t0
  ld t0, 34*8(sp)
  # load trap_handler into t1
  ld t1, 36*8(sp)
  # move to kernel sp
  ld sp, 35*8(sp)

  # switch to kernel address space
  csrw satp, t0
  sfence.vma

  # call trap_handler
  # call指令是相对PC寻址，但是开启VA以后不成立
  jr t1
__restore:
  # a0: *TrapContext in user space
  # a1：user space token
  
  ## 先切换地址空间回user，与__alltraps对应
  csrw satp, a1
  sfence.vma

  # 将TrapContext的地址写入sscratch
  csrw sscratch, a0
  # 移动到sp上
  mv sp, a0
  # 此时sp指向用户空间的TrapContext上下文

  ld t0, 32*8(sp)
  ld t1, 33*8(sp)
  csrw sstatus, t0
  csrw sepc, t1

  ld x1, 1*8(sp)
  ld x3, 3*8(sp)
  .set n, 5
  .rept 27
    LOAD_GP %n
    .set n, n + 1
  .endr

  # 切换到用户栈
  ld sp, 2*8(sp)
  sret
