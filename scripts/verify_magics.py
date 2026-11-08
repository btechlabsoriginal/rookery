"""Independently verify the embedded sliding magic constants.

Runs without Rust or third-party dependencies. The canonical exhaustive check
also lives in the engine's Rust unit tests.
"""
from pathlib import Path
import re

SOURCE = Path(__file__).resolve().parents[1] / "engine/src/attacks/magic_numbers.rs"
DATA = SOURCE.read_text()
MASK64 = (1 << 64) - 1
ROOK = ((1, 0), (-1, 0), (0, 1), (0, -1))
BISHOP = ((1, 1), (1, -1), (-1, 1), (-1, -1))


def rays(square, diagonal, blockers, exclude_edge=False):
    result = 0
    for df, dr in BISHOP if diagonal else ROOK:
        file, rank = square % 8 + df, square // 8 + dr
        while 0 <= file < 8 and 0 <= rank < 8:
            if exclude_edge and not (0 <= file + df < 8 and 0 <= rank + dr < 8):
                break
            bit = 1 << (8 * rank + file)
            result |= bit
            if blockers & bit:
                break
            file += df
            rank += dr
    return result


def read_magics(name):
    pattern = rf"pub const {name}_MAGICS: \[u64; 64\] = \[(.*?)\];"
    match = re.search(pattern, DATA, re.DOTALL)
    if match is None:
        raise AssertionError(f"Missing {name} constants")
    values = [int(n, 16) for n in re.findall(r"0x[0-9a-f]+", match.group(1))]
    assert len(values) == 64
    return values


def check(name, diagonal):
    numbers = read_magics(name)
    checked = 0
    for square, magic in enumerate(numbers):
        mask = rays(square, diagonal, 0, exclude_edge=True)
        bits = [1 << s for s in range(64) if mask & (1 << s)]
        buckets = {}
        for subset in range(1 << len(bits)):
            blockers = sum(bit for i, bit in enumerate(bits) if subset & (1 << i))
            index = ((blockers * magic) & MASK64) >> (64 - len(bits))
            actual = rays(square, diagonal, blockers)
            if index in buckets:
                assert buckets[index] == actual, (name, square, subset, index)
            buckets[index] = actual
            checked += 1
    print(f"{name.lower():6s}: verified {checked:,} blocker occupancies across 64 squares")


if __name__ == "__main__":
    check("BISHOP", True)
    check("ROOK", False)
