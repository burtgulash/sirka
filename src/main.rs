mod indox;

use std::cmp::Ordering;
use std::io::BufReader;
use std::io::BufRead;
use std::fs::File;
use std::collections::HashMap;

use indox::*;

static USAGE: &'static str = "usage: index <input_file> <output_prefix>";

fn main() {
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 3 {
        println!("{}", USAGE);
        std::process::exit(1);
    }
    let path = std::path::Path::new(&args[1]);
    let f = File::open(&path).unwrap();
    let file = BufReader::new(&f);

    let mut term_serial: TermId = 0;
    let mut doc_serial: DocId = 0;

    let mut h = HashMap::<String, TermId>::new();
    let mut docbuf = TermBuf::new();
    let mut tfbuf = TermBuf::new();
    let mut posbuf = TermBuf::new();

    for line in file.lines() {
        let l = line.unwrap();
        doc_serial += 1;

        let mut forward_index = Vec::<(TermId, u32)>::new();
        for (position, s) in l.split("|").enumerate() {
            let term_id = *h.entry(s.into()).or_insert_with(|| {
                term_serial += 1;
                term_serial
            });
            forward_index.push((term_id, position as u32));
        }

        forward_index.sort_by(|a, b| {
            if a.0 > b.0 {
                return Ordering::Greater;
            } else if a.0 < b.0 {
                return Ordering::Less;
            } else if a.1 > b.1 {
                return Ordering::Greater;
            } else if a.1 < b.1 {
                return Ordering::Less;
            }
            return Ordering::Equal;
        });

        let mut last_term_id = 0;
        let mut tf = 0;
        for (term_id, position) in forward_index {
            posbuf.add_doc(term_id, position);
            if term_id == last_term_id {
                tf += 1;
            } else {
                if last_term_id != 0 {
                    docbuf.add_doc(term_id, doc_serial);
                    tfbuf.add_doc(term_id, tf);
                }
                last_term_id = term_id;
                tf = 1;
            }
        }
        docbuf.add_doc(last_term_id, doc_serial);
        tfbuf.add_doc(last_term_id, tf);
    }

    let mut terms: Vec<Term> = h.iter().map(|(term, &term_id)| Term {term: term, term_id: term_id}).collect();
    terms.sort_by(|a, b| a.term.cmp(b.term));

    NuTrie::create(term_serial, terms.iter());

    let mut bk = BKTree::new();
    for term in terms.iter() {
        bk.insert_term(term);
    }
    //bk.print();


    // println!("{}", get_common_prefix_len("autobus", "autoba"));

    //for t in terms {
    //    println!("{}", t.term);
    //}

    //println!("{:?}", terms);
}
