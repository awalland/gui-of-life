use eframe::egui;
use eframe::egui::{ScrollArea, Ui};
use eframe::run_native;
use shared::grid::CellState::Alive;
use shared::grid::Grid;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

const GRID_WIDTH: usize = 200;
const GRID_HEIGHT: usize = GRID_WIDTH * 9 / 16;
const CELL_SIZE: f32 = 8.0;
const SLEEP_DURATION: Duration = Duration::from_millis(50);

fn main() {
    // Shared grid state wrapped in Arc<Mutex<T>> for synchronization between threads
    let shared_grid = Arc::new(Mutex::new((Grid::new(GRID_WIDTH, GRID_HEIGHT), false)));

    run_native(
        "Game of Life GUI",
        eframe::NativeOptions::default(),
        Box::new(|cc| {
            // Pass the creation context and shared grid to initialize the app
            let ctx = cc.egui_ctx.clone();
            let grid_clone = Arc::clone(&shared_grid);

            // Spawn a background thread to update the grid
            thread::spawn(move || loop {
                thread::sleep(SLEEP_DURATION);
                let mut grid_and_state = grid_clone.lock().unwrap();
                let changed = grid_and_state.0.advance();
                grid_and_state.1 = changed; // Mark the grid as dirty
                if changed {
                    ctx.request_repaint();
                }
            });

            Ok(Box::new(GuiOfLife::new(cc, shared_grid)))
        }),
    )
    .unwrap();
}

#[derive(Default)]
struct GuiOfLife {
    grid_and_state: Arc<Mutex<(Grid, bool)>>, // Shared grid state
}

impl GuiOfLife {
    fn new(_cc: &eframe::CreationContext<'_>, shared_grid: Arc<Mutex<(Grid, bool)>>) -> Self {
        Self { grid_and_state: shared_grid }
    }

    fn randomize(&mut self) {
        let mut grid = self.grid_and_state.lock().unwrap();
        grid.0.randomize();
        grid.1 = true;
    }

    fn create_grid(&mut self, ui: &mut Ui) {
        let grid_and_state = self.grid_and_state.lock().unwrap();

        // Calculate the grid starting point
        let (rect_min, _) = ui.allocate_exact_size(
            egui::vec2(
                CELL_SIZE * grid_and_state.0.cells[0].len() as f32,
                CELL_SIZE * grid_and_state.0.cells.len() as f32,
            ),
            egui::Sense::hover(),
        );

        // Draw each cell at its calculated position
        for (row_index, row) in grid_and_state.0.cells.iter().enumerate() {
            for (col_index, cell) in row.iter().enumerate() {
                // Determine the position of the top-left corner of the cell
                let pos = rect_min.min + egui::vec2(col_index as f32 * CELL_SIZE, row_index as f32 * CELL_SIZE);

                // Determine the color for the cell
                let color = if *cell == Alive {
                    egui::Color32::WHITE
                } else {
                    egui::Color32::DARK_GRAY
                };

                // Draw the cell as a filled rectangle
                let painter = ui.painter(); // Get the painter for the UI
                painter.rect_filled(
                    egui::Rect::from_min_size(pos, egui::vec2(CELL_SIZE, CELL_SIZE)),
                    CELL_SIZE / 4f32 ,
                    color,
                );
            }
        }
    }
}

impl eframe::App for GuiOfLife {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ScrollArea::both().show(ui, |ui| {
                ui.heading("Game of Life");
                ui.horizontal(|ui| {
                    if ui.button("Randomize").clicked() {
                        self.randomize();
                    }
                });

                self.create_grid(ui);
            });
        });
    }
}
