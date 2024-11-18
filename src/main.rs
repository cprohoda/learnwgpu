use learnwgpu::run;

use tokio;

use std::error::Error;

#[tokio::main(flavor="current_thread")]
async fn main() -> Result<(), Box<dyn Error>> {
    run().await;

    Ok(())
}
