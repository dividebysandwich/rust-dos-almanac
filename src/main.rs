#[cfg(feature = "ssr")]
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    rust_dos_almanac::server::run().await
}

#[cfg(not(feature = "ssr"))]
fn main() {
    // The binary is the server; the browser half is built with `cargo leptos`.
}
