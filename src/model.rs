use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::BufWriter;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::SystemTime;

use crate::lexer::{self, Lexer};

pub type DocFreq = HashMap<String, usize>;
pub type TermFreq = HashMap<String, usize>;
pub type Docs = HashMap<PathBuf, Doc>;

enum DocumentState {
    UPDATE,
    STABLE,
    ADD,
}
#[derive(Debug, Clone)]
pub struct CountCheck {
    pub add: usize,
    pub stable: usize,
    pub update:usize
}

impl CountCheck {
    fn new() -> Self {
        Self {
            add: 0,
            stable: 0,
            update: 0
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Doc {
    pub tf: TermFreq,
    count: usize,
    pub last_modified: SystemTime,
}

impl Doc {
    pub fn new(tf: TermFreq, count: usize, last_modified: SystemTime) -> Self {
        Self {
            tf,
            count,
            last_modified,
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Model {
    // Term frekuensi untuk satu dokumen disatukan disini
    pub docs: Docs,
    // Total kemunculan semua kata di corpus
    pub df: DocFreq,
}

impl Model {
    pub fn new() -> Self {
        Self {
            docs: HashMap::new(),
            df: HashMap::new(),
        }
    }

    fn check_document(&self, path: &PathBuf, current_last_modified: SystemTime) -> DocumentState {
        if let Some(doc) = self.docs.get(path) {
            if doc.last_modified < current_last_modified {
                return DocumentState::UPDATE;
            } else {
                return DocumentState::STABLE;
            }
        }
        DocumentState::ADD
    }

    fn update_document(&mut self, path: PathBuf, current_last_modified: SystemTime) {
        let last_doc = self.docs.remove(&path).unwrap();

        for t in last_doc.tf.keys() {
            if let Some(freq) = self.df.get_mut(t) {
                *freq -= 1
            }
        }
        if let Some(new_doc) = calculate_document_by_extension(&path, current_last_modified) {
            for t in new_doc.tf.keys() {
                if let Some(freq) = self.df.get_mut(t) {
                    *freq += 1
                } else {
                    self.df.insert(t.to_string(), 1);
                }
            }
            println!("Re-Index: {:?} -> SELESAI", path);

            self.docs.insert(path, new_doc);
        } else {
        }
    }
    pub fn save_model_to_json_file(&self, path: &str) -> Result<(), ()> {
        // Save
        let index_file = File::create(&path).unwrap();
        println!("Saving data to {path}...");
        serde_json::to_writer(BufWriter::new(index_file), self)
            .map_err(|err| eprintln!("ERROR: Tidak dapat save file ke json : {err}"))
    }
}

pub fn begin_index(model: &Mutex<Model>, path: PathBuf) -> CountCheck {
    let mut counter = CountCheck::new();
    if path.is_file() {
        index_file(model, path, &mut counter);
    } else {
        index_folder(model, &path, &mut counter);
    }
    counter

}
fn index_folder(model: &Mutex<Model>, path: &PathBuf, counter: &mut CountCheck) {
    let dir = fs::read_dir(&path).unwrap();
    for entry in dir {
        let path = entry.unwrap().path();

        if path.is_dir() {
            index_folder(model, &path, counter);
        } else {
            index_file(model, path, counter);
        }
    }
}

fn index_file(model: &Mutex<Model>, path: PathBuf, counter: &mut CountCheck) {
    let current_last_modified = path.metadata().unwrap().modified().unwrap();
    let mut model = model.lock().unwrap();
    let document_state = model.check_document(&path, current_last_modified);

    match document_state {
        DocumentState::ADD => {
            if let Some(doc) = calculate_document_by_extension(&path, current_last_modified) {
                println!("Index: {:?} -> SELESAI", path);
                // Kata-kata yang ada didokumen, jika ada kata tersebut, maka tambahkan 1. Misal file ada 500 dan kata "turu" muncul 100 di 500 dokumen tersebut, maka turu akan bernilai 100.
                for t in doc.tf.keys() {
                    if let Some(freq) = model.df.get_mut(t) {
                        *freq += 1
                    } else {
                        model.df.insert(t.to_string(), 1);
                    }
                }
                model.docs.insert(path, doc);
                counter.add += 1;
            } else {
            }
        }
        DocumentState::STABLE => {
            counter.stable += 1;
            // println!(
            //     "File {path} belum diupdate. Tidak di-index ulang.",
            //     path = path.display()
            // );
        }
        DocumentState::UPDATE => {
            model.update_document(path, current_last_modified);
            counter.update += 1;
        },
    }
}

fn table_and_count_term_freq(content: Vec<char>) -> (usize, TermFreq) {
    let mut tf = TermFreq::new();

    let mut total_count_term_in_doc = 0;
    for term in Lexer::new(&content) {
        if let Some(freq) = tf.get_mut(&term) {
            *freq += 1;
        } else {
            tf.insert(term, 1);
        }
        total_count_term_in_doc += 1;
    }

    (total_count_term_in_doc, tf)
}
fn calculate_document_by_extension(path: &PathBuf, last_modified: SystemTime) -> Option<Doc> {
    match path.extension() {
        Some(ext) => {
            if ext == "xhtml" {
                let content = lexer::parse_entire_xml_file(&path)
                    .unwrap()
                    .to_lowercase()
                    .chars()
                    .collect::<Vec<_>>();
                let result = table_and_count_term_freq(content);
                let document = Doc::new(result.1, result.0, last_modified);
                return Some(document);
            } else {
                None
            }
        }
        None => None,
    }
}

pub fn calculate_tf(t: &str, document: &Doc) -> f32 {
    let count_term_in_doc = document.tf.get(t).cloned().unwrap_or(0) as f32;

    count_term_in_doc / document.count as f32
}

pub fn calculate_idf(t: &str, n: usize, df: &DocFreq) -> f32 {
    let n = n as f32;
    let total_count_for_term_in_document = df.get(t).cloned().unwrap_or(1) as f32;

    // println!("{} {}", n, total_count_for_term_in_document.max(1f32));
    (n / total_count_for_term_in_document.max(1f32)).log2()
}

pub fn search_query<'a>(model: &'a Model, query: &'a [char]) -> Vec<(&'a Path, f32)> {
    let mut rank_tf: Vec<(&Path, f32)> = Vec::new();
    // Mengecek semua isinya (corpust)
    for (path, count_all_term_tf_table) in model.docs.iter() {
        // Untuk setiap path dan tf_table (document) yang sudah dijadikan unique.
        let mut rank = 0f32;
        for term in Lexer::new(&query) {
            let idf = calculate_idf(&term, model.docs.len(), &model.df);
            let tfidf = calculate_tf(&term, &count_all_term_tf_table) * idf;
            rank += tfidf;
        }
        rank_tf.push((&path, rank));
    }

    rank_tf.sort_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap());
    rank_tf.reverse();

    rank_tf
}
