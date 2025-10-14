# Performance Optimization Diary

This document tracks performance improvements made to the Game of Life implementation.

## Baseline Performance

**Date**: 2025-10-15
**Commit**: f10bb54

### Test Configuration
- Grid size: 1000x1000 (1,000,000 cells)
- Iterations: 1000
- Random seed: 12345
- Build: Release mode

### Results
```
Total time: 14.128170126s
Time per iteration: 14.13ms
Iterations per second: 70.78
```

### Implementation Details
- Data structure: `Vec<Vec<CellState>>`
- Grid allocation: New allocation per `advance()` call
- Neighbor counting: Iterative with modulo wrapping

### Identified Optimization Opportunities
1. Avoid allocation in `advance()` - use double buffering
2. Cache grid dimensions as fields
3. Use flat `Vec<CellState>` instead of nested vectors for better cache locality
4. Optimize neighbor counting algorithm
5. Early return optimization for static patterns

---

## Optimization History

### Optimization #1: Double Buffering

**Date**: 2025-10-15

**Change**: Added a `next_cells` buffer to the `Grid` struct to avoid allocating a new grid on every `advance()` call. Instead, we compute the next state into `next_cells` and swap the buffers using `std::mem::swap()`.

**Implementation Details**:
- Added `next_cells: Vec<Vec<CellState>>` field to `Grid` struct
- Modified `new()` to initialize both `cells` and `next_cells`
- Changed `advance()` to write to `next_cells` instead of allocating
- Used `std::mem::swap()` to swap buffers after computation

**Results**:
```
Total time: 13.087865677s
Time per iteration: 13.09ms
Iterations per second: 76.41
```

**Improvement**:
- Time per iteration: 14.13ms → 13.09ms (**7.4% faster**)
- Iterations per second: 70.78 → 76.41 (**7.9% increase**)
- Speedup: 1.08x

---

### Optimization #2: Cache Grid Dimensions

**Date**: 2025-10-15

**Change**: Added `width` and `height` fields to the `Grid` struct to avoid repeatedly calling `.len()` on vectors during grid traversal and neighbor calculations.

**Implementation Details**:
- Added `width: usize` and `height: usize` fields to `Grid` struct
- Modified `new()` to initialize width and height
- Changed `advance()` to use `self.height` and `self.width` instead of `self.cells.len()` and `self.cells[0].len()`
- Updated `alive_neighbors()` to use cached dimensions for modulo wrapping

**Results**:
```
Total time: 13.343082072s
Time per iteration: 13.34ms
Iterations per second: 74.95
```

**Improvement**:
- Time per iteration: 13.09ms → 13.34ms (**1.9% slower**)
- Iterations per second: 76.41 → 74.95 (**1.9% decrease**)
- Speedup: 0.98x

**Analysis**: This optimization actually resulted in a slight performance regression. The overhead of accessing two additional fields appears to outweigh the benefit of avoiding `.len()` calls, which are likely already well-optimized by LLVM. The added struct size may also negatively impact cache performance.

**Status**: ❌ **REVERTED** - This change was rolled back as it decreased performance.

---

### Optimization #3: Flat Vector with Manual Indexing

**Date**: 2025-10-15

**Change**: Converted data structure from `Vec<Vec<CellState>>` to a flat `Vec<CellState>` with manual 2D-to-1D index mapping for better cache locality.

**Implementation Details**:
- Changed `cells` and `next_cells` from `Vec<Vec<CellState>>` to `Vec<CellState>`
- Added `width` and `height` fields to track dimensions (needed for indexing)
- Implemented `index()` helper method: `row * width + col`
- Added `get()` and `set()` public methods for accessing cells
- Updated all grid operations to use flat indexing
- Modified test helpers to use new API

**Results**:
```
Total time: 13.702601309s
Time per iteration: 13.70ms
Iterations per second: 72.98
```

**Improvement**:
- Time per iteration: 13.09ms → 13.70ms (**4.7% slower**)
- Iterations per second: 76.41 → 72.98 (**4.5% decrease**)
- Speedup: 0.95x

**Analysis**: This optimization resulted in a performance regression despite better theoretical cache locality. The overhead of manual index calculation (`row * width + col`) appears to outweigh the cache benefits. The nested Vec structure likely benefits from compiler optimizations and predictable access patterns. Additionally, the cached width/height fields add to the struct size, which may hurt performance as seen in Optimization #2.

**Status**: ❌ **REVERTED** - This change will be rolled back as it decreased performance.

---

### Optimization #4: Optimized Neighbor Counting Algorithm

**Date**: 2025-10-15

**Change**: Replaced loop-based neighbor counting with unrolled manual checks and simplified wrapping logic using conditional expressions instead of `rem_euclid`.

**Implementation Details**:
- Pre-calculate neighbor row/column indices using simple conditionals
- Unrolled all 8 neighbor checks explicitly
- Replaced `rem_euclid` with simple `if-else` for wrapping (e.g., `if row == 0 { height - 1 } else { row - 1 }`)
- Eliminated iterator overhead from `[-1, 0, 1].iter()`
- Removed the conditional check for skipping the center cell

**Results**:
```
Total time: 4.80163834s
Time per iteration: 4.80ms
Iterations per second: 208.26
```

**Improvement**:
- Time per iteration: 13.09ms → 4.80ms (**63.3% faster**)
- Iterations per second: 76.41 → 208.26 (**172.6% increase**)
- Speedup: 2.73x

**Analysis**: This is a massive performance win! The combination of unrolling the neighbor checks and replacing expensive `rem_euclid` operations with simple conditionals provided exceptional gains. The `rem_euclid` function has significant overhead compared to simple branching for our small wrapping cases. Loop unrolling eliminates iterator overhead and allows better compiler optimization.

**Status**: ✅ **KEPT** - Excellent performance improvement.

---

### Optimization #5: Early Change Detection & Copy Trait

**Date**: 2025-10-15

**Change**: Track changes during computation instead of comparing entire grids afterward, and made `CellState` implement `Copy` trait to enable efficient value semantics.

**Implementation Details**:
- Added `Copy` trait to `CellState` enum (alongside existing `Clone`)
- Introduced `changed` boolean flag to track if any cell changed state
- Compare `next_state != current_state` during iteration instead of full grid comparison at the end
- Avoided expensive `Vec<Vec<CellState>>` equality check
- Store current_state as value instead of reference for cleaner comparison

**Results**:
```
Total time: 5.003322532s
Time per iteration: 5.00ms
Iterations per second: 199.87
```

**Improvement**:
- Time per iteration: 4.80ms → 5.00ms (**4.2% slower**)
- Iterations per second: 208.26 → 199.87 (**4.0% decrease**)
- Speedup: 0.96x

**Analysis**: This optimization resulted in a slight performance regression. While avoiding the full grid comparison seems beneficial, the per-cell comparison check `next_state != current_state` executed 1 million times per iteration adds overhead that outweighs the benefit. The original full grid comparison happens only once and is likely highly optimized by the compiler. Additionally, the boolean flag check and update on every cell may interfere with CPU pipelining.

**Status**: ❌ **REVERTED** - This change will be rolled back as it decreased performance. However, the `Copy` trait addition will be kept as it's a sensible optimization that enables better codegen.

---
