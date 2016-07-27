use std::io;
use types::*;

pub trait Sequence<'a> {
    type Slider: SequenceSlider + 'a;
    fn slider(&self) -> Self::Slider;
}

pub trait SequenceSlider: Clone {
    fn next(&mut self) -> Option<DocId>;
    fn skip_to(&mut self, doc_id: DocId) -> Option<DocId>;
    fn skip_n(&mut self, n: usize) -> Option<DocId>;
}

pub trait SequenceWriter {
    fn write_doc(&mut self, doc_id: DocId) -> io::Result<()>;
    fn finish_docs(&mut self) -> io::Result<()>;
    //fn write_index(&mut self) -> io::Result<()>;
}
