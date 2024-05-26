# **Anno**

Anno is a Github deployment **anno**tator that uses AI to summarise code changes in a deployment.

## **Local Development**

For local development the app is run as a standard axum server.

### **Getting Started**

1. Make sure you have [Rust](https://www.rust-lang.org/tools/install) and then [cargo-watch](https://github.com/watchexec/cargo-watch)) installed.
2. Create a `.env` file from the `.env.example` file and add values and fill in the missing values.
3. Start the server in watch mode:

    ```bash
    cargo watch -x run
    ```

The server should now be running on `http://localhost:3000`.

## **Deployment**

The app is deployed to AWS using the `cargo-lambda` crate.