use std::io;
use types::*;

pub trait Sequence: Clone {
    fn remains(&self) -> usize;
    fn subsequence(&self, start: usize, len: usize) -> Self;
    fn current(&self) -> Option<DocId>;
    fn current_unchecked(&self) -> DocId;
    fn advance(&mut self) -> Option<DocId>;

    fn move_n(&mut self, mut n: usize) -> Option<DocId> {
        while n > 0 {
            n -= 1;
            self.advance();
        }
        self.current()
    }

    fn move_to(&mut self, doc_id: DocId) -> usize {
        if self.current_unchecked() == doc_id {
            return 0;
        }
        self.advance();
        let mut skipped = 1;

        while let Some(x) = self.advance() {
            skipped += 1;
            if x >= doc_id {
                return skipped
            }
        }
        skipped
    }

    fn to_vec(&mut self) -> Vec<DocId> {
        let mut res = Vec::with_capacity(self.remains());
        while let Some(x) = self.advance() {
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
