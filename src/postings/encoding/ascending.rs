use types::*;
use postings::Sequence;

#[derive(Clone)]
struct Ascending<S> {
    seq: S,
    position: usize,
}

impl<S: Sequence> Ascending<S> {
    fn new(start_position: usize, seq: S) -> Self {
        Ascending {
            seq: seq,
            position: start_position,
        }
    }

    fn _next(&mut self, encoder: bool) -> Option<DocId> {
        if let Some(x) = self.seq.next() {
            let pos = self.position;
            self.position += 1;
            if encoder {
                Some(x - pos as DocId)
            } else {
                Some(x + pos as DocId)
            }
        } else {
            None
        }
    }

    fn next_position(&self) -> usize {
        self.seq.next_position()
    }

    fn remains(&self) -> usize {
        self.seq.remains()
    }

    fn subsequence(&self, start: usize, len: usize) -> Self {
        Ascending::new(self.position + start, self.seq.subsequence(start, len))
    }
}

#[derive(Clone)]
pub struct AscendingEncoder<S>(Ascending<S>);
#[derive(Clone)]
pub struct AscendingDecoder<S>(Ascending<S>);

impl<S: Sequence> AscendingEncoder<S> {
    pub fn new(start_position: usize, seq: S) -> Self {
        AscendingEncoder(Ascending::new(start_position, seq))
    }
}

impl<S: Sequence> AscendingDecoder<S> {
    pub fn new(start_position: usize, seq: S) -> Self {
        AscendingDecoder(Ascending::new(start_position, seq))
    }
}

impl<S: Sequence> Sequence for AscendingEncoder<S> {
    fn next(&mut self) -> Option<DocId> {
        (self.0)._next(true)
    }

    fn next_position(&self) -> usize {
        self.0.next_position()
    }

    fn remains(&self) -> usize {
        self.0.remains()
    }

    fn subsequence(&self, start: usize, len: usize) -> Self {
        AscendingEncoder(self.0.subsequence(start, len))
    }
}

impl<S: Sequence> Sequence for AscendingDecoder<S> {
    fn next(&mut self) -> Option<DocId> {
        (self.0)._next(false)
    }

    fn next_position(&self) -> usize {
        self.0.next_position()
    }

    fn remains(&self) -> usize {
        self.0.remains()
    }

    fn subsequence(&self, start: usize, len: usize) -> Self {
        AscendingDecoder(self.0.subsequence(start, len))
    }
}
