use types::{DocId,Sequence};

#[derive(Clone)]
pub struct SliceSequence<'a> {
    seq: &'a [DocId],
    position: usize,
}

impl<'a> SliceSequence<'a> {
    pub fn new(seq: &'a [DocId]) -> Self {
        SliceSequence {
            seq: seq,
            position: 0,
        }
    }
}

impl<'a> Sequence for SliceSequence<'a> {
    fn subsequence(&self, start: usize, len: usize) -> Self {
        let mut sub = SliceSequence::new(&self.seq[..start+len]);
        sub.move_n(start);
        sub
    }

    fn remains(&self) -> usize {
        self.seq.len() - self.position
    }

    fn current(&self) -> Option<DocId> {
        if self.position < self.seq.len() {
            Some(self.seq[self.position])
        } else {
            None
        }
    }

    fn move_to(&mut self, doc_id: DocId) {
        while self.position < self.seq.len()
           && self.seq[self.position] < doc_id
        {
            self.position += 1;
        }
    }

    fn move_n(&mut self, n: usize) {
        self.position += n;
    }

    fn current_position(&self) -> usize {
        self.position
    }
}
