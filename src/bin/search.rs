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
    if args.len() != 3 {
        println!("{}", USAGE);
        std::process::exit(1);
    }

    let indexdir = Path::new(&args[1]);
    let search_term = &args[2];

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

    let mut tfs_reader = create_reader(indexdir, "tfs");
    let mut tfsbuf = Vec::new();
    tfs_reader.read_to_end(&mut tfsbuf).unwrap();

    println!("META: {:?}", meta);
    match dict.find_term(search_term, true) {
        Some(_) => println!("found!"),
        None => println!("nothing!"),
    };
}

macro_rules! tryopt {
    ($e:expr) => (match $e {
        Some(value) => value,
        None => return None,
    })
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

fn daat<'a, S: Sequence<'a>>(docs: S, tfs: S, query_nodes: &[&TrieNodeHeader]) -> bool {
    let mut doc_sliders = query_nodes.iter().map(|n| docs.slider(n.postings_ptr as usize, n.num_postings as usize)).collect::<Vec<_>>();
    let mut tf_sliders = query_nodes.iter().map(|n| tfs.slider(n.postings_ptr as usize, n.num_postings as usize)).collect::<Vec<_>>();

    // TODO sort'em sliders
    let mut result = Vec::new();

    let mut current_doc_id = 0;
    'merge: loop {
        let mut i = 0;
        while i < doc_sliders.len() {
            let mut slider = &mut doc_sliders[i];
            if let Some(doc_id) = slider.skip_to(current_doc_id) {
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
        for slider in &mut doc_sliders {
            if let Some(next_doc_id) = slider.next() {
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

    false
}
