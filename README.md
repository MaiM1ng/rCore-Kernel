# rCore Kernel

## 说明

本仓库是基于rCore教程，使用`rust`从0实现的os kernel。

+ 复用了rCore工程中的user和easyfs，需要将一些目录的配置指向rCore中原本的位置

## 进度

1. ch2: Batch System
2. ch2: sys_write 安全检查
3. ch3: 多道程序与协作调度: `f1e3f4a25d040477025ba745b1ebdc5c982364fa`
4. ch3: 多道程序与抢占式调度: `a7f881838bd569bd2b5088e3a962e77b7c0359f1`
5. ch4: sys_mmap和sys_munmap: e42d1006be3c42bd454377d8d5969f7e4a29fca4
5. ch5: sys_spawn和stride调度算法: 59d234da94d3a93004b892f8dfde8d834212498c
