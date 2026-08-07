use super::*;

#[derive(Debug)]
pub(crate) struct Channel<T> {
  receiver: UnboundedReceiver<T>,
  sender: UnboundedSender<T>,
}

impl<T> Channel<T> {
  pub(crate) fn new() -> Self {
    let (sender, receiver) = mpsc::unbounded_channel();

    Self { receiver, sender }
  }

  pub(crate) async fn recv(&mut self) -> Option<T> {
    self.receiver.recv().await
  }

  pub(crate) fn sender(&self) -> UnboundedSender<T> {
    self.sender.clone()
  }

  pub(crate) fn try_recv(&mut self) -> Option<T> {
    self.receiver.try_recv().ok()
  }
}
