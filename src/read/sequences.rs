use std::{io,mem,slice};
use types::{DocId,Sequence,SequenceSpawner};

impl<'a> SequenceSpawner for &'a [DocId] {
    type Sequence = SliceSequence<'a>;
    fn spawn(&self, start: usize, len: usize) -> Self::Sequence {
        let mut s = SliceSequence::new(&self[..start+len]);
        s.skip_n(start);
        s
    }
}

#[derive(Clone)]
pub struct SliceSequence<'a> {
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
}

impl<'a> Sequence for SliceSequence<'a> {
    fn put(&mut self) {
        // No need to put anything to read only sequence
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

    fn skip_to(&mut self, doc_id: DocId) {
        while self.position < self.seq.len()
           && self.seq[self.position] < doc_id
        {
            self.position += 1;
        }
    }

    fn skip_n(&mut self, n: usize) {
        self.position += n;
    }

    fn current_position(&self) -> usize {
        self.position
    }

    fn write_current(&self, writer: &mut io::Write) -> io::Result<usize> {
        if let Some(doc_id) = self.current() {
            let docbuf: &[u8] = unsafe {
                slice::from_raw_parts(&doc_id as *const _ as *const u8, mem::size_of::<DocId>())
            };
            writer.write(docbuf)
        } else {
            Ok(0)
        }
    }
}
