use std::collections::VecDeque;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use crate::coroutine::Coroutine;

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

pub struct TimerEntry<T> {
    time: u64,
    dequeue: VecDeque<T>,
}

impl<T> TimerEntry<T> {
    pub fn new(time: u64) -> Self {
        TimerEntry { time, dequeue: VecDeque::new() }
    }

    pub fn len(&self) -> usize {
        self.dequeue.len()
    }

    pub fn get_time(&self) -> u64 {
        self.time
    }

    pub fn pop_front(&mut self) -> Option<T> {
        self.dequeue.pop_front()
    }

    pub fn push_back(&mut self, t: T) {
        self.dequeue.push_back(t)
    }
}

pub struct TimerList<T> {
    dequeue: VecDeque<TimerEntry<T>>,
}

impl<T> TimerList<T> {
    pub fn new() -> Self {
        TimerList {
            dequeue: VecDeque::new(),
        }
    }

    pub fn len(&self) -> usize {
        self.dequeue.len()
    }

    pub fn insert(&mut self, time: u64, t: T) {
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

    pub fn front(&self) -> Option<&TimerEntry<T>> {
        self.dequeue.front()
    }

    pub fn pop_front(&mut self) -> Option<TimerEntry<T>> {
        self.dequeue.pop_front()
    }
}

#[cfg(test)]
mod tests {
    use crate::coroutine::Coroutine;
    use crate::stack::ProtectedFixedSizeStack;
    use crate::timer;
    use crate::timer::TimerList;

    #[test]
    fn now() {
        println!("{}", timer::now());
    }

    lazy_static! {
        static ref STACK: ProtectedFixedSizeStack = ProtectedFixedSizeStack::new(2048).expect("allocate stack failed !");
    }

    #[test]
    fn timer_list() {
        let mut list = TimerList::new();
        assert_eq!(list.len(), 0);
        let coroutine = Coroutine::new(&STACK, |param| {
            match param {
                Some(param) => {
                    print!("user_function {} => ", param as usize);
                }
                None => {
                    print!("user_function no param => ");
                }
            }
            param
        }, None);
        list.insert(1, coroutine);
        assert_eq!(list.len(), 1);

        let mut entry = list.pop_front().unwrap();
        assert_eq!(entry.len(), 1);
        assert!(entry.pop_front().is_some());
    }
}