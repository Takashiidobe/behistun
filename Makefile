CC := m68k-unknown-linux-uclibc-gcc
CFLAGS := -static -O0 -msoft-float -lm
CSMITH_CFLAGS := $(CFLAGS) -I/usr/include/csmith-2.3.0/
CLANG_FORMAT ?= clang-format

# Use all but 1 thread to speed up compilation
MAKEFLAGS += -j $(shell expr $(shell nproc) - 1)

# Old bins for examples (keeping for backward compatibility)
ASM_SRCS := $(wildcard examples/asm/*.S)
C_SRCS := $(wildcard examples/c/*.c)
ASM_BINS := $(patsubst examples/asm/%.S,bins/%,$(ASM_SRCS))
C_BINS := $(patsubst examples/c/%.c,bins/%,$(C_SRCS))
BINS := $(ASM_BINS) $(C_BINS)

# Test binaries - use find to handle subdirectories
TEST_ASM_SRCS := $(shell find test-files/asm -name '*.S' 2>/dev/null)
TEST_C_SRCS := $(shell find test-files/c -name '*.c' 2>/dev/null)
CSMITH_SRCS := $(shell find test-csmith -name '*.c' 2>/dev/null)
INTEGRATION_ASM_SRCS := $(shell find test-integration/asm -name '*.S' 2>/dev/null)
INTEGRATION_C_SRCS := $(shell find test-integration/c -name '*.c' 2>/dev/null)

# Files to format with clang format
FMT_FILES := $(shell find test-files examples -type f -name '*.c' 2>/dev/null)

# Preserve directory structure: test-files/c/syscalls/foo.c -> test-bins/c/syscalls/foo
TEST_ASM_BINS := $(patsubst test-files/asm/%.S,test-bins/asm/%,$(TEST_ASM_SRCS))
TEST_C_BINS := $(patsubst test-files/c/%.c,test-bins/c/%,$(TEST_C_SRCS))
CSMITH_BINS := $(patsubst test-csmith/%.c,test-bins/csmith/%,$(CSMITH_SRCS))
INTEGRATION_ASM_BINS := $(patsubst test-integration/asm/%.S,test-bins/integration/asm/%,$(INTEGRATION_ASM_SRCS))
INTEGRATION_C_BINS := $(patsubst test-integration/c/%.c,test-bins/integration/c/%,$(INTEGRATION_C_SRCS))

TEST_BINS := $(TEST_ASM_BINS) $(TEST_C_BINS)
INTEGRATION_BINS := $(INTEGRATION_ASM_BINS) $(INTEGRATION_C_BINS)

.PHONY: all clean test-bins test-csmith-bins test-integration-bins

all: $(BINS)

# Old bins rules (keeping for backward compatibility)
bins/%: examples/asm/%.S | bins
	$(CC) $(CFLAGS) -nostdlib -o $@ $<

bins/%: examples/c/%.c | bins
	$(CC) $(CFLAGS) -o $@ $<

bins:
	mkdir -p $@

# Test binaries rules
test-bins: $(TEST_BINS)

test-csmith-bins: $(CSMITH_BINS)

test-integration-bins: $(INTEGRATION_BINS)

# Pattern rule for assembly tests - creates subdirs as needed
test-bins/asm/%: test-files/asm/%.S
	@mkdir -p $(dir $@)
	$(CC) $(CFLAGS) -nostdlib -o $@ $<

# Pattern rule for C tests - creates subdirs as needed
test-bins/c/%: test-files/c/%.c
	@mkdir -p $(dir $@)
	$(CC) $(CFLAGS) -o $@ $<

# Pattern rule for csmith tests - creates subdirs as needed
test-bins/csmith/%: test-csmith/%.c
	@mkdir -p $(dir $@)
	$(CC) $(CSMITH_CFLAGS) -o $@ $<

# Pattern rule for integration assembly tests - creates subdirs as needed
test-bins/integration/asm/%: test-integration/asm/%.S
	@mkdir -p $(dir $@)
	$(CC) $(CFLAGS) -nostdlib -o $@ $<

# Pattern rule for integration C tests - creates subdirs as needed
test-bins/integration/c/%: test-integration/c/%.c
	@mkdir -p $(dir $@)
	$(CC) $(CFLAGS) -o $@ $<

fmt:
	$(CLANG_FORMAT) -i $(FMT_FILES)

fmt-check:
	$(CLANG_FORMAT) --dry-run --Werror $(FMT_FILES)

clean:
	rm -rf bins test-bins
