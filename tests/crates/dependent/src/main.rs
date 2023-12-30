use std::env;

fn main() {
    let message = env::args_os()
        .nth(1)
        .expect("missing argument")
        .into_string()
        .expect("invalid argument");
    dependency::print(&message);
}
