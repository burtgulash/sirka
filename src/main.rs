extern crate indox;

use std::cmp::Ordering;
use std::io::{BufReader, BufWriter};
use std::io::BufRead;
use std::fs::File;
use std::collections::HashMap;

use indox::*;

static USAGE: &'static str = "usage: index <input_file> <output_prefix>";

fn create_writer(file_prefix: &str, filename: &str) -> BufWriter<File> {
    let path = format!("{}_{}", file_prefix, filename);
    let file = File::create(path).unwrap();
    BufWriter::new(file)
}

fn main() {
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 3 {
        println!("{}", USAGE);
        std::process::exit(1);
    }
    let path = std::path::Path::new(&args[1]);
    let output_files_prefix = args[2].to_owned();
    let f = File::open(&path).unwrap();
    let file = BufReader::new(&f);

    let mut term_serial: TermId = 0;
    let mut doc_serial: DocId = 0;

    let mut h = HashMap::<String, TermId>::new();
    let mut docbufs = TermBuf::new();
    let mut tfbufs = TermBuf::new();
    let mut posbufs = TermBuf::new();

    for line in file.lines() {
        let l = line.unwrap();
        doc_serial += 1;

        let mut forward_index = Vec::<(TermId, u32)>::new();
        for (position, s) in l.split("|").enumerate() {
            if s.len() > 0 {
                let term_id = *h.entry(s.into()).or_insert_with(|| {
                    term_serial += 1;
                    term_serial
                });
                forward_index.push((term_id, position as u32));
            }
        }

        forward_index.sort_by(|a, b| {
            let c = a.0.cmp(&b.0);
            if c == Ordering::Equal {
                return a.1.cmp(&b.1);
            }
            c
        });

        let mut last_term_id = 0;
        let mut tf = 0;

        let mut control_tf = 0;
        let len = forward_index.len();

        macro_rules! ADD_DOC {
            () => {
                docbufs.add_doc(last_term_id, doc_serial);
                tfbufs.add_doc(last_term_id, tf);
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

    // for buf in &docbufs.buffers {
    //     println!("{:?}", buf.as_ref().unwrap());
    // }
    //

    let terms = {
        let mut ts: Vec<Term> = h.drain().map(|(term, term_id)| Term {term: term, term_id: term_id}).collect();
        ts.sort_by(|a, b| a.term.cmp(&b.term));
        ts
    };
    drop(h);

    let mut postings = (&mut docbufs, &mut tfbufs, &mut posbufs);

    println!("Creating Prefix Trie");
    let (written_terms, dict_size, root_ptr, terms_size) = create_trie(term_serial, &terms, &mut postings,
        &mut create_writer(&output_files_prefix, "dict"),
        &mut create_writer(&output_files_prefix, "docs"),
        &mut create_writer(&output_files_prefix, "tfs"),
        &mut create_writer(&output_files_prefix, "positions"),
    );

    //println!("Creating BK Tree");
    //let mut bk = BKTree::new();
    //for term in terms.iter() {
    //    bk.insert_term(term);
    //}

    //println!("Inserting prefixes into BK Tree");
    //for term in &tr.new_terms {
    //    bk.insert_term(term);
    //    // println!("Prefix: {}", term.term);
    //}
    //bk.print();



    // println!("{}", get_common_prefix_len("autobus", "autoba"));

    //for t in terms {
    //    println!("{}", t.term);
    //}

    //println!("{:?}", terms);
}
