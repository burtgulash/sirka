use types::*;

#[derive(Clone)]
struct CumEncoder<S> {
    seq: S,
    cum: DocId,
}

impl<S: Sequence> CumEncoder<S> {
    fn new(start_from: DocId, seq: S) -> Self {
        CumEncoder {
            seq: seq,
            cum: start_from,
        }
    }
}

macro_rules! tryopt {
    ($e:expr) => (match $e {
        Some(value) => value,
        None => return None,
    })
}

impl<S: Sequence> Sequence for CumEncoder<S> {
    fn next(&mut self) -> Option<DocId> {
        if let Some(x) = self.seq.next() {
            self.cum += x;   
            Some(self.cum)
        } else {
            None
        }
    }

    fn current_position(&self) -> Option<usize> {
        self.seq.current_position()
    }

    fn remains(&self) -> usize {
        self.seq.remains()
    }

    fn subsequence(&self, start: usize, len: usize) -> Self {
        Self::new(0, self.seq.subsequence(start, len))
    }
}

#[cfg(test)]
mod tests {
    use super::CumEncoder;
    use types::{Sequence,SequenceStorage};

    #[test]
    fn test_cum_sequence() {
        let docs = vec![0,3,5,7,8,9,10,11,15,16,18,22];
        let cum_seq1 = CumEncoder::new(0,(&docs[..]).to_sequence());

        let subseq_len = 7;
        let mut cum_seq = cum_seq1.subsequence(0, subseq_len);

        println!("REMAINS: {}", cum_seq.remains());
        assert_eq!(subseq_len, cum_seq.remains());

        let mut count = 0;
        while let Some(doc) = cum_seq.next() {
            count += 1;
            println!("Next cum doc: {}", doc);
        }
        assert_eq!(subseq_len, count);
    }
}
