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
    position: usize,
}

impl<'a> SliceSequence<'a> {
    pub fn new(seq: &'a [DocId]) -> Self {
        SliceSequence {
            position: 0,
            seq: seq,
        }
    }

    fn get_at(&self) -> Option<DocId> {
        if self.position < self.seq.len() {
            Some(self.seq[self.position - 1])
        } else {
            None
        }
    }
}

impl<'a> Sequence for SliceSequence<'a> {
    fn subsequence(&self, start: usize, len: usize) -> SliceSequence<'a> {
        let mut sub = SliceSequence::new(&self.seq[..start+len]);
        if start > 0 {
            sub.skip_n(start);
        }
        sub
    }

    fn next_position(&self) -> usize {
        self.position
    }

    fn current(&self) -> DocId {
        assert!(self.position < self.seq.len());
        self.seq[self.position - 1]
    }

    fn next(&mut self) -> Option<DocId> {
        self.position += 1;
        self.get_at()
    }

    fn remains(&self) -> usize {
        self.seq.len() - self.position
    }

    fn skip_n(&mut self, n: usize) -> Option<DocId> {
        self.position += n;
        self.get_at()
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
        assert_eq!(seq.next().unwrap(), 5);
        assert_eq!(seq.skip_to(9), (2, Some(9)));
        assert_eq!(seq.skip_to(12), (2, Some(15)));
        assert_eq!(seq.skip_to(17), (1, Some(17)));
        assert_eq!(seq.skip_to(30), (1, Some(50)));
        assert_eq!(seq.skip_to(60), (1, Some(90)));
        assert_eq!(seq.skip_to(100), (0, None));
    }

    #[test]
    fn test_slice_subsequence_skip() {
        let docs = vec![5,7,9,11,15,17,50,90, 120, 2000, 2001];
        let mut seq = (&docs[..]).to_sequence();
        let mut subseq = seq.subsequence(2, 6);

        assert_eq!(subseq.next().unwrap(), 9);
        assert_eq!(subseq.skip_to(11), (1, Some(11)));
        assert_eq!(subseq.skip_to(17), (2, Some(17)));
        assert_eq!(subseq.skip_to(30), (1, Some(50)));
        assert_eq!(subseq.skip_to(60), (1, Some(90)));
        assert_eq!(subseq.skip_to(100000), (0, None));
    }

    #[test]
    fn test_slice_sequence_skip_n() {
        let docs = vec![5,7,9,11,15,17,50,90, 120, 2000, 2001];
        let mut seq = (&docs[..]).to_sequence();

        assert_eq!(seq.next().unwrap(), 5);
        assert_eq!(seq.next().unwrap(), 7);
        assert_eq!(seq.skip_n(0).unwrap(), 7);
        assert_eq!(seq.skip_n(0).unwrap(), 7);
        assert_eq!(seq.skip_n(0).unwrap(), 7);

        assert_eq!(seq.skip_n(1).unwrap(), 9);
        assert_eq!(seq.skip_n(0).unwrap(), 9);
        assert_eq!(seq.skip_n(0).unwrap(), 9);

        assert_eq!(seq.skip_n(1).unwrap(), 11);
        assert_eq!(seq.skip_n(1).unwrap(), 15);
        assert_eq!(seq.skip_n(1).unwrap(), 17);

        assert_eq!(seq.skip_n(2).unwrap(), 90);
        assert_eq!(seq.skip_n(1).unwrap(), 120);
        assert_eq!(seq.skip_n(1).unwrap(), 2000);

        assert_eq!(seq.skip_n(2), None);
    }
}
