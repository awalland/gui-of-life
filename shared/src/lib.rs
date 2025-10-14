/**
* A live cell dies if it has fewer than two live neighbors.
* A live cell with two or three live neighbors lives on to the next generation.
* A live cell with more than three live neighbors dies.
* A dead cell will be brought back to live if it has exactly three live neighbors.
*/
pub mod grid {
    use crate::grid::CellState::{Alive, Dead};
    use rand::Rng;

    #[derive(Debug, PartialEq, Clone, Copy)]
    pub enum CellState {
        Dead,
        Alive,
    }
    #[derive(Default)]
    pub struct Grid {
        pub cells: Vec<Vec<CellState>>,
        next_cells: Vec<Vec<CellState>>,
    }

    impl Grid {
        pub fn new(width: usize, height: usize) -> Self {
            Grid {
                cells: vec![vec![Dead; width]; height],
                next_cells: vec![vec![Dead; width]; height],
            }
        }

        pub fn randomize(&mut self) {
            let mut rng = rand::rng();
            self.randomize_with_rng(&mut rng);
        }

        fn randomize_with_rng<R: Rng + ?Sized>(&mut self, rng: &mut R) {
            for row in self.cells.iter_mut() {
                for cell in row.iter_mut() {
                    *cell = if rng.random_bool(0.5) { Alive } else { Dead };
                }
            }
        }

        /// Advance the grid by one step (Game of Life logic)
        pub fn advance(&mut self) -> bool {
            let height = self.cells.len();
            let width = self.cells[0].len();

            for row_index in 0..height {
                for col_index in 0..width {
                    let alive_neighbors = self.alive_neighbors(row_index, col_index);
                    let is_alive = self.cells[row_index][col_index];

                    // Apply Game of Life rules
                    self.next_cells[row_index][col_index] = match (is_alive, alive_neighbors) {
                        (Alive, 2..=3) => Alive, // Survives
                        (Dead, 3) => Alive,      // Becomes alive
                        _ => Dead,               // Dies or remains dead
                    };
                }
            }

            if self.cells == self.next_cells {
                return false;
            }
            std::mem::swap(&mut self.cells, &mut self.next_cells);
            true
        }

        /// Count the number of alive neighbors for a cell
        fn alive_neighbors(&self, row: usize, col: usize) -> usize {
            let height = self.cells.len();
            let width = self.cells[0].len();
            let mut count = 0;

            // Unrolled neighbor checks for better performance
            // Top row
            let top = if row == 0 { height - 1 } else { row - 1 };
            let bottom = if row == height - 1 { 0 } else { row + 1 };
            let left = if col == 0 { width - 1 } else { col - 1 };
            let right = if col == width - 1 { 0 } else { col + 1 };

            if self.cells[top][left] == Alive { count += 1; }
            if self.cells[top][col] == Alive { count += 1; }
            if self.cells[top][right] == Alive { count += 1; }

            if self.cells[row][left] == Alive { count += 1; }
            if self.cells[row][right] == Alive { count += 1; }

            if self.cells[bottom][left] == Alive { count += 1; }
            if self.cells[bottom][col] == Alive { count += 1; }
            if self.cells[bottom][right] == Alive { count += 1; }

            count
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use rand::{rngs::StdRng, Rng, SeedableRng};

        fn grid_with_alive_cells(width: usize, height: usize, alive_positions: &[(usize, usize)]) -> Grid {
            let mut grid = Grid::new(width, height);
            for &(row, col) in alive_positions {
                grid.cells[row][col] = Alive;
            }
            grid
        }

        #[test]
        fn new_initializes_dead_cells() {
            let grid = Grid::new(3, 2);
            assert_eq!(grid.cells.len(), 2);
            assert!(grid.cells.iter().all(|row| row.len() == 3));
            assert!(grid.cells.iter().all(|row| row.iter().all(|cell| *cell == Dead)));
        }

        #[test]
        fn randomize_with_seed_is_deterministic() {
            let mut grid = Grid::new(2, 3);
            let mut rng = StdRng::seed_from_u64(42);
            grid.randomize_with_rng(&mut rng);

            let mut rng = StdRng::seed_from_u64(42);
            let mut expected = vec![vec![Dead; 2]; 3];
            for row in expected.iter_mut() {
                for cell in row.iter_mut() {
                    *cell = if rng.random_bool(0.5) { Alive } else { Dead };
                }
            }

            assert_eq!(grid.cells, expected);
            let alive_count = grid.cells.iter().flatten().filter(|cell| **cell == Alive).count();
            assert!(alive_count > 0);
            assert!(alive_count < grid.cells.len() * grid.cells[0].len());
        }

        #[test]
        fn alive_neighbors_wraps_around_edges() {
            let grid = grid_with_alive_cells(3, 3, &[(0, 2), (2, 0), (2, 2)]);
            assert_eq!(grid.alive_neighbors(0, 0), 3);
        }

        #[test]
        fn alive_neighbors_counts_zero_for_isolated_cell() {
            let grid = Grid::new(3, 3);
            assert_eq!(grid.alive_neighbors(1, 1), 0);
        }

        #[test]
        fn advance_returns_false_for_static_pattern() {
            let mut grid = grid_with_alive_cells(4, 4, &[(1, 1), (1, 2), (2, 1), (2, 2)]);
            assert!(!grid.advance());
        }

        #[test]
        fn lonely_alive_cell_dies() {
            let mut grid = grid_with_alive_cells(3, 3, &[(1, 1)]);
            assert!(grid.advance());
            assert_eq!(grid.cells[1][1], Dead);
        }

        #[test]
        fn alive_cell_with_two_neighbors_survives() {
            let mut grid = grid_with_alive_cells(3, 3, &[(1, 0), (1, 1), (1, 2)]);
            assert!(grid.advance());
            assert_eq!(grid.cells[1][1], Alive);
        }

        #[test]
        fn overcrowded_cell_dies() {
            let mut grid = grid_with_alive_cells(3, 3, &[(1, 1), (0, 1), (1, 0), (1, 2), (2, 1)]);
            assert!(grid.advance());
            assert_eq!(grid.cells[1][1], Dead);
        }

        #[test]
        fn dead_cell_with_three_neighbors_revives() {
            let mut grid = grid_with_alive_cells(3, 3, &[(0, 1), (1, 0), (1, 2)]);
            assert!(grid.advance());
            assert_eq!(grid.cells[1][1], Alive);
        }

        #[test]
        #[ignore] // Run with: cargo test --release -- --ignored --nocapture
        fn benchmark_advance_performance() {
            use std::time::Instant;

            const GRID_WIDTH: usize = 1000;
            const GRID_HEIGHT: usize = 1000;
            const ITERATIONS: usize = 1000;

            // Create a grid with reproducible random state
            let mut grid = Grid::new(GRID_WIDTH, GRID_HEIGHT);
            let mut rng = StdRng::seed_from_u64(12345);
            grid.randomize_with_rng(&mut rng);

            // Warm up
            for _ in 0..10 {
                grid.advance();
            }

            // Reset to initial state for actual benchmark
            grid = Grid::new(GRID_WIDTH, GRID_HEIGHT);
            let mut rng = StdRng::seed_from_u64(12345);
            grid.randomize_with_rng(&mut rng);

            // Benchmark
            let start = Instant::now();
            let mut total_changes = 0;
            for i in 0..ITERATIONS {
                if grid.advance() {
                    total_changes += 1;
                }
                if i % 100 == 0 {
                    println!("Iteration {}/{}", i, ITERATIONS);
                }
            }
            let duration = start.elapsed();

            println!("\n=== Performance Benchmark Results ===");
            println!("Grid size: {}x{} ({} cells)", GRID_WIDTH, GRID_HEIGHT, GRID_WIDTH * GRID_HEIGHT);
            println!("Iterations: {}", ITERATIONS);
            println!("Total time: {:?}", duration);
            println!("Time per iteration: {:?}", duration / ITERATIONS as u32);
            println!("Iterations per second: {:.2}", ITERATIONS as f64 / duration.as_secs_f64());
            println!("Iterations with changes: {}", total_changes);
            println!("=====================================\n");

            // Ensure the benchmark actually ran
            assert!(duration.as_millis() > 0);
        }
    }
}
