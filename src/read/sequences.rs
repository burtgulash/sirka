use std::io::{Result,BufWriter};
use types::{DocId,Sequence,SequenceSlider};

impl<'a> Sequence for &'a [DocId] {
    type Slider = SliceSequenceSlider<'a>;
    fn slider(&self, start: usize, len: usize) -> Self::Slider {
        let mut s = SliceSequenceSlider::new(&self[..start+len]);
        s.skip_n(start);
        s
    }
}

#[derive(Clone)]
pub struct SliceSequenceSlider<'a> {
    seq: &'a [DocId],
    position: usize,
}

impl<'a> SliceSequenceSlider<'a> {
    fn new(seq: &'a [DocId]) -> Self {
        SliceSequenceSlider {
            seq: seq,
            position: 0,
        }
    }

    fn current(&self) -> Option<DocId> {
        if self.position < self.seq.len() {
            Some(self.seq[self.position])
        } else {
            None
        }
    }
}

impl<'a> SequenceSlider for SliceSequenceSlider<'a> {
    fn next(&mut self) -> Option<DocId> {
        self.skip_n(1)
    }

    fn skip_to(&mut self, doc_id: DocId) -> Option<DocId> {
        while self.position < self.seq.len()
           && self.seq[self.position] < doc_id
        {
            self.position += 1;
        }
        self.current()
    }

    fn skip_n(&mut self, n: usize) -> Option<DocId> {
        self.position += n;
        self.current()
    }

    fn index(&self) -> usize {
        self.position
    }
}

// struct SliceSequenceWriter {
//     writer: BufWriter<File>
// }
// 
// impl SequenceWriter for SliceSequenceWriter {
//     fn write_doc(&mut self, doc_id: DocId) -> io::Result<()> {
//         let docbuf = slice::from_raw_parts(&doc_id as *const _ as *const u8, mem::size_of_val(doc_id));
//         self.writer.write(docbuf).map_err(|e| ())
//     }
// 
//     fn finish_docs(&mut self) -> io::Result {
//         self.writer.flush().map_err(|e| ())
//     }
// }

#[cfg(test)]
mod tests {
    use super::*;
    use types::*;

    fn use_sequence<S: Sequence>(seq: S) {
        let mut slider = seq.slider(3, 55);
        while let Some(doc_id) = slider.next() {
            println!("Iterated sequence to: {}", doc_id);
        }
    }

    #[test]
    fn test_it() {
        let mut data = Vec::<DocId>::new();
        for i in 0..123 {
            data.push(i);
        }
        use_sequence(&data[..]);
    }
}
