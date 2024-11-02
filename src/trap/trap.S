.altmacro
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
  # 交换之前
  # sscratch: kernel stack
  # sp: user stack
  csrrw sp, sscratch, sp
  # 交换以后
  # sscratch: user stack
  # sp: kernel stack
  addi sp, sp, -34*8
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
  csrr t2, sscratch
  sd t2, 2*8(sp)

  mv a0, sp
  call trap_handler

__restore:
  ld t0, 32*8(sp)
  ld t1, 33*8(sp)
  ld t2, 2*8(sp)
  csrw sstatus, t0
  csrw sepc, t1
  csrw sscratch, t2

  ld x1, 1*8(sp)
  ld x3, 3*8(sp)
  .set n, 5
  .rept 27
    LOAD_GP %n
    .set n, n + 1
  .endr

  addi sp, sp, 34*8
  # 将sscratch上保存的用户栈与sp指向的内核栈交换
  csrrw sp, sscratch, sp
  sret
