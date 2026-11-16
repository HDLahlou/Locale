fn main() {
    println!("{} (skeleton)", service_name());
}

fn service_name() -> &'static str {
    "service"
}

#[cfg(test)]
mod tests {
    #[test]
    fn service_name_matches() {
        assert_eq!(super::service_name(), "service");
    }
}
