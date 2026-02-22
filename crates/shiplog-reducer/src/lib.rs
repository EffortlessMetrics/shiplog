//! Reducer utilities (like Redux) for shiplog.
//!
//! This crate provides reducer implementations for state management patterns.

/// A reducer function that takes current state and an action, returning new state
pub type Reducer<State, Action> = fn(State, &Action) -> State;

/// A simple store that holds state and applies reducers
#[allow(clippy::type_complexity)]
pub struct Store<State, Action> {
    state: State,
    reducer: Box<dyn Fn(State, &Action) -> State>,
}

impl<State, Action> Store<State, Action> {
    /// Create a new store with initial state and a reducer
    pub fn new(initial_state: State, reducer: impl Fn(State, &Action) -> State + 'static) -> Self {
        Self {
            state: initial_state,
            reducer: Box::new(reducer),
        }
    }

    /// Get the current state
    pub fn get_state(&self) -> &State {
        &self.state
    }

    /// Dispatch an action to update the state
    pub fn dispatch(&mut self, action: &Action)
    where
        State: Clone,
    {
        let current = self.state.clone();
        let new_state = (self.reducer)(current, action);
        self.state = new_state;
    }
}

/// A reducer that combines multiple reducers into one
pub fn combine_reducers<State, Action>(
    reducers: Vec<fn(State, &Action) -> State>,
) -> impl Fn(State, &Action) -> State
where
    State: Clone,
{
    move |state: State, action: &Action| {
        reducers
            .iter()
            .fold(state, |acc, reducer| reducer(acc, action))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Debug, PartialEq, Default)]
    struct CounterState {
        count: i32,
    }

    impl CounterState {
        fn new(count: i32) -> Self {
            Self { count }
        }
    }

    #[derive(Debug, Clone)]
    enum CounterAction {
        Increment,
        Decrement,
        Add(i32),
    }

    fn counter_reducer(state: CounterState, action: &CounterAction) -> CounterState {
        match action {
            CounterAction::Increment => CounterState::new(state.count + 1),
            CounterAction::Decrement => CounterState::new(state.count - 1),
            CounterAction::Add(n) => CounterState::new(state.count + n),
        }
    }

    #[test]
    fn test_store_basic() {
        let mut store = Store::new(CounterState::new(0), counter_reducer);

        assert_eq!(store.get_state().count, 0);

        store.dispatch(&CounterAction::Increment);
        assert_eq!(store.get_state().count, 1);

        store.dispatch(&CounterAction::Increment);
        assert_eq!(store.get_state().count, 2);

        store.dispatch(&CounterAction::Decrement);
        assert_eq!(store.get_state().count, 1);
    }

    #[test]
    fn test_store_with_payload() {
        let mut store = Store::new(CounterState::new(0), counter_reducer);

        store.dispatch(&CounterAction::Add(10));
        assert_eq!(store.get_state().count, 10);

        store.dispatch(&CounterAction::Add(5));
        assert_eq!(store.get_state().count, 15);
    }

    #[test]
    fn test_combine_reducers() {
        #[derive(Clone, Debug, PartialEq, Default)]
        struct State {
            a: i32,
            b: i32,
        }

        let reducer_a = |state: State, _: &i32| State {
            a: state.a + 1,
            ..state
        };
        let reducer_b = |state: State, _: &i32| State {
            b: state.b + 10,
            ..state
        };

        let combined = combine_reducers(vec![reducer_a, reducer_b]);

        let state = State::default();
        let result = combined(state, &1);

        assert_eq!(result.a, 1);
        assert_eq!(result.b, 10);
    }

    #[test]
    fn test_combine_reducers_multiple_actions() {
        #[derive(Clone, Debug, PartialEq, Default)]
        struct State {
            value: i32,
        }

        let increment = |state: State, _: &i32| State {
            value: state.value + 1,
        };
        let double = |state: State, _: &i32| State {
            value: state.value * 2,
        };

        let combined = combine_reducers(vec![increment, double]);

        // 0 -> +1 = 1 -> *2 = 2
        let result = combined(State::default(), &0);
        assert_eq!(result.value, 2);

        // 5 -> +1 = 6 -> *2 = 12
        let result = combined(State { value: 5 }, &0);
        assert_eq!(result.value, 12);
    }
}
