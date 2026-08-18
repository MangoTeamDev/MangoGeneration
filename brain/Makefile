# Makefile для сборки Мозга MangoGeneration (brain.c)
#
#   make            — сборка для текущей (нативной) платформы
#   make windows    — кросс-сборка для Windows (нужен mingw-w64)
#   make clean      — удалить артефакты

CC      ?= cc
WINCC   ?= x86_64-w64-mingw32-gcc

SRC     := brain.c
CFLAGS  := -std=c11 -O2 -Wall -Wextra
LDLIBS  := -lm

OUT     := brain
WINOUT  := brain.exe

.PHONY: all windows clean

all: $(OUT)

$(OUT): $(SRC)
	$(CC) $(CFLAGS) -o $@ $(SRC) $(LDLIBS)

windows: $(WINOUT)

$(WINOUT): $(SRC)
	$(WINCC) $(CFLAGS) -o $@ $(SRC)

clean:
	rm -f $(OUT) $(WINOUT)