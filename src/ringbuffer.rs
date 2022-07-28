use std::collections::VecDeque;

#[derive(Debug)]
pub struct RingBuffer<T> {
    capacity: usize,
    inner: VecDeque<T>,
}

impl<T> RingBuffer<T> {
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity,
            inner: VecDeque::new(),
        }
    }

    pub fn push(&mut self, item: T) {
        if self.inner.len() >= self.capacity {
            self.inner.pop_front();
            self.inner.push_back(item);
            debug_assert!(self.inner.len() <= self.capacity);
        } else {
            self.inner.push_back(item);
        }
    }

    pub fn pop(&mut self) -> Option<T> {
        self.inner.pop_front()
    }

    pub fn inner(&self) -> &VecDeque<T> {
        &self.inner
    }

    pub fn inner_mut(&mut self) -> &mut VecDeque<T> {
        &mut self.inner
    }
}
