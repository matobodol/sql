use rustyline::DefaultEditor;
use rustyline::error::ReadlineError;
use sql::Database;

fn main() {
    let db = Database::new();

    // 1. Inisialisasi Rustyline Editor
    let mut rl = match DefaultEditor::new() {
        Ok(editor) => editor,
        Err(err) => {
            eprintln!("Gagal menginisialisasi REPL editor: {err}");
            return;
        }
    };

    // 2. Load history query jika ada file history sebelumnya
    let history_file = ".sql_history";
    let _ = rl.load_history(history_file);

    println!("===========================================");
    println!("        Welcome to Rust SQL Engine REPL     ");
    println!("===========================================");
    println!("Ketik query SQL kamu atau 'exit' untuk keluar.\n");

    loop {
        // 3. Prompt readline dengan history support & line editing
        let readline = rl.readline("sql> ");

        match readline {
            Ok(line) => {
                let sql = line.trim();

                if sql.is_empty() {
                    continue;
                }

                // Simpan input query ke memory history rustyline
                let _ = rl.add_history_entry(sql);

                if sql.eq_ignore_ascii_case("exit") || sql.eq_ignore_ascii_case("quit") {
                    println!("Goodbye!");
                    break;
                }

                // Eksekusi query ke engine
                match db.execute(sql) {
                    Ok((schema, rows)) => {
                        let columns = schema.columns();

                        if columns.is_empty() {
                            println!("Query OK (0 rows returned)\n");
                            continue;
                        }

                        // Format Header Kolom
                        let headers: Vec<String> = columns.iter().map(|c| c.name.clone()).collect();
                        let header_str = headers.join(" | ");
                        let separator = "-".repeat(header_str.len().max(10));

                        println!("{header_str}");
                        println!("{separator}");

                        // Format Baris Data
                        for row in &rows {
                            let vals: Vec<String> =
                                row.values().iter().map(|v| format!("{v:?}")).collect();
                            println!("{}", vals.join(" | "));
                        }

                        println!("({} row(s) returned)\n", rows.len());
                    }
                    Err(err) => {
                        println!("Error: {err:?}\n");
                    }
                }
            }
            Err(ReadlineError::Interrupted) => {
                // Menangani Ctrl+C (batal ketik/batal query)
                println!("^C");
                continue;
            }
            Err(ReadlineError::Eof) => {
                // Menangani Ctrl+D (exit)
                println!("Goodbye!");
                break;
            }
            Err(err) => {
                println!("Error reading line: {err:?}");
                break;
            }
        }
    }

    // 4. Simpan history query ke file `.sql_history` saat keluar
    if let Err(err) = rl.save_history(history_file) {
        eprintln!("Gagal menyimpan history: {err}");
    }
}
