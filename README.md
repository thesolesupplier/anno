# **Anno**

Anno is a serverless Rust application that leverages GitHub webhook events and AI to review PRs and summarise code changes released in production deployments.

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

### **LLM Model Configuration**

There is no overriding environment variable for whether Anno should use Claude or ChatGPT, as one is used for PR reviews and the other for release summaries. However, their respective models can be configured via the `CHAT_GPT_MODEL` and `CLAUDE_MODEL` environment variables.

## **Local Deployment**

The app is deployed to AWS as a Lambda using [cargo-lambda](https://www.cargo-lambda.info/). The commands to do so locally have been aliased in the `Makefile`.

### **Steps**

1. Follow the [installation guide](https://www.cargo-lambda.info/guide/installation.html) for `cargo-lambda`.
2. Create a `.env.prod` file from the `.env.example` file and fill in the missing values.
3. Build and deploy the app in a single command:

    ```bash
    make release
    ```

Alternatively, you can build and deploy the app separately:

1. Build:

    ```bash
    make build
    ```
2. Deploy:

    ```bash
    make deploy
    ```

The app should now be deployed to AWS Lambda, and a function URL should be printed to the console.
