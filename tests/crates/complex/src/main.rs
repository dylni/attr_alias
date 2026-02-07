use std::env;

#[attr_alias::eval]
#[attr_alias(true)]
fn main() {
    let message = env::args_os()
        .nth(1)
        .expect("missing argument")
        .into_string()
        .expect("invalid argument");
    print!("{}", message);
}
