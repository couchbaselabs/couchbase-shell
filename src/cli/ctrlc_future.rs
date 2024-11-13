use nu_protocol::Signals;
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, Waker};
use std::thread;
use std::time::Duration;

pub struct CtrlcFuture {
    state: Arc<Mutex<CtrlcState>>,
}

struct CtrlcState {
    interrupt: Signals,
    waker: Option<Waker>,
    halt: Arc<AtomicBool>,
}

impl CtrlcFuture {
    pub fn new(signals: Signals) -> CtrlcFuture {
        let state = Arc::new(Mutex::new(CtrlcState {
            interrupt: signals,
            waker: None,
            halt: Arc::new(AtomicBool::new(false)),
        }));

        let state_clone = state.clone();
        thread::spawn(move || loop {
            let mut state = state_clone.lock().unwrap();
            if state.halt.load(Ordering::SeqCst) {
                return;
            }
            if state.interrupt.interrupted() {
                if let Some(waker) = state.waker.take() {
                    waker.wake()
                }
            }
            // Release the mutex as it won't go out of scope.
            drop(state);

            thread::sleep(Duration::from_millis(10));
        });

        CtrlcFuture { state }
    }
}

impl Future for CtrlcFuture {
    type Output = ();

    fn poll(self: Pin<&mut Self>, ctx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut state = self.state.lock().unwrap();
        if state.interrupt.interrupted() {
            Poll::Ready(())
        } else {
            state.waker = Some(ctx.waker().clone());
            Poll::Pending
        }
    }
}

impl Drop for CtrlcFuture {
    fn drop(&mut self) {
        let state = self.state.lock().unwrap();
        state.halt.store(true, Ordering::SeqCst);
    }
}
