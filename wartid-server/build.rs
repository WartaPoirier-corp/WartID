use ructe::Ructe;

fn main() {
    build_info_build::build_script();

    Ructe::from_env()
        .unwrap()
        .compile_templates("templates")
        .unwrap();
}
