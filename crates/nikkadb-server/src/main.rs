use nikkadb_server::utils::builder::NikkaBuilder;

fn main() {
    let server = NikkaBuilder::new().build();
    server.run()
}
