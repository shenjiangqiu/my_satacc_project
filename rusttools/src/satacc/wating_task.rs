use std::{cmp::Reverse, collections::BinaryHeap};

/// # WatingTaskData
/// the data is sorted by the leaving cycle
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
/// # WatingTask
/// A waiting task is a task task queue that sorted by leaving cycle.
/// - it's a somple wrapper of BineryHeap. It uses Reverse to simulate a min-heap.
/// -  the data WatingTaskData is used to store the task and the leaving cycle.
///
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
    pub fn len(&self) -> usize {
        self.data.len()
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

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn waiting_task_test() {
        let mut waiting_task = WaitingTask::new();
        waiting_task.push(1, 1);
        waiting_task.push(2, 2);
        waiting_task.push(3, 3);
        let next = waiting_task.pop();
        assert_eq!(next, Some((1, 1)));
        let next = waiting_task.pop();
        assert_eq!(next, Some((2, 2)));
        waiting_task.push(2, 2);
        let next = waiting_task.pop();
        assert_eq!(next, Some((2, 2)));
        let next = waiting_task.pop();
        assert_eq!(next, Some((3, 3)));
    }
}
