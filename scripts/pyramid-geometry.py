#!/usr/bin/env python3
"""Write a "window pyramid" geometry file for `bpt forge --geometry`.

    python3 scripts/pyramid-geometry.py SIZE STEP MIN_SIDE > pyramid.toml

Every square whose side is a multiple of STEP (at least MIN_SIDE, at most
SIZE) and whose corner sits on the STEP lattice is a region. That is the
closure rule for a layout built of blocks: any square the blocks can form
is itself a puzzle, so no assembled area is left without rules. The
published 4x6x6 and 4x8x8 are pyramids of two levels (SIZE = 2 * STEP);
9x6x6 is the same family with its four 12x12 squares left out, which is
exactly the kind of "square nobody checks" the rule closes.
"""
import sys

size, step, min_side = (int(a) for a in sys.argv[1:4])
assert size % step == 0 and min_side % step == 0 and min_side >= 4 and step % 2 == 0

print(f"# Window pyramid: size {size}, step {step}, smallest square {min_side}.")
print(f"# Every square on the {step}-lattice with a side that is a multiple of {step} is a region.")
print(f"size = {size}")
for side in range(size, min_side - 1, -step):
    for row in range(0, size - side + 1, step):
        for col in range(0, size - side + 1, step):
            print(f"\n[[regions]]\nrow = {row}\ncol = {col}\nrows = {side}\ncols = {side}")
