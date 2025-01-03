/**
* A live cell dies if it has fewer than two live neighbors.
* A live cell with two or three live neighbors lives on to the next generation.
* A live cell with more than three live neighbors dies.
* A dead cell will be brought back to live if it has exactly three live neighbors.
*/

pub mod grid {
    use crate::grid::CellState::{Alive, Dead};
    use rand::Rng;

    #[derive(PartialEq, Clone)]
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
            let mut new_grid = Grid::new(self.cells.first().unwrap().len(), self.cells.len());

            for cell_row in 0..self.cells.len() {
                let row = &self.cells[cell_row];
                for cell_column in 0..row.len() {
                    new_grid.cells[cell_row][cell_column] =
                        if rng.random_bool(0.5) { Alive } else { Dead };
                }
            }
            self.cells = new_grid.cells;
        }

        /// Advance the grid by one step (Game of Life logic)
        pub fn advance(&mut self) -> bool {
            let mut next_grid =
                vec![vec![Dead; self.cells.first().unwrap().len()]; self.cells.len()];

            for row in 0..self.cells.len() {
                for col in 0..self.cells[row].len() {
                    let alive_neighbors = self.alive_neighbors(row, col);
                    let is_alive = &self.cells[row][col];

                    // Apply Game of Life rules
                    next_grid[row][col] = match (is_alive, alive_neighbors) {
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

                    let neighbor_row =
                        (row as isize + dr).rem_euclid(self.cells.len() as isize) as usize;
                    let neighbor_col =
                        (col as isize + dc).rem_euclid(self.cells[row].len() as isize) as usize;

                    if self.cells[neighbor_row][neighbor_col] == Alive {
                        count += 1;
                    }
                }
            }

            count
        }
    }
}
