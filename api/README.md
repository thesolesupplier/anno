# **Anno - API**

Anno started as an API deployed on AWS Lambda, before its release summary feature was moved to a GitHub Action for easier integration. The API now suppliments the action and can review pull requests and add test cases to Jira issues, all of which are triggered via GitHub and Jira webhooks.

## **Usage**

The PR reviews and Jira tests cases are triggered via different webhook endpoints. To create Github and Jira webhooks, follow these guides:
- Github: [Creating Webhooks](https://docs.github.com/en/developers/webhooks-and-events/creating-webhooks)
- Jira: [Webhooks](https://developer.atlassian.com/server/jira/platform/webhooks/)

Webhook secrets are **required** for all webhooks.

### PR Reviews
`POST` `/github/pull-request/review`

Expects a [pull_request](https://docs.github.com/en/webhooks/webhook-events-and-payloads#pull_request) webhook event.

The following environment variables are **required**:

- `CHAT_GPT_API_KEY`
- `CHAT_GPT_BASE_URL`
- `GITHUB_APP_ID`
- `GITHUB_APP_INSTALLATION_ID`
- `GITHUB_APP_PRIVATE_KEY_BASE64`
- `GITHUB_BASE_URL`
- `GITHUB_WEBHOOK_SECRET`

### Jira Test Cases
`POST` `/jira/issue/test-cases`

Expects an [issue_updated](https://developer.atlassian.com/cloud/jira/platform/webhooks/#issue-webhooks) webhook event.

The following environment variables are **required**:

- `JIRA_API_KEY` - _(base64 encoded `<username>:<api_token>`)_
- `JIRA_BASE_URL`
- `JIRA_BOT_USER_ID`
- `JIRA_PROJECT_KEY` - (If your ticket keys are `ABC-123`, the project key is `ABC`)
- `JIRA_WEBHOOK_SECRET`

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

## **LLM Model Configuration**

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
