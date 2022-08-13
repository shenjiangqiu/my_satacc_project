use crate::sim::{ChannelBuilder, InOutPort, SimComponent};

use super::{wating_task::WaitingTask, SataccStatus};
pub trait IcntMessage {
    fn get_target_port(&self) -> usize;
}
#[derive(Debug)]
pub struct SimpleIcnt<T> {
    pub ports: Vec<InOutPort<T>>,
    in_transit_messages: WaitingTask<T>,
    row_size: usize,
}

impl<T> SimpleIcnt<T> {
    pub fn new(ports: Vec<InOutPort<T>>) -> Self {
        let num_ports = ports.len();
        let ports_sqrt = (num_ports as f64).sqrt().floor() as usize;
        let ports_sqrt = if ports_sqrt == 0 { 1 } else { ports_sqrt };

        SimpleIcnt {
            ports,
            in_transit_messages: WaitingTask::new(),
            row_size: ports_sqrt,
        }
    }
    pub fn new_with_config(
        n_ports: usize,
        channel_size: usize,
        channel_builder: &ChannelBuilder,
    ) -> (Self, Vec<InOutPort<T>>) {
        let ports = (0..n_ports)
            .map(|_| {
                let (output_base, input_icnt) = channel_builder.sim_channel(channel_size);
                let (output_icnt, input_base) = channel_builder.sim_channel(channel_size);
                ((input_icnt, output_icnt), (input_base, output_base))
            })
            .fold(
                (vec![], vec![]),
                |(mut icnt_port, mut base_ports),
                 ((input_icnt, output_icnt), (input_base, output_base))| {
                    icnt_port.push(InOutPort {
                        in_port: input_icnt,
                        out_port: output_icnt,
                    });
                    base_ports.push(InOutPort {
                        in_port: input_base,
                        out_port: output_base,
                    });
                    (icnt_port, base_ports)
                },
            );
        let icnt_port = ports.0;
        let base_port = ports.1;

        let icnt = SimpleIcnt::new(icnt_port);
        (icnt, base_port)
    }
}
#[derive(Debug)]
pub struct IcntMsgWrapper<T> {
    pub msg: T,
    pub mem_target_port: usize,
}
impl<T> IcntMessage for IcntMsgWrapper<T> {
    fn get_target_port(&self) -> usize {
        self.mem_target_port
    }
}
impl<T> SimComponent for SimpleIcnt<T>
where
    T: IcntMessage,
{
    type SharedStatus = SataccStatus;
    fn update(&mut self, _: &mut Self::SharedStatus, current_cycle: usize) -> bool {
        let mut busy = !self.in_transit_messages.is_empty();

        // from input to icnt transit
        for (
            input_port,
            InOutPort {
                in_port,
                out_port: _,
            },
        ) in self.ports.iter_mut().enumerate()
        {
            if let Ok(message) = in_port.recv() {
                let output_port = message.get_target_port();
                let input_row = input_port / self.row_size;
                let input_col = input_port % self.row_size;
                let output_row = output_port / self.row_size;
                let output_col = output_port % self.row_size;
                let cycle_to_go = input_row.abs_diff(output_row) + input_col.abs_diff(output_col);
                self.in_transit_messages
                    .push(message, current_cycle + cycle_to_go);
                busy = true;
            }
        }

        // from icnt to output
        while let Some((leaving_cycle, message)) = self.in_transit_messages.pop() {
            if leaving_cycle > current_cycle {
                self.in_transit_messages.push(message, leaving_cycle);
                break;
            } else {
                let output_port = message.get_target_port();
                match self.ports[output_port].out_port.send(message) {
                    Ok(_) => {
                        busy = true;
                    }
                    Err(message) => {
                        self.in_transit_messages.push(message, leaving_cycle);
                        break;
                    }
                }
            }
        }

        busy
    }
}

#[cfg(test)]
mod icnt_test {

    use super::*;
    #[derive(Debug)]
    struct TestMessage {
        output_id: usize,
    }
    impl IcntMessage for TestMessage {
        fn get_target_port(&self) -> usize {
            self.output_id
        }
    }
    #[test]
    fn icnt_test() {
        let channel_builder = ChannelBuilder::new();
        let ports = (0..4)
            .map(|_i| {
                let (output_base, input_icnt) = channel_builder.sim_channel(10);
                let (output_icnt, input_base) = channel_builder.sim_channel(10);
                ((input_icnt, output_icnt), (input_base, output_base))
            })
            .fold(
                (vec![], vec![]),
                |(mut icnt_port, mut base_ports),
                 ((input_icnt, output_icnt), (input_base, output_base))| {
                    icnt_port.push(InOutPort {
                        in_port: input_icnt,
                        out_port: output_icnt,
                    });
                    base_ports.push(InOutPort {
                        in_port: input_base,
                        out_port: output_base,
                    });
                    (icnt_port, base_ports)
                },
            );
        let icnt_port = ports.0;
        let base_port = ports.1;

        let mut icnt = SimpleIcnt::new(icnt_port);
        base_port[0]
            .out_port
            .send(TestMessage { output_id: 3 })
            .unwrap();
        let mut sim_statu = SataccStatus::default();
        let mut current_cycle = 0;
        while let Err(_) = base_port[3].in_port.recv() {
            icnt.update(&mut sim_statu, current_cycle);
            current_cycle += 1;
        }
        println!("{:?}", current_cycle);
    }
}
