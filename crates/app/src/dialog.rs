//! File dialogs that do not block the UI thread.
//!
//! A blocking dialog (`rfd::FileDialog`) runs a nested modal loop inside the egui
//! frame. On macOS, resizing that dialog sends events to winit while it is still
//! handling the frame, and winit aborts the app. The async dialog opens as a sheet
//! on the app window instead (on Windows and Linux, on its own thread), so the event
//! loop keeps running; a thread waits for the answer and the UI polls it.

use std::future::Future;
use std::path::PathBuf;
use std::pin::pin;
use std::sync::Arc;
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::task::{Context, Poll, Wake, Waker};
use std::thread::{self, Thread};

use eframe::egui;

/// What the chosen path is for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Purpose {
    /// A song file to separate.
    Song,
    /// A folder of stems to open.
    Stems,
    /// The folder separated songs are written to.
    Library,
}

/// An open dialog. Dropping it ignores its answer.
pub struct Dialog {
    pub purpose: Purpose,
    answer: Receiver<Option<PathBuf>>,
}

impl Dialog {
    /// Shows `dialog` and picks a file, or a folder if `folder` is set.
    pub fn open(
        purpose: Purpose,
        dialog: rfd::AsyncFileDialog,
        folder: bool,
        ctx: &egui::Context,
    ) -> Self {
        // The future is created here, on the UI thread: on macOS that is where the
        // sheet is shown. Only waiting for it happens on another thread.
        let pick: std::pin::Pin<Box<dyn Future<Output = Option<rfd::FileHandle>> + Send>> =
            if folder {
                Box::pin(dialog.pick_folder())
            } else {
                Box::pin(dialog.pick_file())
            };
        let (send, answer) = mpsc::channel();
        let ctx = ctx.clone();
        thread::spawn(move || {
            let path = block_on(pick).map(PathBuf::from);
            let _ = send.send(path);
            ctx.request_repaint();
        });
        Self { purpose, answer }
    }

    /// `None` while the dialog is open; then the chosen path, or `Some(None)` if it
    /// was cancelled.
    pub fn poll(&self) -> Option<Option<PathBuf>> {
        match self.answer.try_recv() {
            Ok(path) => Some(path),
            Err(TryRecvError::Empty) => None,
            Err(TryRecvError::Disconnected) => Some(None),
        }
    }
}

/// Runs `future` to completion on this thread.
fn block_on<F: Future>(future: F) -> F::Output {
    struct Unpark(Thread);
    impl Wake for Unpark {
        fn wake(self: Arc<Self>) {
            self.0.unpark();
        }
    }
    let waker = Waker::from(Arc::new(Unpark(thread::current())));
    let mut cx = Context::from_waker(&waker);
    let mut future = pin!(future);
    loop {
        if let Poll::Ready(output) = future.as_mut().poll(&mut cx) {
            return output;
        }
        thread::park();
    }
}
