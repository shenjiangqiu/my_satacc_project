use crate::sim::{SimComponent, SimReciver, SimSender};

use super::satacc_minisat_task::{SingleRoundTask, WatcherTask};

/// the task sender, which will send tasks to the watcher list unit
pub struct Trail {
    task_receiver: SimReciver<SingleRoundTask>,
    watcher_sender: Vec<SimSender<WatcherTask>>,
    current_working_task: Option<SingleRoundTask>,
    total_watcher: usize,
    level_sync: bool,
    current_level_remaining: usize,
    current_processing_level: usize,
}
impl Trail {
    pub fn new(
        watcher_sender: Vec<SimSender<WatcherTask>>,
        task_receiver: SimReciver<SingleRoundTask>,
        level_sync: bool,
        total_watcher: usize,
    ) -> Self {
        Trail {
            watcher_sender,
            task_receiver,
            current_working_task: None,
            total_watcher,
            level_sync,
            current_level_remaining: 0,
            current_processing_level: 0,
        }
    }
}

impl SimComponent for Trail {
    type SharedStatus = super::SataccStatus;
    fn update(
        &mut self,
        shared_status: &mut Self::SharedStatus,
        _current_cycle: usize,
    ) -> (bool, bool) {
        let mut busy = self.current_working_task.is_some();
        let mut updated = false;

        match self.level_sync {
            true => {
                // update current running task
                if let Some(current_task) = self.current_working_task.as_mut() {
                    if let Some(watcher_task) = current_task.pop_next_task() {
                        if watcher_task.level != self.current_processing_level {
                            // cannot send the task now, return it back
                            let new_level = watcher_task.level;
                            current_task.ret_task(watcher_task);
                            // check if all tasks in the current level are finished
                            if self.current_level_remaining
                                == shared_status.current_level_finished_tasks
                            {
                                // all tasks is done, can schedule next level
                                self.current_processing_level = new_level;
                                self.current_level_remaining = 0;
                                shared_status.current_level_finished_tasks = 0;
                                updated = true;
                                tracing::debug!(new_level, _current_cycle, "start new level");
                            }
                            // not the same level, wait for the finished
                        } else {
                            // the same level, send to watcher
                            let watcher_unit_id =
                                watcher_task.get_watcher_pe_id(self.total_watcher);
                            let total_level_tasks = watcher_task.get_total_level_tasks();
                            match self.watcher_sender[watcher_unit_id].send(watcher_task) {
                                Ok(_) => {
                                    updated = true;
                                    busy = true;
                                    self.current_level_remaining += total_level_tasks;
                                }
                                Err(watcher_task) => {
                                    current_task.ret_task(watcher_task);
                                    tracing::debug!(
                                        "send task to watcher {} failed",
                                        watcher_unit_id
                                    );
                                }
                            }
                        }
                    } else {
                        // no more tasks, finish the current task
                        busy = true;
                        updated = true;
                        self.current_working_task = None;
                    }
                }
                // get new task
                if self.current_working_task.is_none() {
                    if let Ok(single_round_task) = self.task_receiver.recv() {
                        // a new round begin, update the statistics
                        shared_status.update_single_round_task(&single_round_task);
                        self.current_working_task = Some(single_round_task);
                        busy = true;
                        updated = true;
                    }
                }
            }
            false => {
                // update current running task
                if let Some(current_task) = self.current_working_task.as_mut() {
                    if let Some(watcher_task) = current_task.pop_next_task() {
                        busy = true;
                        let watcher_unit_id = watcher_task.get_watcher_pe_id(self.total_watcher);
                        match self.watcher_sender[watcher_unit_id].send(watcher_task) {
                            Ok(_) => {
                                updated = true;
                                busy = true;
                            }
                            Err(watcher_task) => {
                                current_task.ret_task(watcher_task);
                                tracing::debug!("send task to watcher {} failed", watcher_unit_id);
                            }
                        }
                    } else {
                        // no more tasks, finish the current task
                        busy = true;
                        updated = true;
                        self.current_working_task = None;
                    }
                }
                // get new task
                if self.current_working_task.is_none() {
                    if let Ok(single_round_task) = self.task_receiver.recv() {
                        // a new round begin, update the statistics
                        shared_status.update_single_round_task(&single_round_task);
                        self.current_working_task = Some(single_round_task);
                        busy = true;
                        updated = true;
                    }
                }
            }
        };

        if busy && !updated {
            tracing::debug!("trail is busy but not updated");
        }
        (busy, updated)
    }
}
