//! `image-worker`: runs one image job and exits. See `image_worker::child`.

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let code = image_worker::child::run(&args, std::io::stdin().lock(), std::io::stdout().lock());
    std::process::exit(code);
}
