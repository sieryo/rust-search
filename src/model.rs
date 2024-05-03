use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::BufWriter;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use crate::lexer::{self, Lexer};

pub type DocFreq = HashMap<String, usize>;
pub type TermFreq = HashMap<String, usize>;
pub type Docs = HashMap<PathBuf, Document>;

enum DocumentState {
    UPDATE,
    STABLE,
    ADD,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Document {
    pub tf: TermFreq,
    count: usize,
    pub last_modified: SystemTime,
}

impl Document {
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
    pub fn begin_index(&mut self, path: PathBuf) {
        if path.is_file() {
            if let Some(tf) = self.index_file(&path) {
                self.docs.insert(path, tf);
            }
        } else {
            self.index_folder(&path);
        }
    }
    fn index_folder(&mut self, path: &PathBuf) {
        let dir = fs::read_dir(&path).unwrap();
        for entry in dir {
            let path = entry.unwrap().path();

            if path.is_dir() {
                self.index_folder(&path);
            } else {
                if let Some(tf) = self.index_file(&path) {
                    self.docs.insert(path, tf);
                }
            }
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

    fn update_document(&mut self, path: &PathBuf, current_last_modified: SystemTime) -> Option<Document> {
        let last_doc = self.docs.remove(path).unwrap();
        let new_doc = calculate_document_by_extension(path);

        for t in last_doc.tf.keys() {
            if let Some(freq) = self.df.get_mut(t) {
                *freq -= 1
            }
        }

        for t in new_doc.1.keys() {
            if let Some(freq) = self.df.get_mut(t) {
                *freq += 1
            } else {
                self.df.insert(t.to_string(), 1);
            }
        }

        let document =
        Document::new(new_doc.1, new_doc.0, current_last_modified);

    return Some(document);

    }

    fn index_file(&mut self, path: &PathBuf) -> Option<Document> {
        let current_last_modified = path.metadata().unwrap().modified().unwrap();
        let document_state = self.check_document(path, current_last_modified);

        match document_state {
            DocumentState::ADD => {
                let doc = calculate_document_by_extension(&path);
                println!("Index: {:?} -> SELESAI", path);

                // Kata-kata yang ada didokumen, jika ada kata tersebut, maka tambahkan 1. Misal file ada 500 dan kata "turu" muncul 100 di 500 dokumen tersebut, maka turu akan bernilai 100.
                for t in doc.1.keys() {
                    if let Some(freq) = self.df.get_mut(t) {
                        *freq += 1
                    } else {
                        self.df.insert(t.to_string(), 1);
                    }
                }
                let document = Document::new(doc.1, doc.0, current_last_modified);

                return Some(document)
            }
            DocumentState::STABLE => {
                println!("File {path} belum diupdate. Tidak di-index ulang.", path = path.display());
                return None
            }
            DocumentState::UPDATE => {
                
                return self.update_document(&path, current_last_modified)
            }
        }
    }
    pub fn save_model_to_json_file(&mut self, path: &str) -> Result<(), ()> {
        // Save
        let index_file = File::create(&path).unwrap();
        println!("Saving data to {path}...");
        serde_json::to_writer(BufWriter::new(index_file), self)
            .map_err(|err| eprintln!("ERROR: Tidak dapat save file ke json : {err}"))
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
fn calculate_document_by_extension(path: &PathBuf) -> (usize, HashMap<String, usize>) {
    match path.extension().unwrap().to_str().unwrap() {
        "xhtml" => {
            let content = lexer::parse_entire_xml_file(&path)
            .unwrap()
            .to_lowercase()
            .chars()
            .collect::<Vec<_>>();
        let result = table_and_count_term_freq(content);
    
        result
        }
        _ => {
            todo!("test")
        }
    }

    
}

pub fn calculate_tf(t: &str, document: &Document) -> f32 {
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
