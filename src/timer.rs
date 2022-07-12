use std::collections::VecDeque;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use object_list::ObjectList;

const NANOS_PER_SEC: u64 = 1_000_000_000;

// get the current wall clock in ns
#[inline]
pub fn now() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH)
        .expect("1970-01-01 00:00:00 UTC was {} seconds ago!").as_nanos() as u64
}

#[inline]
fn dur_to_ns(dur: Duration) -> u64 {
    // Note that a duration is a (u64, u32) (seconds, nanoseconds) pair
    dur.as_secs()
        .saturating_mul(NANOS_PER_SEC)
        .saturating_add(u64::from(dur.subsec_nanos()))
}

pub(crate) fn get_timeout_time(dur: Duration) -> u64 {
    let interval = dur_to_ns(dur);
    return now() + interval;
}

#[derive(Debug)]
pub struct TimerEntry {
    time: u64,
    dequeue: ObjectList,
}

impl TimerEntry {
    pub fn new(time: u64) -> Self {
        TimerEntry { time, dequeue: ObjectList::new() }
    }

    pub fn len(&self) -> usize {
        self.dequeue.len()
    }

    pub fn get_time(&self) -> u64 {
        self.time
    }

    pub fn pop_front<T>(&mut self) -> Option<T> {
        self.dequeue.pop_front()
    }

    pub fn push_back<T>(&mut self, t: T) {
        self.dequeue.push_back(t)
    }
}

#[derive(Debug)]
pub struct TimerList {
    dequeue: VecDeque<TimerEntry>,
}

impl TimerList {
    pub fn new() -> Self {
        TimerList {
            dequeue: VecDeque::new(),
        }
    }

    pub fn len(&self) -> usize {
        self.dequeue.len()
    }

    pub fn insert<T>(&mut self, time: u64, t: T) {
        let index = self.dequeue.binary_search_by(|x| x.time.cmp(&time))
            .unwrap_or_else(|x| x);
        match self.dequeue.get_mut(index) {
            Some(entry) => {
                entry.push_back(t);
            }
            None => {
                let mut entry = TimerEntry::new(time);
                entry.push_back(t);
                self.dequeue.insert(index, entry);
            }
        }
    }

    pub fn front(&self) -> Option<&TimerEntry> {
        self.dequeue.front()
    }

    pub fn pop_front(&mut self) -> Option<TimerEntry> {
        self.dequeue.pop_front()
    }
}

#[cfg(test)]
mod tests {
    use crate::timer;
    use crate::timer::TimerList;

    #[test]
    fn now() {
        println!("{}", timer::now());
    }

    #[test]
    fn timer_list() {
        let mut list = TimerList::new();
        assert_eq!(list.len(), 0);
        list.insert(1, String::from("data can be everything"));
        assert_eq!(list.len(), 1);

        let mut entry = list.pop_front().unwrap();
        assert_eq!(entry.len(), 1);
        assert!(entry.pop_front::<i32>().is_some());
    }
}