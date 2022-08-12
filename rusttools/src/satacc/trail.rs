use crate::sim::{SimComponent, SimReciver, SimSender};

use super::satacc_minisat_task::{SingleRoundTask, WatcherTask};

/// the task sender, which will send tasks to the watcher list unit
pub struct Trail {
    task_receiver: SimReciver<SingleRoundTask>,
    watcher_sender: Vec<SimSender<WatcherTask>>,
    current_working_task: Option<SingleRoundTask>,
    total_watcher: usize,
}
impl Trail {
    pub fn new(
        watcher_sender: Vec<SimSender<WatcherTask>>,
        task_receiver: SimReciver<SingleRoundTask>,
        total_watcher: usize,
    ) -> Self {
        Trail {
            watcher_sender,
            task_receiver,
            current_working_task: None,
            total_watcher,
        }
    }
}

impl SimComponent for Trail {
    type SharedStatus = super::SataccStatus;
    fn update(&mut self, _shared_status: &mut Self::SharedStatus, _current_cycle: usize) -> bool {
        let mut busy = self.current_working_task.is_some();
        // update current running task
        if let Some(current_task) = self.current_working_task.as_mut() {
            if let Some(watcher_task) = current_task.pop_next_task() {
                let watcher_unit_id = watcher_task.get_watcher_pe_id(self.total_watcher);
                match self.watcher_sender[watcher_unit_id].send(watcher_task) {
                    Ok(_) => {
                        busy = true;
                    }
                    Err(watcher_task) => {
                        current_task.ret_task(watcher_task);
                    }
                }
            } else {
                // no more tasks, finish the current task
                self.current_working_task = None;
            }
        }
        // get new task
        if self.current_working_task.is_none() {
            if let Ok(single_round_task) = self.task_receiver.recv() {
                self.current_working_task = Some(single_round_task);
                busy = true;
            }
        }
        busy
    }
}