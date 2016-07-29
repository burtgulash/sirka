use std::io;
use types::*;

pub trait SequenceSpawner {
    type Sequence: Sequence;
    fn spawn(&self, start: usize, len: usize) -> Self::Sequence;
}

pub trait Sequence: Clone {
    fn put(&mut self);
    fn skip_to(&mut self, doc_id: DocId);
    fn skip_n(&mut self, n: usize);
    fn current(&self) -> Option<DocId>;
    fn current_position(&self) -> usize;
    fn write_current(&self, w: &mut io::Write) -> io::Result<usize>;
}
