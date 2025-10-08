/**
* A live cell dies if it has fewer than two live neighbors.
* A live cell with two or three live neighbors lives on to the next generation.
* A live cell with more than three live neighbors dies.
* A dead cell will be brought back to live if it has exactly three live neighbors.
*/
pub mod grid {
    use crate::grid::CellState::{Alive, Dead};
    use rand::Rng;

    #[derive(Debug, PartialEq, Clone)]
    pub enum CellState {
        Dead,
        Alive,
    }
    #[derive(Default)]
    pub struct Grid {
        pub cells: Vec<Vec<CellState>>,
    }

    impl Grid {
        pub fn new(width: usize, height: usize) -> Self {
            Grid {
                cells: vec![vec![Dead; width]; height],
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
            let mut next_grid = vec![vec![Dead; self.cells.first().unwrap().len()]; self.cells.len()];

            for (row_index, next_row) in next_grid.iter_mut().enumerate() {
                for (col_index, next_cell) in next_row.iter_mut().enumerate() {
                    let alive_neighbors = self.alive_neighbors(row_index, col_index);
                    let is_alive = &self.cells[row_index][col_index];

                    // Apply Game of Life rules
                    *next_cell = match (is_alive, alive_neighbors) {
                        (Alive, 2..=3) => Alive, // Survives
                        (Dead, 3) => Alive,      // Becomes alive
                        _ => Dead,               // Dies or remains dead
                    };
                }
            }

            if self.cells == next_grid {
                return false;
            }
            self.cells = next_grid;
            true
        }

        /// Count the number of alive neighbors for a cell
        fn alive_neighbors(&self, row: usize, col: usize) -> usize {
            let mut count = 0;

            for dr in [-1, 0, 1].iter() {
                for dc in [-1, 0, 1].iter() {
                    if *dr == 0 && *dc == 0 {
                        // Skip the current cell
                        continue;
                    }

                    let neighbor_row = (row as isize + dr).rem_euclid(self.cells.len() as isize) as usize;
                    let neighbor_col = (col as isize + dc).rem_euclid(self.cells[row].len() as isize) as usize;

                    if self.cells[neighbor_row][neighbor_col] == Alive {
                        count += 1;
                    }
                }
            }

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
    }
}
