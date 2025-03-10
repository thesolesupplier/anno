# **Anno - API**

Anno started as an API deployed on AWS Lambda, before its release summary feature was moved to a GitHub Action for easier integration. The API now supplements the action and can summarise and review pull requests, which is triggered via a GitHub webhook.

## **Usage**

To create a GitHub webhook, follow their [Creating Webhooks](https://docs.github.com/en/developers/webhooks-and-events/creating-webhooks) guide.

**Note:** Using a webhook secret is currently expected by Anno's middleware and **highly recommended**.

### Endpoint
`POST` `/github/pull-request/review`

Expects a [`pull_request`](https://docs.github.com/en/webhooks/webhook-events-and-payloads#pull_request) webhook event.

The following environment variables are **required**:

- `CLAUDE_API_KEY`
- `CLAUDE_BASE_URL`
- `GITHUB_APP_ID`
- `GITHUB_APP_INSTALLATION_ID`
- `GITHUB_APP_PRIVATE_KEY_BASE64`
- `GITHUB_BASE_URL`
- `GITHUB_WEBHOOK_SECRET`

To have Anno include Jira issue links in PR summaries, the following optional environment variables should be provided:

- `JIRA_API_KEY` - _(base64 encoded `<username>:<api_token>`)_
- `JIRA_BASE_URL`

## **Local Development**

For local development, the app is run as a standard [Axum](https://github.com/tokio-rs/axum) server. The [Cargo](https://doc.rust-lang.org/cargo/) command to do so has been aliased in the `Makefile`.

### **Getting Started**

1. Make sure you have [Rust](https://www.rust-lang.org/tools/install) along with [cargo-watch](https://github.com/watchexec/cargo-watch) installed.
2. Create a `.env` file from the `.env.example` file and fill in the missing, non-action specific values.
3. Start the server in watch mode:

    ```bash
    make dev
    ```

The server should now be running on port `3000`.

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
