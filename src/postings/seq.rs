use std::io;
use types::*;

pub trait Sequence: Clone {
    fn remains(&self) -> usize;
    fn subsequence(&self, start: usize, len: usize) -> Self;
    fn current(&self) -> Option<DocId>;
    fn next(&mut self) -> Option<DocId>;

    fn move_n(&mut self, mut n: usize) -> Option<DocId> {
        while n > 0 {
            n -= 1;
            let _ = self.next();
        }
        self.current()
    }

    fn move_to(&mut self, doc_id: DocId) -> usize {
        if let Some(x) = self.current() {
            if x == doc_id {
                return 0;
            }
        }

        let mut skipped = 0;
        while let Some(x) = self.next() {
            skipped += 1;
            if x >= doc_id {
                return skipped
            }
        }
        skipped
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
