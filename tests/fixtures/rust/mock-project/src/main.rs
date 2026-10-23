fn hello_world() -> &'static str {
    "Hello, World!"
}

fn main() {
    println!("{}", hello_world());
}
