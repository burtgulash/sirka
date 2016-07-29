use std::io;
use types::*;

pub trait Sequence {
    fn remains(&self) -> usize;
    fn move_to(&mut self, doc_id: DocId);
    fn move_n(&mut self, n: usize);
    fn current(&self) -> Option<DocId>;
    fn current_position(&self) -> usize;
    fn subsequence(&self, start: usize, len: usize) -> Self;
}

pub trait SequenceEncoder {
    fn write(&mut self, doc_id: DocId) -> io::Result<usize>;
    fn write_sequence<S: Sequence>(&mut self, seq: S) -> io::Result<usize>;
}

pub trait SequenceStorage {
    type Sequence: Sequence;
    fn to_sequence(&self) -> Self::Sequence;
}

// pub trait SequenceDecoder {
//     // TODO fn read() 
// }
