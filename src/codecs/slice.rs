use std::{mem,slice,io};
use types::{DocId,Sequence,SequenceStorage,SequenceEncoder};

impl<'a> SequenceStorage for &'a [DocId] {
    type Sequence = SliceSequence<'a>;

    fn to_sequence(&self) -> Self::Sequence {
        SliceSequence::new(self)
    }
}

pub struct PlainEncoder<W> {
    writer: W,
}

impl<W: io::Write> PlainEncoder<W> {
    pub fn new(writer: W) -> PlainEncoder<W> {
        PlainEncoder {
            writer: writer
        }
    }
}

impl<W: io::Write> SequenceEncoder for PlainEncoder<W> {
    fn write(&mut self, doc_id: DocId) -> io::Result<usize> {
        let docbuf = unsafe{slice::from_raw_parts(&doc_id as *const _ as *const u8, mem::size_of::<DocId>())};
        self.writer.write(docbuf)
    }
}


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
