# **Anno**

Anno is a GitHub deployment annotator that uses AI to summarise code changes released in a deployment.

## **Local Development**

For local development, the app is run as a standard [Axum](https://github.com/tokio-rs/axum) server. The [Cargo](https://doc.rust-lang.org/cargo/) command to do so has been aliased in the `Makefile`.

### **Getting Started**

1. Make sure you have [Rust](https://www.rust-lang.org/tools/install) along with [cargo-watch](https://github.com/watchexec/cargo-watch) installed.
2. Create a `.env` file from the `.env.example` file and fill in the missing values.
3. Start the server in watch mode:

    ```bash
    make dev
    ```

The server should now be running at `http://localhost:3000`.

## **Deployment**

The app is deployed to AWS as a Lambda using the `cargo-lambda` crate. The commands to do so locally have been aliased in the `Makefile`:

1. Build:

    ```bash
    make build
    ```
2. Deploy:

    ```bash
    make deploy
    ```

The app should now be deployed to AWS Lambda, and a function URL should be printed to the console.