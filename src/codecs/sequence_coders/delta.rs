use types::*;

#[derive(Clone)]
pub struct DeltaEncoder<S> {
    seq: S,
    last: DocId,
    to_return: Option<DocId>,
}

impl<S: Sequence> DeltaEncoder<S> {
    pub fn new(mut seq: S) -> Self {
        let to_return = seq.next();
        let last = if let Some(x) = to_return { x } else { 0 };
        DeltaEncoder {
            seq: seq,
            last: last,
            to_return: to_return,
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
            let to_return = self.to_return;
            assert!(next >= self.last);
            self.to_return = Some(next - self.last);
            self.last = next;
            to_return
        } else {
            let to_return = self.to_return;
            self.to_return = None;
            to_return
        }
    }

    fn remains(&self) -> usize {
        self.seq.remains() + 1
    }

    fn skip_n(&mut self, n: usize) -> Option<DocId> {
        if n == 0 {
            return self.to_return;
        }

        let last = tryopt!(self.seq.skip_n(n - 1));
        self.to_return = None;
        self.last = last;

        let next = tryopt!(self.seq.next());
        self.to_return = self.seq.next();
        self.last = next;
        Some(next - last)
    }

    fn subsequence(&self, start: usize, len: usize) -> Self {
        let mut sub = Self::new(self.seq.subsequence(0, len));
        sub.skip_n(start);
        sub
    }
}

#[cfg(test)]
mod tests {
    use super::DeltaEncoder;
    use types::{Sequence,SequenceStorage};

    #[test]
    fn test_delta_sequence() {
        let docs = vec![3,4,7,9,10,14,15,18];
        let mut delta_seq = DeltaEncoder::new((&docs).to_sequence());
        assert_eq!(docs.len(), delta_seq.remains());

        let mut count = 0;
        let mut check = Vec::new();
        while let Some(doc) = delta_seq.next() {
            count += 1;
            check.push(doc);
            println!("Next delta doc: {}", doc);
        }
        assert_eq!(docs.len(), count);
        assert_eq!(check, vec![3,1,3,2,1,4,1,3]);
    }
}
