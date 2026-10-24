//! Verifica a conformidade OKF v0.2 de um bundle: `cargo run -p leptos-press-okf --example
//! check_bundle -- <pasta>`. Sai com código 1 e lista as violações se houver.

fn main() {
    let Some(dir) = std::env::args().nth(1) else {
        eprintln!("uso: check_bundle <pasta do bundle>");
        std::process::exit(2);
    };
    match leptos_press_okf::check_bundle(std::path::Path::new(&dir)) {
        Ok(()) => println!(
            "{dir}: bundle OKF {} conforme",
            leptos_press_okf::OKF_VERSION
        ),
        Err(errors) => {
            for e in &errors {
                eprintln!("{dir}/{e}");
            }
            std::process::exit(1);
        }
    }
}
