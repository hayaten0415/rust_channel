use std::sync::Arc;
use std::mem::MaybeUninit;
use std::sync::atomic::AtomicBool;
use std::cell::UnsafeCell;
use std::sync::atomic::Ordering::Release;
use std::sync::atomic::Ordering::Acquire;
use std::sync::atomic::Ordering::Relaxed;
use std::thread;

pub struct Sender<T> {
    channel: Arc<Channel<T>>
}

pub struct  Receiver<T> {
    channel: Arc<Channel<T>>
}

struct Channel<T> {
    message: UnsafeCell<MaybeUninit<T>>,
    ready: AtomicBool,
}

unsafe impl<T> Sync for Channel<T> where T: Send {}

pub fn channel<T>() -> (Sender<T>, Receiver<T>) {
    let a = Arc::new(Channel {
        message: UnsafeCell::new(MaybeUninit::uninit()), 
        ready: AtomicBool::new(false),
    });
    (Sender { channel: a.clone() }, Receiver { channel: a })
}

impl<T> Sender<T>  {
    /// This never panics. :)
    pub fn send(self, message: T) {
        unsafe {(*self.channel.message.get()).write(message)};
        self.channel.ready.store(true, Release);
    }
}

impl<T> Receiver<T> {
    pub fn is_ready(&self) -> bool {
        self.channel.ready.load(Relaxed)
    }

    pub fn receive(self) -> T {
        if !self.channel.ready.swap(false, Acquire) {
            panic!("no message available!");
        }
        unsafe {(*self.channel.message.get()).assume_init_read()}
    }
}

impl<T> Drop for Channel<T> {
    fn drop(&mut self) {
        if *self.ready.get_mut() {
            unsafe{self.message.get_mut().assume_init_drop()}
        }
    }
}

#[test]
fn main() {
    thread::scope(|s| {
        let (sender, receiver) = channel();
        let t = thread::current();
        s.spawn(move || {
            sender.send("hello world!");
            t.unpark();
        });
        while !receiver.is_ready() {
            thread::park();
        }
        assert_eq!(receiver.receive(), "hello world!");
    });
}