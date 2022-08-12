use std::{cmp::Reverse, collections::BinaryHeap};
#[derive(Debug)]
pub struct WaitingTaskData<T> {
    pub task: T,
    pub leaving_cycle: usize,
}
impl<T> PartialEq for WaitingTaskData<T> {
    fn eq(&self, other: &Self) -> bool {
        self.leaving_cycle == other.leaving_cycle
    }
}
impl<T> Eq for WaitingTaskData<T> {}
impl<T> PartialOrd for WaitingTaskData<T> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.leaving_cycle.cmp(&other.leaving_cycle))
    }
}
impl<T> Ord for WaitingTaskData<T> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.leaving_cycle.cmp(&other.leaving_cycle)
    }
}
#[derive(Debug)]

pub struct WaitingTask<T> {
    data: BinaryHeap<Reverse<WaitingTaskData<T>>>,
}

impl<T> WaitingTask<T> {
    pub fn new() -> Self {
        WaitingTask {
            data: BinaryHeap::new(),
        }
    }
    pub fn push(&mut self, task: T, leaving_cycle: usize) {
        self.data.push(Reverse(WaitingTaskData {
            task,
            leaving_cycle,
        }));
    }
    pub fn pop(&mut self) -> Option<(usize, T)> {
        self.data.pop().map(
            |Reverse(WaitingTaskData {
                 task,
                 leaving_cycle,
             })| (leaving_cycle, task),
        )
    }
    #[allow(dead_code)]
    pub fn peek(&self) -> Option<(usize, &T)> {
        self.data.peek().map(
            |Reverse(WaitingTaskData {
                 task,
                 leaving_cycle,
             })| (*leaving_cycle, task),
        )
    }
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }
}
