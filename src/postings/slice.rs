use std::{mem,slice,io};
use types::*;
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

    fn get_at(&self, at: usize) -> Option<DocId> {
        if at < self.seq.len() {
            Some(self.seq[at])
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

    fn next(&mut self) -> Option<DocId> {
        self.position += 1;
        self.get_at(self.position - 1)
    }

    fn remains(&self) -> usize {
        self.seq.len() - self.position
    }

    fn skip_n(&mut self, n: usize) -> Option<DocId> {
        if n == 0 {
            return self.get_at(self.position - 1);
        }
        self.position += n - 1;
        self.next()
    }

    fn next_position(&self) -> usize {
        self.position + 1
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

    #[test]
    fn test_slice_sequence_skip() {
        let docs = vec![5,7,9,11,15,17,50,90];
        let mut seq = (&docs[..]).to_sequence();
        assert_eq!(seq.next().unwrap(), 5);
        assert_eq!(seq.skip_to(9).unwrap(), 9);
        assert_eq!(seq.skip_to(12).unwrap(), 15);
        assert_eq!(seq.skip_to(17).unwrap(), 17);
        assert_eq!(seq.skip_to(30).unwrap(), 50);
        assert_eq!(seq.skip_to(60).unwrap(), 90);
    }

    #[test]
    fn test_slice_subsequence_skip() {
        let docs = vec![5,7,9,11,15,17,50,90, 120, 2000, 2001];
        let mut seq = (&docs[..]).to_sequence();
        let mut subseq = seq.subsequence(3, 5);
        assert_eq!(seq.skip_to(11).unwrap(), 11);
        assert_eq!(seq.skip_to(17).unwrap(), 17);
        assert_eq!(seq.skip_to(30).unwrap(), 50);
        assert_eq!(seq.skip_to(60).unwrap(), 90);
    }
}
