use indexer::*;

pub trait DocSequence {
    fn next(&mut self) -> Option<DocId>;
    fn skip_to(&mut self, doc_id: DocId) -> Option<DocId>;
    fn skip_n(&mut self, n: usize) -> Option<DocId>;
}

struct SliceSequence<'a> {
    seq: &'a [DocId],
    position: usize,
}

impl<'a> SliceSequence<'a> {
    fn new(seq: &'a [DocId]) -> Self {
        SliceSequence {
            seq: seq,
            position: 0,
        }
    }

    fn return_at_current(&self) -> Option<DocId> {
        if self.position < self.seq.len() {
            Some(self.seq[self.position - 1])
        } else {
            None
        }
    }
}

impl<'a> DocSequence for SliceSequence<'a> {
    fn next(&mut self) -> Option<DocId> {
        self.skip_n(1)
    }

    fn skip_to(&mut self, doc_id: DocId) -> Option<DocId> {
        while self.position < self.seq.len() 
           && self.seq[self.position] < doc_id
        { self.position += 1; }
        self.return_at_current()
    }

    fn skip_n(&mut self, n: usize) -> Option<DocId> {
        self.position += n;
        self.return_at_current()
    }
}
