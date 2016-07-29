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
    next_position: usize,
}

impl<'a> SliceSequence<'a> {
    pub fn new(seq: &'a [DocId]) -> Self {
        SliceSequence {
            seq: seq,
            next_position: 0,
        }
    }

    fn current(&self) -> Option<DocId> {
        if 0 < self.next_position && self.next_position <= self.seq.len() {
            Some(self.seq[self.next_position - 1])
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
        self.seq.len() - self.next_position
    }

    fn skip_to(&mut self, doc_id: DocId) -> Option<DocId> {
        while self.next_position <= self.seq.len()
           && self.seq[self.next_position - 1] < doc_id
        {
            self.next_position += 1;
        }
        self.current()
    }

    fn skip_n(&mut self, n: usize) -> Option<DocId> {
        self.next_position += n;
        self.current()
    }

    fn current_position(&self) -> Option<usize> {
        if self.next_position > 0 {
            Some(self.next_position - 1)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use types::{Sequence,SequenceStorage};

    #[test]
    fn test_sequence() {
        let docs = vec![5,7,3,9,45,1,0,4,7];
        let mut seq = (&docs[..]).to_sequence();
        while let Some(doc) = seq.next() {
            println!("Next doc: {}", doc);
        }
        println!("CUUU");
    }
}
