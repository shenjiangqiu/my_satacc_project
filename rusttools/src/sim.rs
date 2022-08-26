#![allow(non_snake_case)]
use std::{cell::UnsafeCell, collections::VecDeque, rc::Rc};

pub trait SimComponent {
    type SharedStatus;
    /// update the component, return(busy, updated)
    fn update(
        &mut self,
        shared_status: &mut Self::SharedStatus,
        current_cycle: usize,
    ) -> (bool, bool);
}
pub trait Connectable: SimComponent + Sized {
    fn connect<T: SimComponent<SharedStatus = Self::SharedStatus> + Sized>(
        self,
        other: T,
    ) -> AndSim<Self, T>;
}

impl<Status> SimComponent for Box<dyn SimComponent<SharedStatus = Status>> {
    type SharedStatus = Status;
    fn update(
        &mut self,
        shared_status: &mut Self::SharedStatus,
        current_cycle: usize,
    ) -> (bool, bool) {
        self.as_mut().update(shared_status, current_cycle)
    }
}

impl<U> Connectable for U
where
    U: SimComponent + Sized,
{
    fn connect<T: SimComponent<SharedStatus = Self::SharedStatus> + Sized>(
        self,
        other: T,
    ) -> AndSim<Self, T> {
        AndSim { a: self, b: other }
    }
}
#[derive(Debug)]
pub struct AndSim<A, B>
where
    A: SimComponent,
    B: SimComponent<SharedStatus = A::SharedStatus>,
{
    a: A,
    b: B,
}

impl<A, B> AndSim<A, B>
where
    A: SimComponent,
    B: SimComponent<SharedStatus = A::SharedStatus>,
{
    pub fn new(a: A, b: B) -> AndSim<A, B> {
        AndSim { a, b }
    }
}

impl<A, B> SimComponent for AndSim<A, B>
where
    A: SimComponent,
    B: SimComponent<SharedStatus = A::SharedStatus>,
{
    type SharedStatus = A::SharedStatus;
    fn update(
        &mut self,
        shared_status: &mut Self::SharedStatus,
        current_cycle: usize,
    ) -> (bool, bool) {
        let a_result = self.a.update(shared_status, current_cycle);
        let b_result = self.b.update(shared_status, current_cycle);

        (a_result.0 || b_result.0, a_result.1 || b_result.1)
    }
}
impl<T, C> SimComponent for &mut T
where
    T: SimComponent<SharedStatus = C>,
{
    type SharedStatus = C;
    fn update(
        &mut self,
        shared_status: &mut Self::SharedStatus,
        current_cycle: usize,
    ) -> (bool, bool) {
        (*self).update(shared_status, current_cycle)
    }
}

impl<T, C> SimComponent for Vec<T>
where
    T: SimComponent<SharedStatus = C>,
{
    type SharedStatus = C;
    fn update(
        &mut self,
        shared_status: &mut Self::SharedStatus,
        current_cycle: usize,
    ) -> (bool, bool) {
        // first collect to make sure all components are updated
        let result: Vec<_> = self
            .iter_mut()
            .map(|item| item.update(shared_status, current_cycle))
            .collect();
        let busy = result.iter().any(|&(busy, _)| busy);
        let updated = result.iter().any(|&(_, updated)| updated);
        (busy, updated)
    }
}
#[derive(Debug)]
pub struct SimRunner<T, S> {
    sim: T,
    shared_status: S,
    current_cycle: usize,
}
impl<T, S> SimRunner<T, S>
where
    T: SimComponent<SharedStatus = S>,
{
    pub fn new(sim: T, shared_status: S) -> SimRunner<T, S> {
        SimRunner {
            sim,
            current_cycle: 0,
            shared_status,
        }
    }
    pub fn get_sim(&self) -> &T {
        &self.sim
    }
    pub fn get_shared_status(&self) -> &S {
        &self.shared_status
    }
    pub fn get_sim_mut(&mut self) -> &mut T {
        &mut self.sim
    }
    pub fn get_shared_status_mut(&mut self) -> &mut S {
        &mut self.shared_status
    }
    pub fn run(&mut self) -> eyre::Result<()> {
        loop {
            let result = self.sim.update(&mut self.shared_status, self.current_cycle);
            match result {
                (true, true) => {
                    self.current_cycle += 1;
                }
                (true, false) => {
                    tracing::error!(
                        "simulation is busy but not updated at cycle {}",
                        self.current_cycle
                    );
                    self.current_cycle += 1;

                    return Err(eyre::eyre!(
                        "simulation is busy but not updated at cycle {}",
                        self.current_cycle
                    ));
                }
                (false, _) => {
                    // not busy, so we are done
                    break;
                }
            }
            self.current_cycle += 1;
        }
        Ok(())
    }
    pub fn get_current_cycle(&self) -> usize {
        self.current_cycle
    }
    pub fn into_inner(self) -> (T, S, usize) {
        (self.sim, self.shared_status, self.current_cycle)
    }
}
#[derive(Debug)]
pub struct SimSender<T> {
    buffer: Rc<UnsafeCell<VecDeque<T>>>,
    max_size: usize,
    current_value_size: Rc<UnsafeCell<usize>>,
}
impl<T> Clone for SimSender<T> {
    fn clone(&self) -> SimSender<T> {
        SimSender {
            buffer: self.buffer.clone(),
            max_size: self.max_size,
            current_value_size: self.current_value_size.clone(),
        }
    }
}
#[derive(Debug)]

pub struct SimReciver<T> {
    buffer: Rc<UnsafeCell<VecDeque<T>>>,
    current_value_size: Rc<UnsafeCell<usize>>,
}
impl<T> Clone for SimReciver<T> {
    fn clone(&self) -> SimReciver<T> {
        SimReciver {
            buffer: self.buffer.clone(),
            current_value_size: self.current_value_size.clone(),
        }
    }
}
#[derive(Debug)]

pub struct InOutPort<T> {
    pub in_port: SimReciver<T>,
    pub out_port: SimSender<T>,
}
impl<T> Clone for InOutPort<T> {
    fn clone(&self) -> InOutPort<T> {
        InOutPort {
            in_port: self.in_port.clone(),
            out_port: self.out_port.clone(),
        }
    }
}
pub struct ChannelBuilder {
    current_values: Rc<UnsafeCell<usize>>,
}
impl ChannelBuilder {
    pub fn new() -> ChannelBuilder {
        ChannelBuilder {
            current_values: Rc::new(UnsafeCell::new(0)),
        }
    }
    pub fn sim_channel<T>(&self, queue_len: usize) -> (SimSender<T>, SimReciver<T>) {
        let buffer = Rc::new(UnsafeCell::new(VecDeque::with_capacity(queue_len)));
        (
            SimSender::<T> {
                buffer: buffer.clone(),
                max_size: queue_len,
                current_value_size: self.current_values.clone(),
            },
            SimReciver::<T> {
                buffer,
                current_value_size: self.current_values.clone(),
            },
        )
    }
    pub fn sim_channel_array<T>(
        &self,
        queue_len: usize,
        num_queues: usize,
    ) -> (Vec<SimSender<T>>, Vec<SimReciver<T>>) {
        let mut senders = Vec::with_capacity(queue_len);
        let mut receivers = Vec::with_capacity(queue_len);
        for _ in 0..num_queues {
            let (sender, receiver) = self.sim_channel::<T>(queue_len);
            senders.push(sender);
            receivers.push(receiver);
        }
        (senders, receivers)
    }

    pub fn in_out_port<T>(&self, queue_len: usize) -> (InOutPort<T>, InOutPort<T>) {
        let (sender1, receiver1) = self.sim_channel::<T>(queue_len);
        let (sender2, receiver2) = self.sim_channel::<T>(queue_len);
        (
            InOutPort {
                in_port: receiver1,
                out_port: sender2,
            },
            InOutPort {
                in_port: receiver2,
                out_port: sender1,
            },
        )
    }
    pub fn in_out_poat_array<T>(
        &self,
        queue_len: usize,
        num_queues: usize,
    ) -> (Vec<InOutPort<T>>, Vec<InOutPort<T>>) {
        let mut senders = Vec::with_capacity(queue_len);
        let mut receivers = Vec::with_capacity(queue_len);
        for _ in 0..num_queues {
            let (sender, receiver) = self.in_out_port(queue_len);
            senders.push(sender);
            receivers.push(receiver);
        }
        (senders, receivers)
    }

    pub fn get_current_queue_size(&self) -> usize {
        unsafe { *self.current_values.get() }
    }
}

impl<T> SimSender<T> {
    pub fn have_space(&self) -> bool {
        unsafe {
            let buffer = &*self.buffer.get();
            buffer.len() < self.max_size
        }
    }
    pub fn send(&self, data: T) -> Result<(), T> {
        unsafe {
            let buffer = &mut *self.buffer.get();
            if buffer.len() >= self.max_size {
                return Err(data);
            }
            buffer.push_back(data);
            (*self.current_value_size.get()) += 1;
            Ok(())
        }
    }
}
impl<T> SimReciver<T> {
    pub fn recv(&self) -> Result<T, ()> {
        unsafe {
            let buffer = &mut *self.buffer.get();
            if buffer.is_empty() {
                return Err(());
            }
            let data = buffer.pop_front().unwrap();
            (*self.current_value_size.get()) -= 1;
            Ok(data)
        }
    }
    pub fn ret(&self, data: T) {
        unsafe {
            let buffer = &mut *self.buffer.get();
            (*self.current_value_size.get()) += 1;
            buffer.push_front(data);
        }
    }
}
impl_for_tuples_with_type!(SimComponent;update;SharedStatus;
    (A),
    (A,B),
    (A,B,C),
    (A,B,C,D),
    (A,B,C,D,E),
    (A,B,C,D,E,F),
    (A,B,C,D,E,F,G),
    (A,B,C,D,E,F,G,H),
    (A,B,C,D,E,F,G,H,I),
    (A,B,C,D,E,F,G,H,I,J),
    (A,B,C,D,E,F,G,H,I,J,K),
    (A,B,C,D,E,F,G,H,I,J,K,L),
    (A,B,C,D,E,F,G,H,I,J,K,L,M),);

#[cfg(test)]
mod test {

    use super::*;
    struct TaskSender {
        current_taks_id: usize,
        task_sender: SimSender<usize>,
    }
    impl SimComponent for TaskSender {
        type SharedStatus = ();
        fn update(&mut self, _: &mut Self::SharedStatus, _current_cycle: usize) -> (bool, bool) {
            if self.current_taks_id < 100 {
                match self.task_sender.send(self.current_taks_id) {
                    Ok(_) => {
                        self.current_taks_id += 1;
                        (true, true)
                    }
                    Err(_) => (false, false),
                }
            } else {
                (false, false)
            }
        }
    }

    struct TaskReceiver {
        task_receiver: SimReciver<usize>,
    }
    impl SimComponent for TaskReceiver {
        type SharedStatus = ();
        fn update(&mut self, _: &mut Self::SharedStatus, current_cycle: usize) -> (bool, bool) {
            match self.task_receiver.recv() {
                Ok(id) => {
                    println!("{current_cycle}:{id}");
                    (true, true)
                }
                Err(_) => (false, false),
            }
        }
    }
    #[test]
    fn sim_test() {
        let channel_builder = ChannelBuilder::new();
        let (task_sender, task_receiver) = channel_builder.sim_channel(10);
        let task_sender = TaskSender {
            current_taks_id: 0,
            task_sender,
        };
        let task_receiver = TaskReceiver { task_receiver };
        let sim = task_sender.connect(task_receiver);

        let mut sim_runner = SimRunner::new(sim, ());
        sim_runner.run().unwrap();
    }
    #[test]
    fn sim_test_box() {
        let channel_builder = ChannelBuilder::new();
        let (task_sender, task_receiver) = channel_builder.sim_channel(10);
        let task_sender = TaskSender {
            current_taks_id: 0,
            task_sender,
        };
        let task_receiver = TaskReceiver { task_receiver };
        let task_sender: Box<dyn SimComponent<SharedStatus = ()>> = Box::new(task_sender);
        let task_receiver: Box<dyn SimComponent<SharedStatus = ()>> = Box::new(task_receiver);

        let sim = task_sender.connect(task_receiver);

        let mut sim_runner = SimRunner::new(sim, ());
        sim_runner.run().unwrap();
    }

    #[test]
    fn channel_builder_test() {
        let channel_builder = ChannelBuilder::new();
        let (sender, receiver) = channel_builder.sim_channel(10);
        sender.send(1).unwrap();
        assert!(channel_builder.get_current_queue_size() == 1);
        receiver.recv().unwrap();
        assert!(channel_builder.get_current_queue_size() == 0);
        receiver.ret(1);
        assert!(channel_builder.get_current_queue_size() == 1);
    }
}
