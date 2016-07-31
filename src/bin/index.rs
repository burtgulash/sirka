extern crate sirka;

use std::cmp::Ordering;
use std::io::{BufReader, BufWriter, Write};
use std::io::BufRead;
use std::fs::{self,File};
use std::collections::HashMap;

use sirka::*;

static USAGE: &'static str = "usage: index <inputfile> <outputdir>";

fn create_writer(directory: &str, filename: &str) -> BufWriter<File> {
    let path = format!("{}/{}", directory, filename);
    let file = File::create(path).unwrap();
    BufWriter::new(file)
}

fn process_docs(reader: BufReader<File>, term_serial: &mut TermId, doc_serial: &mut DocId, separator: &str) -> (Vec<Term>, (TermBuf, TermBuf, TermBuf))
{
    let mut h = HashMap::<String, TermId>::new();
    let mut docbufs = TermBuf::new();
    let mut tfbufs = TermBuf::new();
    let mut posbufs = TermBuf::new();

    for line in reader.lines() {
        let line = line.unwrap();

        let mut forward_index = Vec::<(TermId, DocId)>::new();
        for (position, s) in line.split(separator).enumerate() {
            if s.len() > 0 {
                let term_id = *h.entry(s.into()).or_insert_with(|| {
                    *term_serial += 1;
                    *term_serial
                });
                forward_index.push((term_id, position as DocId));
            }
        }

        // Skip empty lines
        if forward_index.len() == 0 {
            continue;
        }

        *doc_serial += 1;
        forward_index.sort_by(|a, b| {
            let c = a.0.cmp(&b.0);
            if c == Ordering::Equal {
                return a.1.cmp(&b.1);
            }
            c
        });

        // Checksum for tfs. All positions must sum to this
        let mut control_tf = 0;
        // forward_index will be moved to loop
        let len = forward_index.len();

        let mut last_term_id = 0;
        let mut tf = 0;

        macro_rules! ADD_DOC {
            () => {
                docbufs.add_doc(last_term_id, *doc_serial);
                tfbufs.add_doc(last_term_id, tf);
                assert!(tf > 0);
                control_tf += tf;
            }
        }

        for (term_id, position) in forward_index {
            posbufs.add_doc(term_id, position);
            if term_id == last_term_id {
                tf += 1;
            } else {
                if last_term_id != 0 {
                    ADD_DOC!();
                }
                last_term_id = term_id;
                tf = 1;
            }
        }
        ADD_DOC!();
        assert_eq!(control_tf as usize, len);
    }

    let mut ts: Vec<Term> = h.drain().map(|(term, term_id)| Term {term: term, term_id: term_id}).collect();
    ts.sort_by(|a, b| a.term.cmp(&b.term));
    (ts, (docbufs, tfbufs, posbufs))
}

fn main() {
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 3 {
        println!("{}", USAGE);
        std::process::exit(1);
    }
    let path = std::path::Path::new(&args[1]);
    let dirname = args[2].to_owned();
    let documents_reader = BufReader::new(File::open(&path).unwrap());

    fs::create_dir_all(&dirname).unwrap();

    let mut term_serial: TermId = 0;
    let mut doc_serial: DocId = 0;

    let (terms, (mut docbufs, mut tfbufs, mut posbufs)) = process_docs(documents_reader, &mut term_serial, &mut doc_serial, "|");
    let mut postings = (&mut docbufs, &mut tfbufs, &mut posbufs);

    println!("Creating Prefix Trie");
    let mut enc = PostingsEncoders {
        docs: PlainEncoder::new(create_writer(&dirname, "docs")),
        tfs: PlainEncoder::new(create_writer(&dirname, "tfs")),
        positions: PlainEncoder::new(create_writer(&dirname, "positions")),
    };
    let (written_terms, dict_size, root_ptr, terms_size) = create_trie(term_serial, &terms, &mut postings,
                                                                       &mut create_writer(&dirname, "dict"), &mut enc);

    let meta = IndexMeta {
        dict_size: dict_size as u64,
        root_ptr: root_ptr as u64,
        term_buffer_size: terms_size as u64,
        docs_size: 0,
        tfs_size: 0,
        positions_size: 0,
    };

    create_writer(&dirname, "meta").write(&meta.to_bytes()).unwrap();
}
