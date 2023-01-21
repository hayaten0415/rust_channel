use std::mem::MaybeUninit;
use std::sync::atomic::AtomicBool;
use std::cell::UnsafeCell;
use std::sync::atomic::Ordering::Release;
use std::sync::atomic::Ordering::Acquire;
use std::sync::atomic::Ordering::Relaxed;
use std::thread;

pub struct Channel<T> {
    message: UnsafeCell<MaybeUninit<T>>,
    in_use: AtomicBool, // New!
    ready: AtomicBool,
}

unsafe impl<T> Sync for Channel<T> where T: Send {}

impl<T> Channel<T> {
    pub const fn new() -> Self {
        Self { 
            message: UnsafeCell::new(MaybeUninit::uninit()), 
            in_use: AtomicBool::new(false), // New!
            ready: (AtomicBool::new(false)), 
        }
    }

    /// Panics when trying to send more than one message.
    pub unsafe fn send(&self, message: T) {
        if self.in_use.swap(true, Relaxed) {
            panic!("can't send more than one message");
        }
        unsafe {(*self.message.get()).write(message) };
        self.ready.store(true, Release);
    }

    pub fn is_ready(&self) -> bool {
        self.ready.load(Relaxed)
    }

    /// Panics if no message is available yet,
    /// or if the message was already consumed.
    ///
    /// Tip: Use `is_ready` to check first.
    pub unsafe fn receive(&self) -> T {
        if !self.ready.swap(false, Acquire) {
            panic!("no message available!");
        }
        // Safety: We've just checked (and reset) the ready flag.
        unsafe { (*self.message.get()).assume_init_read() }
    }
}

impl<T> Drop for Channel<T> {
    fn drop(&mut self) {
        if *self.ready.get_mut() {
            unsafe {self.message.get_mut().assume_init_drop()}
        }
    }
}

#[test]
fn main() {
    let channel = Channel::new();
    let t = thread::current();
    thread::scope(|s| {
        s.spawn(|| {
            unsafe {channel.send("hello world!")};
            t.unpark();
        });
        while !channel.is_ready() {
            thread::park();
        }
        assert_eq!(unsafe{channel.receive()}, "hello world!");
    });
}