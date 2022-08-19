use rust_macros::generate_tuple_defines;

fn main() {
    generate_tuple_defines!(3);
    println!("{:?}", TUPLE_0);
}
