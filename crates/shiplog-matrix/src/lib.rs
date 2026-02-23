//! Matrix/2D array utilities for shiplog.
//!
//! Provides basic matrix data structures and operations for 2D arrays.

use serde::{Deserialize, Serialize};
use std::fmt;

/// A 2D matrix of type T.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Matrix<T> {
    /// Number of rows
    rows: usize,
    /// Number of columns
    cols: usize,
    /// Flattened data in row-major order
    data: Vec<T>,
}

impl<T: Clone> Matrix<T> {
    /// Create a new matrix with the given dimensions.
    pub fn new(rows: usize, cols: usize) -> Self
    where
        T: Default,
    {
        let data = vec![T::default(); rows * cols];
        Self { rows, cols, data }
    }

    /// Create a matrix filled with a specific value.
    pub fn filled(rows: usize, cols: usize, value: T) -> Self {
        let data = vec![value; rows * cols];
        Self { rows, cols, data }
    }

    /// Get the number of rows.
    pub fn rows(&self) -> usize {
        self.rows
    }

    /// Get the number of columns.
    pub fn cols(&self) -> usize {
        self.cols
    }

    /// Get a reference to an element at (row, col).
    pub fn get(&self, row: usize, col: usize) -> Option<&T> {
        if row < self.rows && col < self.cols {
            Some(&self.data[row * self.cols + col])
        } else {
            None
        }
    }

    /// Get a mutable reference to an element at (row, col).
    pub fn get_mut(&mut self, row: usize, col: usize) -> Option<&mut T> {
        if row < self.rows && col < self.cols {
            Some(&mut self.data[row * self.cols + col])
        } else {
            None
        }
    }

    /// Set an element at (row, col).
    pub fn set(&mut self, row: usize, col: usize, value: T) -> bool {
        if let Some(elem) = self.get_mut(row, col) {
            *elem = value;
            true
        } else {
            false
        }
    }

    /// Iterate over all elements.
    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.data.iter()
    }

    /// Iterate over rows.
    pub fn rows_iter(&self) -> impl Iterator<Item = Vec<T>> {
        (0..self.rows).map(|r| {
            (0..self.cols)
                .map(|c| self.data[r * self.cols + c].clone())
                .collect()
        })
    }

    /// Transpose the matrix.
    pub fn transpose(&self) -> Matrix<T>
    where
        T: Clone,
    {
        let mut result = Matrix::filled(self.cols, self.rows, self.data[0].clone());
        for r in 0..self.rows {
            for c in 0..self.cols {
                result.data[c * self.rows + r] = self.data[r * self.cols + c].clone();
            }
        }
        result
    }
}

/// A position in a 2D grid.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Position {
    /// Row index
    pub row: usize,
    /// Column index
    pub col: usize,
}

impl Position {
    /// Create a new position.
    pub fn new(row: usize, col: usize) -> Self {
        Self { row, col }
    }

    /// Create a position from (row, col) tuple.
    pub fn from_tuple((row, col): (usize, usize)) -> Self {
        Self { row, col }
    }

    /// Convert to (row, col) tuple.
    pub fn to_tuple(&self) -> (usize, usize) {
        (self.row, self.col)
    }
}

impl fmt::Display for Position {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({}, {})", self.row, self.col)
    }
}

/// A 2D grid with optional values.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Grid<T> {
    rows: usize,
    cols: usize,
    cells: Vec<Option<T>>,
}

impl<T: Clone> Grid<T> {
    /// Create a new grid with the given dimensions.
    pub fn new(rows: usize, cols: usize) -> Self {
        let cells = vec![None; rows * cols];
        Self { rows, cols, cells }
    }

    /// Get the number of rows.
    pub fn rows(&self) -> usize {
        self.rows
    }

    /// Get the number of columns.
    pub fn cols(&self) -> usize {
        self.cols
    }

    /// Get an element at position.
    pub fn get(&self, pos: &Position) -> Option<&T> {
        self.get_at(pos.row, pos.col)
    }

    /// Get an element at (row, col).
    pub fn get_at(&self, row: usize, col: usize) -> Option<&T> {
        if row < self.rows && col < self.cols {
            self.cells[row * self.cols + col].as_ref()
        } else {
            None
        }
    }

    /// Set an element at position.
    pub fn set(&mut self, pos: &Position, value: T) -> bool {
        self.set_at(pos.row, pos.col, value)
    }

    /// Set an element at (row, col).
    pub fn set_at(&mut self, row: usize, col: usize, value: T) -> bool {
        if row < self.rows && col < self.cols {
            self.cells[row * self.cols + col] = Some(value);
            true
        } else {
            false
        }
    }

    /// Clear an element at position.
    pub fn clear(&mut self, pos: &Position) -> bool {
        self.clear_at(pos.row, pos.col)
    }

    /// Clear an element at (row, col).
    pub fn clear_at(&mut self, row: usize, col: usize) -> bool {
        if row < self.rows && col < self.cols {
            self.cells[row * self.cols + col] = None;
            true
        } else {
            false
        }
    }

    /// Check if a position is within bounds.
    pub fn in_bounds(&self, row: usize, col: usize) -> bool {
        row < self.rows && col < self.cols
    }

    /// Iterate over all occupied positions.
    pub fn occupied(&self) -> impl Iterator<Item = (Position, &T)> {
        self.cells.iter().enumerate().filter_map(|(i, cell)| {
            cell.as_ref().map(|v| {
                let row = i / self.cols;
                let col = i % self.cols;
                (Position { row, col }, v)
            })
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matrix_creation() {
        let matrix: Matrix<i32> = Matrix::new(3, 4);
        assert_eq!(matrix.rows(), 3);
        assert_eq!(matrix.cols(), 4);
    }

    #[test]
    fn matrix_filled() {
        let matrix = Matrix::filled(2, 3, 5);
        assert_eq!(matrix.rows(), 2);
        assert_eq!(matrix.cols(), 3);
        assert_eq!(matrix.get(0, 0), Some(&5));
        assert_eq!(matrix.get(1, 2), Some(&5));
    }

    #[test]
    fn matrix_get_set() {
        let mut matrix = Matrix::filled(2, 2, 0);
        matrix.set(0, 0, 1);
        matrix.set(1, 1, 2);

        assert_eq!(matrix.get(0, 0), Some(&1));
        assert_eq!(matrix.get(1, 1), Some(&2));
    }

    #[test]
    fn matrix_out_of_bounds() {
        let mut matrix: Matrix<i32> = Matrix::new(2, 2);
        assert_eq!(matrix.get(5, 5), None);
        assert!(!matrix.set(5, 5, 1));
    }

    #[test]
    fn matrix_iter() {
        let matrix = Matrix::filled(2, 2, 1);
        let sum: i32 = matrix.iter().sum();
        assert_eq!(sum, 4);
    }

    #[test]
    fn matrix_rows_iter() {
        let matrix = Matrix::filled(2, 3, 1);
        let rows: Vec<_> = matrix.rows_iter().collect();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].len(), 3);
    }

    #[test]
    fn matrix_transpose() {
        let matrix = Matrix::filled(2, 3, 1);
        let transposed = matrix.transpose();
        assert_eq!(transposed.rows(), 3);
        assert_eq!(transposed.cols(), 2);
    }

    #[test]
    fn position_creation() {
        let pos = Position::new(2, 3);
        assert_eq!(pos.row, 2);
        assert_eq!(pos.col, 3);
    }

    #[test]
    fn position_tuple() {
        let pos = Position::from_tuple((2, 3));
        assert_eq!(pos.to_tuple(), (2, 3));
    }

    #[test]
    fn position_display() {
        let pos = Position::new(2, 3);
        assert_eq!(format!("{}", pos), "(2, 3)");
    }

    #[test]
    fn grid_creation() {
        let grid: Grid<i32> = Grid::new(3, 4);
        assert_eq!(grid.rows(), 3);
        assert_eq!(grid.cols(), 4);
    }

    #[test]
    fn grid_get_set() {
        let mut grid = Grid::new(2, 2);
        grid.set(&Position::new(0, 0), 1);
        grid.set_at(1, 1, 2);

        assert_eq!(grid.get(&Position::new(0, 0)), Some(&1));
        assert_eq!(grid.get_at(1, 1), Some(&2));
    }

    #[test]
    fn grid_clear() {
        let mut grid = Grid::new(2, 2);
        grid.set(&Position::new(0, 0), 1);
        grid.clear(&Position::new(0, 0));

        assert_eq!(grid.get(&Position::new(0, 0)), None);
    }

    #[test]
    fn grid_in_bounds() {
        let grid: Grid<i32> = Grid::new(2, 2);
        assert!(grid.in_bounds(0, 0));
        assert!(grid.in_bounds(1, 1));
        assert!(!grid.in_bounds(2, 0));
        assert!(!grid.in_bounds(0, 2));
    }

    #[test]
    fn grid_occupied() {
        let mut grid = Grid::new(2, 2);
        grid.set(&Position::new(0, 0), 1);
        grid.set(&Position::new(1, 1), 2);

        let occupied: Vec<_> = grid.occupied().collect();
        assert_eq!(occupied.len(), 2);
    }

    #[test]
    fn matrix_clone() {
        let matrix = Matrix::filled(2, 2, 1);
        let cloned = matrix.clone();
        assert_eq!(matrix, cloned);
    }

    #[test]
    fn grid_clone() {
        let mut grid = Grid::new(2, 2);
        grid.set(&Position::new(0, 0), 1);
        let cloned = grid.clone();
        assert_eq!(grid, cloned);
    }
}
