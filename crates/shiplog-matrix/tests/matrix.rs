use proptest::prelude::*;
use shiplog_matrix::{Grid, Matrix, Position};

// ── Known-answer tests ──────────────────────────────────────────────

#[test]
fn matrix_get_set_round_trip() {
    let mut m: Matrix<i32> = Matrix::new(3, 4);
    m.set(1, 2, 42);
    assert_eq!(m.get(1, 2), Some(&42));
}

#[test]
fn matrix_filled_all_same() {
    let m = Matrix::filled(2, 3, 7);
    for r in 0..2 {
        for c in 0..3 {
            assert_eq!(m.get(r, c), Some(&7));
        }
    }
}

#[test]
fn transpose_dimensions_swap() {
    let m = Matrix::filled(2, 5, 0i32);
    let t = m.transpose();
    assert_eq!(t.rows(), 5);
    assert_eq!(t.cols(), 2);
}

#[test]
fn transpose_values_preserved() {
    let mut m: Matrix<i32> = Matrix::new(2, 3);
    m.set(0, 1, 10);
    m.set(1, 2, 20);
    let t = m.transpose();
    assert_eq!(t.get(1, 0), Some(&10));
    assert_eq!(t.get(2, 1), Some(&20));
}

#[test]
fn transpose_involution() {
    let mut m: Matrix<i32> = Matrix::new(3, 4);
    m.set(0, 0, 1);
    m.set(1, 2, 5);
    m.set(2, 3, 9);
    let tt = m.transpose().transpose();
    assert_eq!(m, tt);
}

#[test]
fn rows_iter_returns_correct_count() {
    let m = Matrix::filled(4, 3, 0i32);
    let rows: Vec<_> = m.rows_iter().collect();
    assert_eq!(rows.len(), 4);
    assert!(rows.iter().all(|r| r.len() == 3));
}

#[test]
fn iter_total_count() {
    let m = Matrix::filled(3, 5, 1i32);
    assert_eq!(m.iter().count(), 15);
    assert_eq!(m.iter().sum::<i32>(), 15);
}

// ── Edge cases ──────────────────────────────────────────────────────

#[test]
fn matrix_out_of_bounds_get() {
    let m: Matrix<i32> = Matrix::new(2, 2);
    assert_eq!(m.get(2, 0), None);
    assert_eq!(m.get(0, 2), None);
    assert_eq!(m.get(100, 100), None);
}

#[test]
fn matrix_out_of_bounds_set() {
    let mut m: Matrix<i32> = Matrix::new(2, 2);
    assert!(!m.set(2, 0, 1));
    assert!(!m.set(0, 2, 1));
}

#[test]
fn matrix_1x1() {
    let mut m = Matrix::filled(1, 1, 99);
    assert_eq!(m.rows(), 1);
    assert_eq!(m.cols(), 1);
    assert_eq!(m.get(0, 0), Some(&99));
    m.set(0, 0, 0);
    assert_eq!(m.get(0, 0), Some(&0));
}

// ── Grid tests ──────────────────────────────────────────────────────

#[test]
fn grid_set_get_clear_round_trip() {
    let mut g: Grid<String> = Grid::new(3, 3);
    let pos = Position::new(1, 1);
    g.set(&pos, "hello".into());
    assert_eq!(g.get(&pos), Some(&"hello".to_string()));
    g.clear(&pos);
    assert_eq!(g.get(&pos), None);
}

#[test]
fn grid_out_of_bounds() {
    let mut g: Grid<i32> = Grid::new(2, 2);
    assert!(!g.set_at(5, 5, 1));
    assert_eq!(g.get_at(5, 5), None);
    assert!(!g.clear_at(5, 5));
}

#[test]
fn grid_occupied_only_set_cells() {
    let mut g: Grid<i32> = Grid::new(3, 3);
    g.set_at(0, 0, 1);
    g.set_at(2, 2, 2);
    let occ: Vec<_> = g.occupied().collect();
    assert_eq!(occ.len(), 2);
}

#[test]
fn grid_in_bounds_boundary() {
    let g: Grid<i32> = Grid::new(3, 4);
    assert!(g.in_bounds(2, 3));
    assert!(!g.in_bounds(3, 3));
    assert!(!g.in_bounds(2, 4));
}

// ── Position tests ──────────────────────────────────────────────────

#[test]
fn position_tuple_round_trip() {
    let p = Position::from_tuple((5, 10));
    assert_eq!(p.to_tuple(), (5, 10));
}

#[test]
fn position_display() {
    assert_eq!(format!("{}", Position::new(0, 0)), "(0, 0)");
}

#[test]
fn position_equality() {
    assert_eq!(Position::new(1, 2), Position::new(1, 2));
    assert_ne!(Position::new(1, 2), Position::new(2, 1));
}

// ── Property tests ──────────────────────────────────────────────────

proptest! {
    #[test]
    fn set_then_get_returns_value(
        rows in 1usize..16, cols in 1usize..16,
        r in 0usize..16, c in 0usize..16,
        val in -1000i32..1000
    ) {
        let mut m: Matrix<i32> = Matrix::new(rows, cols);
        if r < rows && c < cols {
            m.set(r, c, val);
            prop_assert_eq!(m.get(r, c), Some(&val));
        } else {
            prop_assert!(!m.set(r, c, val));
            prop_assert_eq!(m.get(r, c), None);
        }
    }

    #[test]
    fn transpose_preserves_element_count(
        rows in 1usize..16, cols in 1usize..16
    ) {
        let m = Matrix::filled(rows, cols, 0i32);
        let t = m.transpose();
        prop_assert_eq!(m.iter().count(), t.iter().count());
    }

    #[test]
    fn double_transpose_is_identity(
        rows in 1usize..8, cols in 1usize..8,
        vals in prop::collection::vec(-100i32..100, 1..65)
    ) {
        let rows = rows.min(8);
        let cols = cols.min(8);
        let mut m: Matrix<i32> = Matrix::new(rows, cols);
        for (i, &v) in vals.iter().take(rows * cols).enumerate() {
            let r = i / cols;
            let c = i % cols;
            m.set(r, c, v);
        }
        prop_assert_eq!(m.transpose().transpose(), m);
    }

    #[test]
    fn grid_set_then_occupied_contains(
        rows in 1usize..8, cols in 1usize..8,
        r in 0usize..8, c in 0usize..8
    ) {
        let mut g: Grid<i32> = Grid::new(rows, cols);
        if r < rows && c < cols {
            g.set_at(r, c, 42);
            let positions: Vec<_> = g.occupied().map(|(p, _)| p).collect();
            prop_assert!(positions.contains(&Position::new(r, c)));
        }
    }

    #[test]
    fn grid_clear_removes_from_occupied(
        rows in 1usize..8, cols in 1usize..8,
        r in 0usize..8, c in 0usize..8
    ) {
        let mut g: Grid<i32> = Grid::new(rows, cols);
        if r < rows && c < cols {
            g.set_at(r, c, 42);
            g.clear_at(r, c);
            let positions: Vec<_> = g.occupied().map(|(p, _)| p).collect();
            prop_assert!(!positions.contains(&Position::new(r, c)));
        }
    }
}
