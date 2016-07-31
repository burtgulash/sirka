use std::io;
use types::*;

pub trait Sequence: Clone {
    fn remains(&self) -> usize;
    fn next_position(&self) -> usize;
    fn subsequence(&self, start: usize, len: usize) -> Self;
    fn next(&mut self) -> Option<DocId>;

    fn skip_n(&mut self, mut n: usize) -> Option<DocId> {
        let mut next = None;
        while n > 0 {
            next = self.next();
            if next.is_none() {
                break;
            }
            n -= 1;
        }
        next
    }

    fn skip_to(&mut self, doc_id: DocId) -> (Option<DocId>, usize) {
        let mut skipped = 0;
        while let Some(x) = self.next() {
            skipped += 1;
            if x >= doc_id {
                return (Some(x), skipped)
            }
        }
        (None, skipped)
    }

    fn collect(&mut self) -> Vec<DocId> {
        let mut res = Vec::with_capacity(self.remains());
        while let Some(x) = self.next() {
            res.push(x);
        }
        res
    }
}

pub trait SequenceEncoder {
    fn write(&mut self, doc_id: DocId) -> io::Result<usize>;
    fn write_sequence<S: Sequence>(&mut self, seq: S) -> io::Result<usize>;
}

pub trait SequenceStorage<'a> {
    type Sequence: Sequence + 'a;
    fn to_sequence(&self) -> Self::Sequence;
}

// pub trait SequenceDecoder {
//     // TODO fn read() 
// }
