use search_engine::model::{Model, self};
use search_engine::server::{self};
use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::thread::{self, sleep};
use std::{env, path::PathBuf, process::ExitCode};

fn main() {
    match entry() {
        Ok(()) => ExitCode::SUCCESS,
        Err(()) => ExitCode::FAILURE,
    };
}

fn usage(command: &str) -> () {
    eprintln!("Penggunaan {command} [SUBCOMMAND] [OPTIONS]");
    eprintln!("Subcommands");
    eprintln!(
        "      index <folder>                Index <folder> nya dan menyimpan indexnya ke index.json"
    );
    eprintln!("      search <index-file>     Mengecek seberapa banyak dokumen yang diindex didalam file tersebut ");
    eprintln!(
        "      serve <folder>            Menjalankan local http server dengan tampilan website  "
    );
}

fn entry() -> Result<(), ()> {
    let mut args = env::args();
    let program = args.next().expect("path to program harus disediakan");

    let subcommand = args.next().ok_or_else(|| {
        usage(&program);
        eprintln!("ERROR: tidak ada subcommand yang diberi");
    })?;

    match subcommand.as_str() {
        "test" => {
            thread::spawn(|| {
                for n in 1..=100 {
                    println!("{n}");
                }
            });
            for a in 100..=200 {
                println!("{a}");
            }
        }
        // "index" => {
        //     let dir_path = args.next().ok_or_else(|| {
        //         usage(&program);
        //         eprintln!("ERROR: tidak ada directory yang diberi");
        //     })?;

        //     let save_path = format!(
        //         "{dir_path}{name_dir}.index.json",
        //         name_dir = dir_path.replace("/", "")
        //     );
        //     let mut new_model: Model = Model::new();

        //     new_model.begin_index(PathBuf::from(dir_path));
        //     new_model.save_model_to_json_file(&save_path)?;
        // }
        // "reindex" => {
        //     let dir_path = args.next().ok_or_else(|| {
        //         usage(&program);
        //         eprintln!("ERROR: tidak ada directory yang diberi");
        //     })?;

        //     let mut current_model: Model =
        //         serde_json::from_reader(BufReader::new(File::open("index.json").unwrap())).unwrap();
        //     current_model.begin_index(PathBuf::from(dir_path));

        //     current_model.save_model_to_json_file("index.json")?;
        // }
        // "search" => {
        //     let index_path = args.next().ok_or_else(|| {
        //         usage(&program);
        //         eprintln!("ERROR: tidak ada index yang diberi");
        //     })?;
        //     let index_file = File::open(&index_path);

        //     match index_file {
        //         Ok(file) => {
        //             println!("Getting data from {}...", index_path);
        //             let _: Model = serde_json::from_reader(&file).unwrap();
        //             println!(
        //                 "Mengecek total file dari {} yang ada : {}",
        //                 index_path, "test"
        //             );
        //         }
        //         Err(_) => {
        //             usage(&program);
        //             eprintln!("ERROR: File tidak ada");
        //         }
        //     }
        // }
        "serve" => {
            let mut is_loaded = false;
            let dir_path = args.next().ok_or_else(|| {
                usage(&program);
                eprintln!("ERROR: tidak ada directory yang diberi");
            })?;

            let index_path = format!(
                "{dir_path}{dir_name}.index.json",
                dir_name = dir_path.replace("/", "")
            );

            let file_exist = Path::new(&index_path).try_exists().unwrap();

            let model: Arc<Mutex<Model>>;

            if file_exist {
                is_loaded = true;
                let index_file = File::open(&index_path).map_err(|err| {
                    eprintln!("ERROR: Tidak ada index file : {err}");
                })?;

                model = Arc::new(Mutex::new(
                    serde_json::from_reader(BufReader::new(&index_file)).unwrap(),
                ));
            } else {
                model = Arc::new(Mutex::new(Model::new()));
            }

            {
                let model = Arc::clone(&model);
                thread::spawn(move || {
                    // let mut model = model_copy1.lock().unwrap();
                    let model = Arc::clone(&model);

                    let counter = model::begin_index(&model, PathBuf::from(&dir_path));

                    let model = model.lock().unwrap();

                    let _ = model.save_model_to_json_file(&index_path);
                    println!("---------------");
                    println!("Document ditambah: {}", counter.add);
                    println!("Document didiamkan: {}", counter.stable);
                    println!("Document diupdate: {}", counter.update);
                    println!("---------------");
                    // Mulai server
                });
            }
            server::begin_server(&Arc::clone(&model)).unwrap_or_else(|err| {
                eprintln!("Error saat memulai server: {:?}", err);
            });
        }
        _ => usage("Search engine"),
    };
    Ok(())
}
