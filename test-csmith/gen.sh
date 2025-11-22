#!/usr/bin/env bash

for i in {501..550}; do
  csmith --max-funcs 2 \
         --max-expr-complexity 1 \
         --max-block-depth 2 \
    > "csmith_${i}.c"
done
