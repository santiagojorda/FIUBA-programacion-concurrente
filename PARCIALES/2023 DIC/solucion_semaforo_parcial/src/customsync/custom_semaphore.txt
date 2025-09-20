use std::sync::{Condvar, Mutex};

#[derive(Debug)]
pub struct Semaphore {
    count_lock: Mutex<usize>,
    condv: Condvar,
}

impl Semaphore {
    pub fn new(initial_value: usize) -> Self {
        Semaphore {
            count_lock: Mutex::new(initial_value),
            condv: Condvar::new(),
        }
    }

    pub fn acquire(&self) {
        let mut count = self
            .condv
            .wait_while(self.count_lock.lock().unwrap(), |count| *count <= 0)
            .unwrap();
        *count -= 1;
    }

    pub fn release(&self) {
        *self.count_lock.lock().unwrap() += 1;
        self.condv.notify_one();
    }
}

#[cfg(test)]
mod tests {
    use super::Semaphore;

    use std::{
        sync::{mpsc::channel, Arc},
        thread,
    };

    #[test]
    fn test_sem_acquire_release() {
        let s = Semaphore::new(1);
        s.acquire();
        s.release();
        s.acquire();
    }

    #[test]
    fn test_sem_as_mutex() {
        let s = Arc::new(Semaphore::new(1));
        let s2 = s.clone();
        let _t = thread::spawn(move || {
            s2.acquire();
            s2.release();
        });

        s.acquire();
        s.release();
    }

    #[test]
    fn test_sem_as_cvar() {
        // Child waits and parent signals
        let (tx, rx) = channel();
        let s = Arc::new(Semaphore::new(0));
        let s2 = s.clone();
        let _t = thread::spawn(move || {
            s2.acquire();
            tx.send(()).unwrap();
        });
        s.release();
        let _ = rx.recv();

        // Parent waits and child signals
        let (tx, rx) = channel();
        let s = Arc::new(Semaphore::new(0));
        let s2 = s.clone();
        let _t = thread::spawn(move || {
            s2.release();
            let _ = rx.recv();
        });
        s.acquire();
        tx.send(()).unwrap();
    }

    #[test]
    fn test_sem_multi_resource() {
        // Parent and child both get in the critical section at the same
        // time, and shake hands.
        let s = Arc::new(Semaphore::new(2));
        let s2 = s.clone();
        let (tx1, rx1) = channel();
        let (tx2, rx2) = channel();
        let _t = thread::spawn(move || {
            s2.acquire();
            let _ = rx2.recv();
            tx1.send(()).unwrap();
            s2.release();
        });
        s.acquire();
        tx2.send(()).unwrap();
        rx1.recv().unwrap();
        s.release();
    }
}
