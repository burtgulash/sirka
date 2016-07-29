use types::*;

pub trait Sequence {
    fn remains(&self) -> usize;
    fn move_to(&mut self, doc_id: DocId);
    fn move_n(&mut self, n: usize);
    fn current(&self) -> Option<DocId>;
    fn current_position(&self) -> usize;
    fn subsequence(&self, start: usize, len: usize) -> Self;
}
