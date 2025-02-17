RCORE_TUTORIAL_DIR := ../rCore-Tutorial-Code-2024S

BUILD_MODE = release
TARGET := riscv64gc-unknown-none-elf
BUILD_DIR := target/$(TARGET)/$(BUILD_MODE)

#FS
FS_IMG := ../rCore-2024A/user/target/$(TARGET)/$(BUILD_MODE)/fs.img

# build
OS_EXEC := os
OS_BIN := $(OS_EXEC).bin
OS_ENTRY_ADDR := 0x80200000
BUILD_FLAGS := 
ifeq ($(BUILD_MODE), release)
	BUILD_FLAGS += --release
endif

# strip metadata
OBJCPY := rust-objcopy
OBJCPY_FLAGS := --strip-all \
								-O binary \

# QEMU
BOOTLOADER := $(RCORE_TUTORIAL_DIR)/bootloader/rustsbi-qemu.bin
QEMU := qemu-system-riscv64
QEMU_FLAGS := -machine virt \
							-nographic \
							-bios $(BOOTLOADER) \
							-device loader,file=$(BUILD_DIR)/$(OS_BIN),addr=$(OS_ENTRY_ADDR) \
							-drive file=$(FS_IMG),if=none,format=raw,id=x0 \
							-device virtio-blk-device,drive=x0,bus=virtio-mmio-bus.0 \

# GDB
GDB := riscv64-unknown-elf-gdb
GDB_FLAGS := -ex 'file $(BUILD_DIR)/$(OS_EXEC)' \
						 -ex 'set arch riscv:rv64' \
						 -ex 'target remote localhost:1234' \

# LOG
LOG ?= info

BASE ?= 1
TEST ?= 0
CHAPTER ?= 0

CARGO_FLAGS := LOG=$(LOG)

all: build

kernel:
	@make -C ../rCore-Tutorial-Code-2024S/user build TEST=$(TEST) CHAPTER=$(CHAPTER) BASE=$(BASE)


build: kernel
	@$(CARGO_FLAGS) cargo build $(BUILD_FLAGS)
	$(OBJCPY) $(OBJCPY_FLAGS) $(BUILD_DIR)/$(OS_EXEC) $(BUILD_DIR)/$(OS_BIN)

run: $(BUILD_DIR)/$(OS_BIN)
	$(QEMU) $(QEMU_FLAGS)

server: $(BUILD_DIR)/$(OS_BIN)
	$(QEMU) $(QEMU_FLAGS) -s -S

telnet: $(BUILD_DIR)/$(OS_EXEC)
	$(GDB) $(GDB_FLAGS)

clean:
	@cargo clean

$(BUILD_DIR)/$(OS_EXEC): $(shell find src -name '*.rs')
	@$(MAKE) build

$(BUILD_DIR)/$(OS_BIN): $(shell find src -name '*.rs')
	@$(MAKE) build

disasm:
	riscv64-unknown-elf-objdump -d $(BUILD_DIR)/$(OS_EXEC) | nvim


.PHONY: build run server telnet clean
