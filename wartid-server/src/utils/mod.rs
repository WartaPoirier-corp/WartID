pub mod jwt;

pub fn gen_alphanumeric(len: usize) -> String {
    use rand::distributions::Alphanumeric;
    use rand::Rng;

    let mut rng = rand::rngs::OsRng::default();

    std::iter::repeat(())
        .map(|()| rng.sample(Alphanumeric))
        .map(char::from)
        .take(len)
        .collect()
}
