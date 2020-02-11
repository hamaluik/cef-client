use std::sync::atomic::{AtomicI64, Ordering};
use std::time::SystemTime;

#[derive(Debug)]
pub struct Schedule {
    next_work_time: AtomicI64,
    start_time: SystemTime,
}

impl Schedule {
    pub fn new() -> Schedule {
        Schedule {
            next_work_time: AtomicI64::new(0),
            start_time: SystemTime::now(),
        }
    }

    pub fn schedule_work(&self, delay_ms: i64) {
        let now = SystemTime::now();
        let delta = now.duration_since(self.start_time).expect("can sub time");
        let time_stamp = delta.as_millis() as i64;
        let delay_ms = delay_ms.min(16); // wait max. 30 FPS
        let next_time = time_stamp + delay_ms;
        self.next_work_time.store(next_time, Ordering::SeqCst);
    }

    pub fn should_do_work(&self) -> bool {
        let next_time: i64 = self.next_work_time.load(Ordering::SeqCst);

        let now = SystemTime::now();
        let delta = now.duration_since(self.start_time).expect("can sub time");
        let time_stamp = delta.as_millis() as i64;

        time_stamp >= next_time
    }
}