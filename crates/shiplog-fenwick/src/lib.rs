//! Fenwick tree (Binary Indexed Tree) implementation for shiplog.
//!
//! This crate provides a Fenwick tree implementation for efficient
//! prefix sum queries and point updates.

/// A Fenwick Tree (Binary Indexed Tree) for prefix sum queries.
///
/// Supports:
/// - Point updates: O(log n)
/// - Prefix sum queries: O(log n)
/// - Range sum queries: O(log n)
#[derive(Debug, Clone)]
pub struct FenwickTree<T> {
    data: Vec<T>,
    size: usize,
}

impl<T: Copy + Default + std::ops::Add<Output = T>> FenwickTree<T> {
    /// Creates a new FenwickTree with the given size.
    pub fn new(size: usize) -> Self {
        Self {
            data: vec![T::default(); size + 1],
            size,
        }
    }

    /// Creates a FenwickTree from a slice of values.
    pub fn from_slice(slice: &[T]) -> Self {
        let size = slice.len();
        let mut tree = Self::new(size);

        for (i, &val) in slice.iter().enumerate() {
            tree.data[i + 1] = val;
        }

        // Build the tree
        for i in 1..=size {
            let j = i + (i & i.wrapping_neg());
            if j <= size {
                tree.data[j] = tree.data[j] + tree.data[i];
            }
        }

        tree
    }

    /// Updates the value at index `i` by adding `delta`.
    pub fn add(&mut self, mut i: usize, delta: T) {
        i += 1; // Convert to 1-based indexing
        while i <= self.size {
            self.data[i] = self.data[i] + delta;
            i += i & i.wrapping_neg();
        }
    }

    /// Sets the value at index `i` to `value`.
    pub fn set(&mut self, i: usize, value: T)
    where
        T: std::ops::Sub<Output = T>,
    {
        let current = self.sum(i);
        let delta = value - current;
        self.add(i, delta);
    }

    /// Returns the prefix sum of elements [0, i] (inclusive).
    pub fn sum(&self, mut i: usize) -> T {
        if i >= self.size {
            i = self.size - 1;
        }

        i += 1; // Convert to 1-based indexing
        let mut result = T::default();

        while i > 0 {
            result = result + self.data[i];
            i -= i & i.wrapping_neg();
        }

        result
    }

    /// Returns the sum of elements in range [l, r] (inclusive).
    pub fn range_sum(&self, l: usize, r: usize) -> T
    where
        T: std::ops::Sub<Output = T>,
    {
        if l > r {
            return T::default();
        }
        if r >= self.size {
            return T::default();
        }

        self.sum(r) - (if l > 0 { self.sum(l - 1) } else { T::default() })
    }

    /// Returns the number of elements in the tree.
    pub fn len(&self) -> usize {
        self.size
    }

    /// Returns true if the tree is empty.
    pub fn is_empty(&self) -> bool {
        self.size == 0
    }

    /// Returns the value at index `i`.
    pub fn get(&self, i: usize) -> T
    where
        T: std::ops::Sub<Output = T>,
    {
        if i >= self.size {
            return T::default();
        }
        self.range_sum(i, i)
    }
}

impl<T: Copy + Default + std::ops::Add<Output = T> + std::ops::Sub<Output = T> + PartialOrd>
    FenwickTree<T>
{
    /// Finds the smallest index `i` such that prefix sum >= target.
    /// Returns None if target is greater than the total sum.
    pub fn lower_bound(&self, mut target: T) -> Option<usize> {
        if target <= T::default() {
            return Some(0);
        }

        let total = self.sum(self.size - 1);
        if target > total {
            return None;
        }

        let mut idx = 0;
        let mut bit_mask = self.size.next_power_of_two();

        while bit_mask > 0 {
            let next = idx + bit_mask;
            if next <= self.size && self.data[next] < target {
                idx = next;
                target = target - self.data[next];
            }
            bit_mask >>= 1;
        }

        Some(idx)
    }
}

/// A 2D Fenwick Tree for range sum queries on a 2D matrix.
#[derive(Debug, Clone)]
pub struct FenwickTree2D<T> {
    data: Vec<Vec<T>>,
    rows: usize,
    cols: usize,
}

impl<T: Copy + Default + std::ops::Add<Output = T>> FenwickTree2D<T> {
    /// Creates a new 2D FenwickTree with the given dimensions.
    pub fn new(rows: usize, cols: usize) -> Self {
        Self {
            data: vec![vec![T::default(); cols + 1]; rows + 1],
            rows,
            cols,
        }
    }

    /// Updates the value at (row, col) by adding delta.
    pub fn add(&mut self, mut row: usize, mut col: usize, delta: T) {
        row += 1;
        col += 1;

        let mut i = row;
        while i <= self.rows {
            let mut j = col;
            while j <= self.cols {
                self.data[i][j] = self.data[i][j] + delta;
                j += j & j.wrapping_neg();
            }
            i += i & i.wrapping_neg();
        }
    }

    /// Returns the prefix sum of submatrix [(0,0), (row, col)].
    pub fn sum(&self, mut row: usize, mut col: usize) -> T {
        if row >= self.rows {
            row = self.rows - 1;
        }
        if col >= self.cols {
            col = self.cols - 1;
        }

        row += 1;
        col += 1;

        let mut result = T::default();
        let mut i = row;

        while i > 0 {
            let mut j = col;
            while j > 0 {
                result = result + self.data[i][j];
                j -= j & j.wrapping_neg();
            }
            i -= i & i.wrapping_neg();
        }

        result
    }

    /// Returns the sum of submatrix [(r1, c1), (r2, c2)] (inclusive).
    pub fn range_sum(&self, r1: usize, c1: usize, r2: usize, c2: usize) -> T
    where
        T: std::ops::Sub<Output = T>,
    {
        if r1 > r2 || c1 > c2 {
            return T::default();
        }

        self.sum(r2, c2)
            - (if r1 > 0 {
                self.sum(r1 - 1, c2)
            } else {
                T::default()
            })
            - (if c1 > 0 {
                self.sum(r2, c1 - 1)
            } else {
                T::default()
            })
            + (if r1 > 0 && c1 > 0 {
                self.sum(r1 - 1, c1 - 1)
            } else {
                T::default()
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Basic FenwickTree tests
    #[test]
    fn test_fenwick_new() {
        let tree: FenwickTree<i32> = FenwickTree::new(5);
        assert_eq!(tree.len(), 5);
        assert!(!tree.is_empty());
    }

    #[test]
    fn test_fenwick_from_slice() {
        let tree = FenwickTree::from_slice(&[1, 2, 3, 4, 5]);
        assert_eq!(tree.len(), 5);
        assert_eq!(tree.sum(0), 1);
        assert_eq!(tree.sum(4), 15);
    }

    #[test]
    fn test_fenwick_add() {
        let mut tree = FenwickTree::new(5);
        tree.add(0, 1);
        tree.add(1, 2);
        tree.add(2, 3);

        assert_eq!(tree.sum(0), 1);
        assert_eq!(tree.sum(1), 3);
        assert_eq!(tree.sum(2), 6);
    }

    #[test]
    fn test_fenwick_set() {
        let mut tree = FenwickTree::new(5);
        tree.add(0, 1);
        tree.add(1, 2);
        // set replaces the value at index 1 from 2 to 10
        // current value at index 1 is 2, we want to set it to 10
        // we compute delta = 10 - current_sum(1) = 10 - 3 = 7
        // then add 7 to index 1, so the prefix sum becomes 1 + 9 = 10
        tree.set(1, 10);

        assert_eq!(tree.sum(0), 1);
        assert_eq!(tree.sum(1), 10); // 1 + 9 after set
    }

    #[test]
    fn test_fenwick_range_sum() {
        let tree = FenwickTree::from_slice(&[1, 2, 3, 4, 5]);

        assert_eq!(tree.range_sum(0, 2), 6); // 1 + 2 + 3
        assert_eq!(tree.range_sum(1, 3), 9); // 2 + 3 + 4
        assert_eq!(tree.range_sum(2, 4), 12); // 3 + 4 + 5
        assert_eq!(tree.range_sum(0, 4), 15); // 1 + 2 + 3 + 4 + 5
    }

    #[test]
    fn test_fenwick_range_sum_empty() {
        let tree = FenwickTree::from_slice(&[1, 2, 3]);

        assert_eq!(tree.range_sum(3, 5), 0); // Out of bounds
        assert_eq!(tree.range_sum(2, 1), 0); // Invalid range
    }

    #[test]
    fn test_fenwick_get() {
        let tree = FenwickTree::from_slice(&[1, 2, 3, 4, 5]);

        assert_eq!(tree.get(0), 1);
        assert_eq!(tree.get(2), 3);
        assert_eq!(tree.get(4), 5);
        assert_eq!(tree.get(5), 0); // Out of bounds
    }

    #[test]
    fn test_fenwick_lower_bound() {
        let tree = FenwickTree::from_slice(&[1, 2, 3, 4, 5]);
        // Prefix sums: [1, 3, 6, 10, 15]

        assert_eq!(tree.lower_bound(1), Some(0)); // First prefix >= 1 is at index 0
        assert_eq!(tree.lower_bound(3), Some(1)); // First prefix >= 3 is at index 1 (sum=3)
        assert_eq!(tree.lower_bound(6), Some(2)); // First prefix >= 6 is at index 2 (sum=6)
        assert_eq!(tree.lower_bound(16), None); // Greater than total sum
    }

    #[test]
    fn test_fenwick_empty() {
        let tree: FenwickTree<i32> = FenwickTree::new(0);
        assert!(tree.is_empty());
    }

    // 2D FenwickTree tests
    #[test]
    fn test_fenwick2d_new() {
        let tree: FenwickTree2D<i32> = FenwickTree2D::new(3, 3);
        assert_eq!(tree.range_sum(0, 0, 2, 2), 0);
    }

    #[test]
    fn test_fenwick2d_add() {
        let mut tree = FenwickTree2D::new(3, 3);
        tree.add(0, 0, 1);
        tree.add(1, 1, 2);
        tree.add(2, 2, 3);

        assert_eq!(tree.sum(0, 0), 1);
        assert_eq!(tree.sum(1, 1), 3); // 1 + 2
        assert_eq!(tree.sum(2, 2), 6); // 1 + 2 + 3
    }

    #[test]
    fn test_fenwick2d_range_sum() {
        let mut tree = FenwickTree2D::new(3, 3);

        // Matrix:
        // 1 2 3
        // 4 5 6
        // 7 8 9
        for i in 0..3 {
            for j in 0..3 {
                tree.add(i, j, (i * 3 + j + 1) as i32);
            }
        }

        assert_eq!(tree.range_sum(0, 0, 0, 0), 1);
        assert_eq!(tree.range_sum(0, 0, 1, 1), 12); // 1+2+4+5
    }
}
