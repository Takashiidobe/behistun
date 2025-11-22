#!/bin/bash

m68k-unknown-linux-gnu-gcc test.S --static -O0 -o test -nostdlib
m68k-unknown-linux-gnu-objdump -D ./test
