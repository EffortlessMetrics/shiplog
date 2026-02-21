//! Processor utilities for shiplog.
//!
//! This crate provides processor implementations for transforming and handling data.

use std::marker::PhantomData;

/// A simple processor that transforms input to output
pub struct Processor<I, O> {
    name: String,
    transform: fn(I) -> O,
}

impl<I, O> Processor<I, O> {
    pub fn new(name: &str, transform: fn(I) -> O) -> Self {
        Self {
            name: name.to_string(),
            transform,
        }
    }

    pub fn process(&self, input: I) -> O {
        (self.transform)(input)
    }

    pub fn name(&self) -> &str {
        &self.name
    }
}

/// A batch processor that processes items in batches
pub struct BatchProcessor<T> {
    batch_size: usize,
    name: String,
    _phantom: PhantomData<T>,
}

impl<T> BatchProcessor<T> {
    pub fn new(batch_size: usize) -> Self {
        Self {
            batch_size,
            name: "batch-processor".to_string(),
            _phantom: PhantomData,
        }
    }

    pub fn with_name(mut self, name: &str) -> Self {
        self.name = name.to_string();
        self
    }

    pub fn process_batch<F>(&self, items: &[T], f: F) -> Vec<T>
    where
        F: Fn(&T) -> T,
    {
        items.iter().map(f).collect()
    }

    pub fn batch_size(&self) -> usize {
        self.batch_size
    }

    pub fn name(&self) -> &str {
        &self.name
    }
}

/// A stateful processor that maintains state between processing
pub struct StatefulProcessor<T, S> {
    state: S,
    process_fn: fn(S, T) -> (S, T),
}

impl<T, S: Clone> StatefulProcessor<T, S> {
    pub fn new(state: S, process_fn: fn(S, T) -> (S, T)) -> Self {
        Self { state, process_fn }
    }

    pub fn process(&mut self, input: T) -> T {
        let state_clone = self.state.clone();
        let (new_state, output) = (self.process_fn)(state_clone, input);
        self.state = new_state;
        output
    }

    pub fn state(&self) -> &S {
        &self.state
    }

    pub fn reset(&mut self, state: S) {
        self.state = state;
    }
}

/// A pipeline that chains multiple processors together
pub struct Pipeline<T> {
    stages: Vec<Box<dyn Fn(T) -> T>>,
    name: String,
}

impl<T: Clone> Pipeline<T> {
    pub fn new() -> Self {
        Self {
            stages: Vec::new(),
            name: "pipeline".to_string(),
        }
    }

    pub fn with_name(mut self, name: &str) -> Self {
        self.name = name.to_string();
        self
    }

    pub fn add_stage<F>(mut self, stage: F) -> Self
    where
        F: Fn(T) -> T + 'static,
    {
        self.stages.push(Box::new(stage));
        self
    }

    pub fn execute(&self, input: T) -> T {
        self.stages.iter().fold(input, |acc, stage| stage(acc))
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn len(&self) -> usize {
        self.stages.len()
    }

    pub fn is_empty(&self) -> bool {
        self.stages.is_empty()
    }
}

impl<T: Clone> Default for Pipeline<T> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_processor() {
        let processor = Processor::new("doubler", |x: i32| x * 2);

        assert_eq!(processor.process(5), 10);
        assert_eq!(processor.name(), "doubler");
    }

    #[test]
    fn test_batch_processor() {
        let processor: BatchProcessor<i32> = BatchProcessor::new(10);

        let items = vec![1, 2, 3];
        let result = processor.process_batch(&items, |&x| x * 2);

        assert_eq!(result, vec![2, 4, 6]);
    }

    #[test]
    fn test_batch_processor_with_name() {
        let processor = BatchProcessor::<i32>::new(5).with_name("custom-processor");

        assert_eq!(processor.name(), "custom-processor");
        assert_eq!(processor.batch_size(), 5);
    }

    #[test]
    fn test_stateful_processor() {
        let mut processor =
            StatefulProcessor::new(0, |state: i32, item: i32| (state + item, item * 2));

        assert_eq!(processor.process(5), 10);
        assert_eq!(processor.state(), &5);

        assert_eq!(processor.process(3), 6);
        assert_eq!(processor.state(), &8);
    }

    #[test]
    fn test_stateful_processor_reset() {
        let mut processor = StatefulProcessor::new(0, |state: i32, item: i32| (state + item, item));

        processor.process(5);
        assert_eq!(processor.state(), &5);

        processor.reset(0);
        assert_eq!(processor.state(), &0);
    }

    #[test]
    fn test_pipeline() {
        let pipeline = Pipeline::<i32>::new()
            .add_stage(|x| x + 1)
            .add_stage(|x| x * 2)
            .add_stage(|x| x - 3);

        // (5 + 1) * 2 - 3 = 9
        assert_eq!(pipeline.execute(5), 9);
    }

    #[test]
    fn test_pipeline_empty() {
        let pipeline: Pipeline<i32> = Pipeline::new();

        assert_eq!(pipeline.execute(5), 5);
        assert!(pipeline.is_empty());
    }

    #[test]
    fn test_pipeline_with_name() {
        let pipeline = Pipeline::<i32>::new()
            .with_name("test-pipeline")
            .add_stage(|x| x);

        assert_eq!(pipeline.name(), "test-pipeline");
    }
}
