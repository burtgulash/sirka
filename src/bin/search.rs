extern crate sirka;

use std::path::Path;
use std::io::{BufReader,Read};
use std::fs::File;
use sirka::*;

static USAGE: &'static str = "usage: search <indexdir> <term>";

fn create_reader(dirname: &Path, filename: &str) -> BufReader<File> {
    let path = dirname.join(Path::new(filename));
    let reader = BufReader::new(File::open(&path).unwrap());
    reader
}

fn main() {
    let args: Vec<_> = std::env::args().collect();
    if args.len() < 3 {
        println!("{}", USAGE);
        std::process::exit(1);
    }

    let indexdir = Path::new(&args[1]);
    let query_to_seach = &args[2..];


    let mut meta_reader = create_reader(indexdir, "meta");
    let mut buf = Vec::new();
    meta_reader.read_to_end(&mut buf).unwrap();

    let meta = IndexMeta::from_bytes(&buf[..]);

    let mut trie_reader = create_reader(indexdir, "dict");
    let dictbuf = StaticTrie::read(&mut trie_reader);
    let dict = StaticTrie::new(&dictbuf[..], meta.dict_size as usize, meta.root_ptr as usize, meta.term_buffer_size as usize);

    let mut docs_reader = create_reader(indexdir, "docs");
    let mut docsbuf = Vec::new();
    docs_reader.read_to_end(&mut docsbuf).unwrap();
    let docs = bytes_to_typed(&docsbuf).to_sequence();

    let mut tfs_reader = create_reader(indexdir, "tfs");
    let mut tfsbuf = Vec::new();
    tfs_reader.read_to_end(&mut tfsbuf).unwrap();
    let tfs = bytes_to_typed(&tfsbuf).to_sequence();

    if let Some(result) = query(&dict, &docs, &tfs, query_to_seach) {
        println!("Found in {} docs!", result.len());
    } else {
        println!("Not found!");
    }
}

macro_rules! tryopt {
    ($e:expr) => (match $e {
        Some(value) => value,
        None => return None,
    })
}

struct PostingSequences<DS: Sequence, TS: Sequence> {
    index: usize,
    term_id: TermId,
    docs: DS,
    tfs: TS,
//    pos: S,
}

fn query<STRING, DS, TS>(dict: &StaticTrie, docs: &DS, tfs: &TS, q: &[STRING]) -> Option<Vec<DocId>> 
    where STRING: AsRef<str>,
          DS: Sequence,
          TS: Sequence
{
    let q = q.iter().map(|s| s.as_ref()).collect::<Vec<&str>>();
    println!("Searching query: {:?}", &q);
    let term_headers = tryopt!(find_terms(&dict, &q));

    let mut term_sequences = term_headers.iter().enumerate().map(|(i, th)| {
        println!("Term found. term='{}', numdocs={}", th.term_id, th.num_postings);
        PostingSequences {
            index: i,
            term_id: th.term_id,
            docs: docs.subsequence(th.postings_ptr as usize, th.num_postings as usize),
            tfs: tfs.subsequence(th.postings_ptr as usize, th.num_postings as usize),
        }
    }).collect::<Vec<_>>();

    // sort sequences ascending by their size to make daat skipping much faster
    term_sequences.sort_by(|a, b| {
        a.docs.remains().cmp(&b.docs.remains())
    });

    Some(search_daat(term_sequences))
}

fn find_terms<'a>(dict: &'a StaticTrie, query: &[&str]) -> Option<Vec<&'a TrieNodeHeader>> {
    let mut headers = Vec::new();
    for term in query.iter() {
        match dict.find_term(term, true) {
            Some(header) => headers.push(header),
            None => return None,
        }
    }
    Some(headers)
}

// search daat = search document at a time
fn search_daat<DS, TS>(mut term_sequences: Vec<PostingSequences<DS, TS>>) -> Vec<DocId> 
    where DS: Sequence,
          TS: Sequence
{
    let mut result = Vec::new();

    let mut current_doc_id = 0;
    'merge: loop {
        let mut i = 0;
        while i < term_sequences.len() {
            let mut current_seq = &mut term_sequences[i];
            current_seq.docs.move_to(current_doc_id);
            if let Some(doc_id) = current_seq.docs.current() {
                if doc_id > current_doc_id {
                    // Aligning failed. Start from first term
                    i = 0;
                    current_doc_id = doc_id;
                    continue;
                }
            } else {
                break 'merge;
            }

            // Try to align next query term
            i += 1;
        }

        // Sliders are now aligned on 'doc_id'.
        // This means a match, so output one result
        result.push(current_doc_id);

        // Advance all sliders to next doc and record
        // the maximum doc_id for each slider
        let mut max_doc_id = current_doc_id;
        for sequence in term_sequences.iter_mut() {
            sequence.docs.move_n(1);
            if let Some(next_doc_id) = sequence.docs.current() {
                if next_doc_id > max_doc_id {
                    max_doc_id = next_doc_id;
                }
            } else {
                break 'merge;
            }
        }

        // Start next iteration alignment with maximum doc id
        current_doc_id = max_doc_id;
    }

    result
}
