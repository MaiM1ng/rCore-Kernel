.altmacro
.macro SAVE_SN n
  sd s\n, (\n+2)*8(a0)
.endm
.macro LOAD_SN n
  ld s\n, (\n+2)*8(a1)
.endm
  .section .text
  .global __switch
__switch:
  # function: switch(current_task, next_task)
  # a0: current_task_ptr
  # a1: next_task_ptr
  ## stage 0: save current task regs
  ### save sp
  sd sp, 8(a0)
  ### save ra
  sd ra, 0(a0)
  .set n, 0
  .rept 12
    SAVE_SN %n
    .set n, n + 1
  .endr

  ## stage 1: restore next task regs
  ### restore ra
  ld ra, 0(a1)
  .set n, 0
  .rept 12
    LOAD_SN %n
    .set n, n + 1
  .endr
  ### restore sp
  ld sp, 8(a1)

  # no return val
  # 此时将a1指向的next_task_ptr的trap流快照恢复到CPU中
  # 控制流是next_task_ptr所指向任务了

  ret
