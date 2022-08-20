#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
}

#[proc_macro]
pub fn generate_tuple_defines(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    // get a usize from input
    let input = format!("{}", input);
    let input = input.parse::<usize>().unwrap();
    let mut output = String::new();
    for i in 0..input {
        output.push_str(&format!(
            "pub const TUPLE_{}: (usize, usize) = ({}, {});\n",
            i, i, i
        ));
    }
    output.parse::<proc_macro::TokenStream>().unwrap()
}
