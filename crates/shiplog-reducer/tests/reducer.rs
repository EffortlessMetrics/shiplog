use proptest::prelude::*;
use shiplog_reducer::{Store, combine_reducers};

// ── Helpers ────────────────────────────────────────────────────────

#[derive(Clone, Debug, PartialEq, Default)]
struct Counter {
    value: i32,
}

#[derive(Debug, Clone)]
enum Action {
    Inc,
    Dec,
    Add(i32),
    Reset,
}

fn counter_reducer(state: Counter, action: &Action) -> Counter {
    match action {
        Action::Inc => Counter {
            value: state.value + 1,
        },
        Action::Dec => Counter {
            value: state.value - 1,
        },
        Action::Add(n) => Counter {
            value: state.value + n,
        },
        Action::Reset => Counter { value: 0 },
    }
}

// ── Correctness tests ──────────────────────────────────────────────

#[test]
fn store_initial_state() {
    let store = Store::new(Counter { value: 42 }, counter_reducer);
    assert_eq!(store.get_state().value, 42);
}

#[test]
fn store_dispatch_increment() {
    let mut store = Store::new(Counter::default(), counter_reducer);
    store.dispatch(&Action::Inc);
    store.dispatch(&Action::Inc);
    store.dispatch(&Action::Inc);
    assert_eq!(store.get_state().value, 3);
}

#[test]
fn store_dispatch_decrement() {
    let mut store = Store::new(Counter { value: 10 }, counter_reducer);
    store.dispatch(&Action::Dec);
    assert_eq!(store.get_state().value, 9);
}

#[test]
fn store_dispatch_add() {
    let mut store = Store::new(Counter::default(), counter_reducer);
    store.dispatch(&Action::Add(100));
    assert_eq!(store.get_state().value, 100);
}

#[test]
fn store_dispatch_reset() {
    let mut store = Store::new(Counter { value: 999 }, counter_reducer);
    store.dispatch(&Action::Reset);
    assert_eq!(store.get_state().value, 0);
}

#[test]
fn store_multiple_actions_sequence() {
    let mut store = Store::new(Counter::default(), counter_reducer);
    store.dispatch(&Action::Add(10));
    store.dispatch(&Action::Inc);
    store.dispatch(&Action::Dec);
    store.dispatch(&Action::Add(5));
    assert_eq!(store.get_state().value, 15);
}

// ── Edge cases ─────────────────────────────────────────────────────

#[test]
fn store_no_dispatch() {
    let store = Store::new(Counter { value: 7 }, counter_reducer);
    assert_eq!(store.get_state().value, 7);
}

#[test]
fn store_add_zero() {
    let mut store = Store::new(Counter { value: 5 }, counter_reducer);
    store.dispatch(&Action::Add(0));
    assert_eq!(store.get_state().value, 5);
}

// ── combine_reducers ───────────────────────────────────────────────

#[test]
fn combine_reducers_empty() {
    let combined = combine_reducers::<i32, i32>(vec![]);
    assert_eq!(combined(42, &0), 42);
}

#[test]
fn combine_reducers_single() {
    let combined = combine_reducers(vec![|state: i32, _: &i32| state + 1]);
    assert_eq!(combined(0, &0), 1);
}

#[test]
fn combine_reducers_ordering() {
    // First add 1, then multiply by 2
    let add_one = |state: i32, _: &i32| state + 1;
    let double = |state: i32, _: &i32| state * 2;
    let combined = combine_reducers(vec![add_one, double]);

    // 0 + 1 = 1, 1 * 2 = 2
    assert_eq!(combined(0, &0), 2);
    // 5 + 1 = 6, 6 * 2 = 12
    assert_eq!(combined(5, &0), 12);
}

#[test]
fn combine_reducers_in_store() {
    let inc = |state: Counter, _: &i32| Counter {
        value: state.value + 1,
    };
    let triple = |state: Counter, _: &i32| Counter {
        value: state.value * 3,
    };
    let combined = combine_reducers(vec![inc, triple]);
    let mut store = Store::new(Counter::default(), combined);
    store.dispatch(&0);
    // (0+1)*3 = 3
    assert_eq!(store.get_state().value, 3);
    store.dispatch(&0);
    // (3+1)*3 = 12
    assert_eq!(store.get_state().value, 12);
}

// ── Property tests ─────────────────────────────────────────────────

proptest! {
    #[test]
    fn prop_inc_dec_cancel(n in 0u32..100) {
        let mut store = Store::new(Counter::default(), counter_reducer);
        for _ in 0..n {
            store.dispatch(&Action::Inc);
        }
        for _ in 0..n {
            store.dispatch(&Action::Dec);
        }
        prop_assert_eq!(store.get_state().value, 0);
    }

    #[test]
    fn prop_add_is_equivalent_to_n_increments(n in 0i32..200) {
        let mut store_add = Store::new(Counter::default(), counter_reducer);
        store_add.dispatch(&Action::Add(n));

        let mut store_inc = Store::new(Counter::default(), counter_reducer);
        for _ in 0..n {
            store_inc.dispatch(&Action::Inc);
        }

        prop_assert_eq!(store_add.get_state().value, store_inc.get_state().value);
    }

    #[test]
    fn prop_reset_always_zeros(initial in -1000i32..1000, adds in prop::collection::vec(-100i32..100, 0..20)) {
        let mut store = Store::new(Counter { value: initial }, counter_reducer);
        for a in adds {
            store.dispatch(&Action::Add(a));
        }
        store.dispatch(&Action::Reset);
        prop_assert_eq!(store.get_state().value, 0);
    }

    #[test]
    fn prop_combine_identity_reducer(x in -1000i32..1000) {
        let identity = |state: i32, _: &i32| state;
        let combined = combine_reducers(vec![identity, identity]);
        prop_assert_eq!(combined(x, &0), x);
    }
}
