# **Anno**
Anno is a **GitHub Action** that leverages LLMs to summarise code changes released in deployments:

<img src="docs/release_summary_example.png" alt="Release summary example" width="650">

It can also be deployed as an **AWS HTTP Lambda** that receives Jira and GitHub webhook events to additionally review PRs and add test cases to Jira issues.

## **Usage**

```yaml
uses: The-Sole-Supplier/anno
with:
  # The name of the app being deployed. This is used in the slack message and defaults to the repository name if not provided.
  app_name:
  # The API key for ChatGPT. It must have read and write permissions for chat completions.
  chat_gpt_api_key: # !required
  # The model to use for ChatGPT. Defaults to `latest` if not provided.
  chat_gpt_model:
  # A base64 encoded string containing the Jira username and API key in a `<username>:<api_token>` format. The API key must have read permissions for Jira issues.
  jira_api_key: # !required
  # The base URL for your Jira instance, e.g. https://my-company.atlassian.net
  jira_base_url: # !required
  # The webhook URL for the Slack channel where release summary will be sent.
  slack_webhook_url: # !required
```

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
