use std::io;
use types::*;

pub trait Sequence {
    fn remains(&self) -> usize;
    fn skip_to(&mut self, doc_id: DocId) -> Option<DocId>;
    fn skip_n(&mut self, n: usize) -> Option<DocId>;
    fn current_position(&self) -> Option<usize>;
    fn subsequence(&self, start: usize, len: usize) -> Self;

    fn next(&mut self) -> Option<DocId> {
        self.skip_n(1)
    }
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
