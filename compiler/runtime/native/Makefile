# Makefile for Reml Runtime (Phase 1)
#
# ビルドターゲット:
#   make runtime     - ランタイムライブラリをビルド (libreml_runtime.a)
#   make test        - テストをビルドして実行
#   make clean       - ビルド成果物を削除
#   make install     - ランタイムをインストール (未実装)

# ========== 変数定義 ==========

CC = clang
AR = ar
RANLIB =
CFLAGS = -Wall -Wextra -std=c11 -O2 -g
LDFLAGS =
INCLUDES = -Iinclude
CROSS ?= 0
CROSS_PREFIX ?= x86_64-unknown-linux-gnu
CROSS_SYSROOT ?=

ifeq ($(CROSS),1)
CC = $(CROSS_PREFIX)-clang
CFLAGS += --target=$(CROSS_PREFIX)
LDFLAGS += --target=$(CROSS_PREFIX)

AR_CANDIDATE := $(shell command -v $(CROSS_PREFIX)-ar 2>/dev/null)
ifeq ($(strip $(AR_CANDIDATE)),)
AR = llvm-ar
else
AR = $(CROSS_PREFIX)-ar
endif

RANLIB ?= ranlib
RANLIB_CANDIDATE := $(shell command -v $(CROSS_PREFIX)-ranlib 2>/dev/null)
ifneq ($(strip $(RANLIB_CANDIDATE)),)
RANLIB = $(CROSS_PREFIX)-ranlib
endif

ifneq ($(strip $(CROSS_SYSROOT)),)
CFLAGS += --sysroot=$(CROSS_SYSROOT)
LDFLAGS += --sysroot=$(CROSS_SYSROOT)
endif
endif

# macOS SDK パスを追加（必要に応じて）
UNAME_S := $(shell uname -s)
ifeq ($(UNAME_S),Darwin)
SDK_PATH := $(shell xcrun --show-sdk-path 2>/dev/null)
ifneq ($(SDK_PATH),)
CFLAGS += -isysroot $(SDK_PATH)
LDFLAGS += -isysroot $(SDK_PATH) -L$(SDK_PATH)/usr/lib
endif
endif

# デバッグビルドの場合は DEBUG マクロを定義
ifdef DEBUG
CFLAGS += -DDEBUG -fsanitize=address -fno-omit-frame-pointer
LDFLAGS += -fsanitize=address
endif

# ソースファイルとオブジェクトファイル
SRC_DIR = src
OBJ_DIR = build
TEST_DIR = tests

SOURCES = $(wildcard $(SRC_DIR)/*.c)
OBJECTS = $(patsubst $(SRC_DIR)/%.c,$(OBJ_DIR)/%.o,$(SOURCES))

RUNTIME_LIB = $(OBJ_DIR)/libreml_runtime.a

TEST_SOURCES = $(wildcard $(TEST_DIR)/*.c)
TEST_BINARIES = $(patsubst $(TEST_DIR)/%.c,$(OBJ_DIR)/%,$(TEST_SOURCES))

# ========== ビルドルール ==========

.PHONY: all runtime test clean install

all: runtime

# ランタイムライブラリのビルド
runtime: $(RUNTIME_LIB)

$(RUNTIME_LIB): $(OBJECTS)
	@mkdir -p $(OBJ_DIR)
	$(AR) rcs $@ $^
ifdef RANLIB
	$(RANLIB) $@
endif
	@echo "Built runtime library: $@"

# オブジェクトファイルのコンパイル
$(OBJ_DIR)/%.o: $(SRC_DIR)/%.c
	@mkdir -p $(OBJ_DIR)
	$(CC) $(CFLAGS) $(INCLUDES) -c $< -o $@

# テストのビルドと実行
test: $(TEST_BINARIES)
	@echo "Running tests..."
	@for test in $(TEST_BINARIES); do \
		echo ""; \
		echo "Executing $$test..."; \
		$$test || exit 1; \
	done
	@echo ""
	@echo "All tests passed!"

$(OBJ_DIR)/%: $(TEST_DIR)/%.c $(RUNTIME_LIB)
	@mkdir -p $(OBJ_DIR)
	$(CC) $(CFLAGS) $(INCLUDES) $< $(RUNTIME_LIB) $(LDFLAGS) -o $@

# クリーン
clean:
	rm -rf $(OBJ_DIR)

# インストール（Phase 2 以降で実装）
install:
	@echo "Install target not yet implemented"
	@exit 1

# ========== ヘルプ ==========

help:
	@echo "Reml Runtime Build System"
	@echo ""
	@echo "Targets:"
	@echo "  make runtime     - Build runtime library (libreml_runtime.a)"
	@echo "  make test        - Build and run tests"
	@echo "  make clean       - Remove build artifacts"
	@echo "  make install     - Install runtime (not yet implemented)"
	@echo ""
	@echo "Options:"
	@echo "  DEBUG=1          - Enable debug build with ASan"
	@echo ""
	@echo "Examples:"
	@echo "  make runtime"
	@echo "  make test DEBUG=1"
