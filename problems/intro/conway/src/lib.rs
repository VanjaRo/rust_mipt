#![forbid(unsafe_code)]

////////////////////////////////////////////////////////////////////////////////

#[derive(Clone, PartialEq, Eq)]
pub struct Grid<T> {
    rows: usize,
    cols: usize,
    grid: Vec<T>,
}

impl<T: Clone + Default> Grid<T> {
    pub fn new(rows: usize, cols: usize) -> Self {
        Self {
            rows,
            cols,
            grid: vec![T::default(); rows * cols],
        }
    }

    pub fn from_slice(grid: &[T], rows: usize, cols: usize) -> Self {
        Self {
            rows,
            cols,
            grid: grid.to_vec(),
        }
    }

    pub fn size(&self) -> (usize, usize) {
        (self.rows, self.cols)
    }

    pub fn get(&self, row: usize, col: usize) -> &T {
        &self.grid[col * self.rows + row]
    }

    pub fn set(&mut self, value: T, row: usize, col: usize) {
        self.grid[col * self.rows + row] = value;
    }

    pub fn neighbours(&self, row: usize, col: usize) -> Vec<(usize, usize)> {
        let (irow, icol) = (row as isize, col as isize);
        let directions = vec![
            // row above
            (-1, -1),
            (-1, 0),
            (-1, 1),
            // row same
            (0, -1),
            (0, 1),
            // row below
            (1, -1),
            (1, 0),
            (1, 1),
        ];
        directions
            .into_iter()
            .map(|pr| (pr.0 + irow, pr.1 + icol))
            .filter(|(p_row, p_col)| {
                *p_row >= 0
                    && *p_row < self.rows as isize
                    && *p_col >= 0
                    && *p_col < self.cols as isize
            })
            .map(|pr| (pr.0 as usize, pr.1 as usize))
            .collect()
    }
}

////////////////////////////////////////////////////////////////////////////////

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Cell {
    Dead,
    Alive,
}

impl Default for Cell {
    fn default() -> Self {
        Self::Dead
    }
}

////////////////////////////////////////////////////////////////////////////////

#[derive(PartialEq, Eq)]
pub struct GameOfLife {
    grid: Grid<Cell>,
}

impl GameOfLife {
    pub fn from_grid(grid: Grid<Cell>) -> Self {
        Self { grid }
    }

    pub fn get_grid(&self) -> &Grid<Cell> {
        &self.grid
    }

    pub fn step(&mut self) {
        let mut new_grid = Grid::<Cell>::new(self.grid.rows, self.grid.cols);
        for r in 0..self.get_grid().rows {
            for c in 0..self.get_grid().cols {
                let alive_count = self
                    .get_grid()
                    .neighbours(r, c)
                    .into_iter()
                    .map(|(c_row, c_col)| *self.get_grid().get(c_row, c_col))
                    .filter(|cell| *cell == Cell::Alive)
                    .count();
                let cell = self.get_grid().get(r, c);
                new_grid.set(
                    match (cell, alive_count) {
                        (c, 2) => *c,
                        (_, 3) => Cell::Alive,
                        _ => Cell::Dead,
                    },
                    r,
                    c,
                );
            }
        }
        self.grid = new_grid;
    }
}
