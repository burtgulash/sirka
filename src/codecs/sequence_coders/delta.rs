use types::*;

#[derive(Clone)]
pub struct DeltaEncoder<S> {
    seq: S,
    last: DocId,
}

impl<S: Sequence> DeltaEncoder<S> {
    pub fn new(mut seq: S) -> Self {
        assert!(seq.remains() > 0);
        let last = seq.next().unwrap();
        DeltaEncoder {
            seq: seq,
            last: last,
        }
    }
}

macro_rules! tryopt {
    ($e:expr) => (match $e {
        Some(value) => value,
        None => return None,
    })
}

impl<S: Sequence> Sequence for DeltaEncoder<S> {
    fn next(&mut self) -> Option<DocId> {
        if let Some(next) = self.seq.next() {
            let last = self.last;
            self.last = next;
            Some(next - last)
        } else {
            None
        }
    }

    fn current_position(&self) -> Option<usize> {
        Some(tryopt!(self.seq.current_position()) - 1)
    }

    fn remains(&self) -> usize {
        self.seq.remains()
    }

    fn skip_n(&mut self, n: usize) -> Option<DocId> {
        if let Some(last) = self.seq.skip_n(n - 1) {
            self.last = last;
            self.next()
        } else {
            None
        }
    }

    fn subsequence(&self, start: usize, len: usize) -> Self {
        Self::new(self.seq.subsequence(start, len + 1))
    }
}

#[cfg(test)]
mod tests {
    use super::DeltaEncoder;
    use types::{Sequence,SequenceStorage};

    #[test]
    fn test_delta_sequence() {
        let docs = vec![3,4,7,9,10,14,15,18,25,27,33,50,55,100];
        let subseq_len = 7;
        let delta_seq1 = DeltaEncoder::new((&docs[..]).to_sequence());
        let mut delta_seq = delta_seq1.subsequence(0, subseq_len);
        println!("REMAINS: {}", delta_seq.remains());
        assert_eq!(subseq_len, delta_seq.remains());

        let mut count = 0;
        while let Some(doc) = delta_seq.next() {
            count += 1;
            println!("Next delta doc: {}", doc);
        }
        assert_eq!(subseq_len, count);
    }
}
