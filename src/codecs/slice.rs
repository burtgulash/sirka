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

    fn write_sequence<S: Sequence>(&mut self, mut seq: S) -> io::Result<usize> {
        let mut total_size = 0;
        while let Some(doc_id) = seq.next() {
            total_size += try!(self.write(doc_id));
        }
        Ok(total_size)
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

    fn return_current(&self) -> Option<DocId> {
        if self.position < self.seq.len() {
            Some(self.seq[self.position])
        } else {
            None
        }
    }
}

impl<'a> Sequence for SliceSequence<'a> {
    fn subsequence(&self, start: usize, len: usize) -> Self {
        let mut sub = SliceSequence::new(&self.seq[..start+len]);
        sub.skip_n(start);
        sub
    }

    fn remains(&self) -> usize {
        self.seq.len() - self.position
    }

    fn next(&mut self) -> Option<DocId> {
        self.skip_n(1)
    }

    fn skip_to(&mut self, doc_id: DocId) -> Option<DocId> {
        while self.position < self.seq.len()
           && self.seq[self.position] < doc_id
        {
            self.position += 1;
        }
        self.return_current()
    }

    fn skip_n(&mut self, n: usize) -> Option<DocId> {
        self.position += n;
        self.return_current()
    }

    fn current_position(&self) -> usize {
        self.position
    }
}
