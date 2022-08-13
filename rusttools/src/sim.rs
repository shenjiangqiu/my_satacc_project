use std::{cell::UnsafeCell, collections::VecDeque, rc::Rc};

pub trait SimComponent {
    type SharedStatus;
    fn update(&mut self, shared_status: &mut Self::SharedStatus, current_cycle: usize) -> bool;
}
pub trait Connectable: SimComponent + Sized {
    fn connect<T>(self, other: T) -> AndSim<Self, T>
    where
        T: SimComponent<SharedStatus = Self::SharedStatus> + Sized,
    {
        AndSim::new(self, other)
    }
}

impl<Status> SimComponent for Box<dyn SimComponent<SharedStatus = Status>> {
    type SharedStatus = Status;
    fn update(&mut self, shared_status: &mut Self::SharedStatus, current_cycle: usize) -> bool {
        self.as_mut().update(shared_status, current_cycle)
    }
}

impl<T> Connectable for T where T: SimComponent + Sized {}
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
    fn update(&mut self, shared_status: &mut Self::SharedStatus, current_cycle: usize) -> bool {
        let a_result = self.a.update(shared_status, current_cycle);
        let b_result = self.b.update(shared_status, current_cycle);

        a_result || b_result
    }
}

impl<T> SimComponent for Vec<T>
where
    T: SimComponent,
{
    type SharedStatus = T::SharedStatus;
    fn update(&mut self, shared_status: &mut Self::SharedStatus, current_cycle: usize) -> bool {
        self.iter_mut()
            .map(|item| item.update(shared_status, current_cycle))
            .collect::<Vec<_>>()
            .contains(&true)
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
    pub fn run(&mut self) {
        while self.sim.update(&mut self.shared_status, self.current_cycle) {
            self.current_cycle += 1;
        }
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

#[cfg(test)]
mod test {

    use super::*;
    struct TaskSender {
        current_taks_id: usize,
        task_sender: SimSender<usize>,
    }
    impl SimComponent for TaskSender {
        type SharedStatus = ();
        fn update(&mut self, _: &mut Self::SharedStatus, _current_cycle: usize) -> bool {
            if self.current_taks_id < 100 {
                match self.task_sender.send(self.current_taks_id) {
                    Ok(_) => {
                        self.current_taks_id += 1;
                        true
                    }
                    Err(_) => false,
                }
            } else {
                false
            }
        }
    }

    struct TaskReceiver {
        task_receiver: SimReciver<usize>,
    }
    impl SimComponent for TaskReceiver {
        type SharedStatus = ();
        fn update(&mut self, _: &mut Self::SharedStatus, current_cycle: usize) -> bool {
            match self.task_receiver.recv() {
                Ok(id) => {
                    println!("{current_cycle}:{id}");
                    true
                }
                Err(_) => false,
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
        sim_runner.run();
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
        sim_runner.run();
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
