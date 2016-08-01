use std::{mem,slice,io};
use types::*;
use util::typed_to_bytes;
use postings::{Sequence,SequenceStorage,SequenceEncoder};

// impl<'a> SequenceStorage<'a> for Vec<DocId> {
//     type Sequence = SliceSequence<'a>;
// 
//     fn to_sequence(&self) -> Self::Sequence {
//         SliceSequence::new(&self)
//     }
// }

impl<'a> SequenceStorage<'a> for &'a Vec<DocId> {
    type Sequence = SliceSequence<'a>;

    fn to_sequence(&self) -> Self::Sequence {
        SliceSequence::new(&self[..])
    }
}

impl<'a> SequenceStorage<'a> for &'a [DocId] {
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
        let xs = seq.to_vec();
        self.writer.write(typed_to_bytes(&xs))
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
            next_position: 1,
            seq: seq,
        }
    }
}

impl<'a> Sequence for SliceSequence<'a> {
    fn subsequence(&self, start: usize, len: usize) -> SliceSequence<'a> {
        let mut sub = SliceSequence::new(&self.seq[..start+len]);
        sub.move_n(start);
        sub
    }

    fn current_unchecked(&self) -> DocId {
        self.seq[self.next_position - 1]
    }

    fn current(&self) -> Option<DocId> {
        if self.next_position <= self.seq.len() {
            Some(self.current_unchecked())
        } else {
            None
        }
    }

    fn advance(&mut self) -> Option<DocId> {
        let cur = self.current();
        self.next_position += 1;
        cur
    }

    fn remains(&self) -> usize {
        self.seq.len() + 1 - self.next_position
    }

    fn move_n(&mut self, n: usize) -> Option<DocId> {
        self.next_position += n;
        self.current()
    }
}

#[cfg(test)]
mod tests {
    use types::*;
    use postings::{Sequence,SequenceStorage};

    #[test]
    fn test_sequence() {
        let docs = vec![5,7,3,9,45,1,0,4,7];
        let mut seq = (&docs[..]).to_sequence();
        while let Some(doc) = seq.next() {
            println!("Next doc: {}", doc);
        }
        println!("---");
    }

    #[test]
    fn test_slice_sequence_skip() {
        let docs = vec![5,7,9,11,15,17,50,90];
        let mut seq = (&docs[..]).to_sequence();
        assert_eq!(seq.current().unwrap(), 5);
        assert_eq!(seq.move_to(9), 2);
        assert_eq!(seq.move_to(12), 2);
        assert_eq!(seq.move_to(17), 1);
        assert_eq!(seq.move_to(30), 1);
        assert_eq!(seq.move_to(60), 1);
    }

    #[test]
    fn test_slice_subsequence_skip() {
        let docs = vec![5,7,9,11,15,17,50,90, 120, 2000, 2001];
        let mut seq = (&docs[..]).to_sequence();
        let mut subseq = seq.subsequence(3, 5);

        assert_eq!(seq.current().unwrap(), 5);

        assert_eq!(subseq.current().unwrap(), 11);
        assert_eq!(subseq.move_to(11), 0);
        assert_eq!(subseq.move_to(17), 2);
        assert_eq!(subseq.move_to(30), 1);
        assert_eq!(subseq.move_to(60), 1);
    }
}
